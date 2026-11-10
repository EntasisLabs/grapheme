//! RFC-0007 step 2: headless rendering and action dispatch over a store session.

use grapheme_sdk::GraphemeEngine;
use serde_json::json;

const COUNTER: &str = r#"
state Counter {
  count: Int = 0
  derive double: Int = $count * 2
}

mutation Increment on Counter {
  apply { count: $count + 1 }
}

mutation Add($by: Int) on Counter {
  apply { count: $count + $by }
}

view CounterView on Counter {
  Column(gap: 4) {
    value: Text("{$count} (x2 = {$double})")
    increment: Button(label: "Increment") { on click: Increment }
    add: Button { on click: Add(by: $count + 1), Text("Add more") }
    footer: Footer
  }
}

view Footer {
  Row { Text("footer") }
}
"#;

fn session() -> grapheme_sdk::GraphemeStoreSession {
    GraphemeEngine::builder()
        .build()
        .store_session(COUNTER)
        .expect("session")
}

fn action<'a>(
    snapshot: &'a grapheme_runtime::ViewSnapshot,
    key: &str,
) -> &'a grapheme_runtime::ViewAction {
    snapshot
        .actions
        .iter()
        .find(|a| a.key == key)
        .unwrap_or_else(|| panic!("no action on {key}: {:?}", snapshot.actions))
}

#[test]
fn render_evaluates_the_tree_against_the_latest_revision() {
    let mut session = session();
    let snapshot = session.render("CounterView").unwrap();
    assert_eq!(snapshot.store.as_deref(), Some("Counter"));
    assert_eq!(snapshot.revision, Some(0));
    assert_eq!(snapshot.ir["$ir"], "grapheme-ui/1");
    let root = &snapshot.ir["root"];
    assert_eq!(root["key"], "CounterView");
    assert_eq!(root["children"][0]["props"]["content"], "0 (x2 = 0)");
    assert_eq!(
        root["children"][1]["events"]["click"],
        json!({"$action": "Increment", "args": {}, "id": "CounterView/increment#click@1"})
    );
    assert_eq!(
        root["children"][2]["events"]["click"]["args"],
        json!({"by": 1})
    );

    let footer = &root["children"][3];
    assert_eq!(footer["view"], "Footer");
    assert_eq!(footer["$prim"], "Row");
    assert_eq!(footer["key"], "CounterView/footer");
    assert_eq!(footer["children"][0]["key"], "CounterView/footer/0");

    let increment = action(&snapshot, "CounterView/increment");
    assert_eq!(increment.event, "click");
    assert_eq!(increment.target, "Increment");
    assert_eq!(increment.role.as_deref(), Some("button"));
    assert_eq!(increment.label.as_deref(), Some("Increment"));
    assert!(increment.bound_args.is_empty());

    let add = action(&snapshot, "CounterView/add");
    assert_eq!(
        add.bound_args,
        json!({"by": 1}).as_object().unwrap().clone()
    );
    assert_eq!(add.label.as_deref(), Some("Add more"));
    assert_eq!(snapshot.actions.len(), 2);
}

#[test]
fn invoking_an_action_commits_one_revision_like_invoke() {
    let mut session = session();
    let snapshot = session.render("CounterView").unwrap();
    let id = action(&snapshot, "CounterView/increment").id.clone();
    let commit = session.invoke_action(&id, None).unwrap();
    assert_eq!(commit.revision, 1);
    let revision = commit.committed.expect("a revision");
    assert_eq!(revision.cause, "Increment");
    assert_eq!(revision.diff["count"], json!([0, 1]));

    let snapshot = session.render("CounterView").unwrap();
    assert_eq!(snapshot.revision, Some(1));
    assert_eq!(
        snapshot.ir["root"]["children"][0]["props"]["content"],
        "1 (x2 = 2)"
    );
    assert_eq!(action(&snapshot, "CounterView/increment").id, id);
}

#[test]
fn headless_actions_and_direct_invokes_agree() {
    let mut by_action = session();
    let mut by_invoke = session();
    for _ in 0..3 {
        let snapshot = by_action.render("CounterView").unwrap();
        let id = action(&snapshot, "CounterView/increment").id.clone();
        by_action.invoke_action(&id, None).unwrap();
        by_invoke.invoke("Increment", None).unwrap();
    }
    assert_eq!(
        by_action.snapshot("Counter").unwrap(),
        by_invoke.snapshot("Counter").unwrap()
    );
    assert_eq!(by_action.revisions(), by_invoke.revisions());
}

#[test]
fn rebinding_an_action_makes_its_old_id_stale() {
    let mut session = session();
    let first = session.render("CounterView").unwrap();
    let old_add = action(&first, "CounterView/add").id.clone();
    let increment = action(&first, "CounterView/increment").id.clone();

    session.invoke_action(&increment, None).unwrap();
    // `Add(by: $count + 1)` is now bound to a different argument.
    let err = session.invoke_action(&old_add, None).unwrap_err().to_string();
    assert!(err.contains("stale_action"), "{err}");
    assert_eq!(session.revision("Counter"), Some(1));

    let second = session.render("CounterView").unwrap();
    let new_add = action(&second, "CounterView/add");
    assert_ne!(new_add.id, old_add);
    assert_eq!(new_add.bound_args["by"], 2);
    let commit = session.invoke_action(&new_add.id.clone(), None).unwrap();
    assert_eq!(commit.committed.unwrap().diff["count"], json!([1, 3]));
}

#[test]
fn unknown_action_ids_and_unrendered_views_are_rejected() {
    let mut session = session();
    let err = session
        .invoke_action("CounterView/increment#click@1", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("stale_action"), "{err}");
    let err = session.render("Missing").unwrap_err().to_string();
    assert!(err.contains("no view named 'Missing'"), "{err}");
    assert_eq!(session.revisions().len(), 0);
}

#[test]
fn views_without_a_store_render_without_a_revision() {
    let mut session = session();
    let snapshot = session.render("Footer").unwrap();
    assert_eq!(snapshot.revision, None);
    assert!(snapshot.actions.is_empty());
    assert_eq!(
        snapshot.ir["root"]["children"][0]["props"]["content"],
        "footer"
    );
}

#[test]
fn feed_view_if_for_expands_after_load() {
    let source = include_str!("../../../examples/feed-view.gr");
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(source)
        .unwrap();
    let idle = session.mount(Some("FeedApp")).unwrap();
    assert!(
        idle.actions.iter().any(|a| a.target == "LoadFeed"),
        "{:?}",
        idle.actions
    );
    assert!(
        !idle.actions.iter().any(|a| a.target == "Like" && a.id.starts_with("FeedView/")),
        "like buttons should not render while idle: {:?}",
        idle.actions
    );

    session.invoke_action("expose:LoadFeed", None).unwrap();
    let ready = session.app_snapshot().unwrap();
    assert_eq!(ready.revision, 2);
    let likes: Vec<_> = ready
        .actions
        .iter()
        .filter(|a| a.target == "Like" && a.id.starts_with("FeedView/"))
        .map(|a| (a.id.clone(), a.bound_args.clone()))
        .collect();
    assert_eq!(likes.len(), 2, "{likes:?}");
    assert!(
        likes.iter().any(|(id, args)| id.contains("/1/") && args.get("id") == Some(&json!("1"))),
        "{likes:?}"
    );
    assert!(
        likes.iter().any(|(id, args)| id.contains("/2/") && args.get("id") == Some(&json!("2"))),
        "{likes:?}"
    );

    let like_id = likes[0].0.clone();
    session.invoke_action(&like_id, None).unwrap();
    assert_eq!(session.snapshot("Feed").unwrap()["likes_total"], json!(1));
}

#[test]
fn textfield_change_merges_payload_into_search() {
    let source = include_str!("../../../examples/feed-view.gr");
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(source)
        .unwrap();
    let snap = session.mount(Some("FeedApp")).unwrap();
    let search = snap
        .actions
        .iter()
        .find(|a| a.target == "Search" && a.event == "change")
        .expect("TextField change action");
    assert_eq!(search.payload_params, vec!["value".to_string()]);
    assert!(search.bound_args.is_empty());

    let missing = session
        .invoke_action(&search.id, None)
        .unwrap_err()
        .to_string();
    assert!(
        missing.contains("requires payload param 'value'"),
        "{missing}"
    );

    session
        .invoke_action(&search.id, Some(json!({"value": "Store"})))
        .unwrap();
    assert_eq!(session.snapshot("Feed").unwrap()["query"], json!("Store"));

    session
        .invoke_action("expose:Search", Some(json!({"value": "Hello"})))
        .unwrap();
    assert_eq!(session.snapshot("Feed").unwrap()["query"], json!("Hello"));

    let unknown = session
        .invoke_action(&search.id, Some(json!({"value": "x", "extra": 1})))
        .unwrap_err()
        .to_string();
    assert!(unknown.contains("unknown payload param 'extra'"), "{unknown}");

    let wrong_type = session
        .invoke_action(&search.id, Some(json!({"value": 1})))
        .unwrap_err()
        .to_string();
    assert!(
        wrong_type.contains("expects String"),
        "{wrong_type}"
    );
}

fn render(source: &str, view: &str) -> grapheme_sdk::GraphemeStoreSession {
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(source)
        .unwrap();
    session.render(view).unwrap();
    session
}

#[test]
fn duplicate_for_keys_fail_the_render() {
    let src = r#"
struct Post { id: String title: String }
state Feed { posts: [Post] = [{ id: "1", title: "A" }, { id: "1", title: "B" }] }
mutation Like($id: String) on Feed { apply { posts: $posts } }
view V on Feed {
  Column {
    for $post in $posts key $post.id {
      Button(label: $post.title) { on click: Like(id: $post.id) }
    }
  }
}
"#;
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(src)
        .unwrap();
    let err = session.render("V").unwrap_err().to_string();
    assert!(err.contains("duplicate for key"), "{err}");
}

#[test]
fn false_if_unmounts_its_action() {
    let src = r#"
state S { show: Bool = true }
mutation Hide on S { apply { show: false } }
view V on S {
  Column {
    if $show {
      Button(label: "Go") { on click: Hide }
    }
  }
}
"#;
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(src)
        .unwrap();
    let first = session.render("V").unwrap();
    assert_eq!(first.ir["$ir"], "grapheme-ui/1");
    assert_eq!(first.actions.len(), 1, "{:?}", first.actions);
    let id = first.actions[0].id.clone();
    assert!(id.contains("/$if/"), "{id}");
    session.invoke_action(&id, None).unwrap();
    let second = session.render("V").unwrap();
    assert!(second.actions.is_empty(), "{:?}", second.actions);
    let err = session.invoke_action(&id, None).unwrap_err().to_string();
    assert!(err.contains("stale_action"), "{err}");
}

#[test]
fn empty_list_renders_no_rows_and_non_list_source_fails() {
    let src = r#"
state S { items: [Int] = [], n: Int = 1 }
mutation Nop on S { apply { n: $n } }
view V on S {
  Column {
    for $item in $items key $item { Text("{$item}") }
  }
}
view Bad on S {
  Column {
    for $item in $n key $item { Text("x") }
  }
}
"#;
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(src)
        .unwrap();
    let snap = session.render("V").unwrap();
    let children = snap.ir["root"]["children"].as_array().map(|c| c.len()).unwrap_or(0);
    assert_eq!(children, 0, "{:?}", snap.ir);
    let err = session.render("Bad").unwrap_err().to_string();
    assert!(err.contains("for source must be a list"), "{err}");
}

#[test]
fn for_keys_stay_stable_when_the_list_is_reordered() {
    let src = r#"
struct Item { id: String }
state S { items: [Item] = [{ id: "a" }, { id: "b" }] }
mutation Swap on S { apply { items: [{ id: "b" }, { id: "a" }] } }
view V on S {
  Column {
    for $item in $items key $item.id {
      Button(label: $item.id) { on click: Swap }
    }
  }
}
"#;
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(src)
        .unwrap();
    let first = session.render("V").unwrap();
    let mut ids: Vec<_> = first.actions.iter().map(|a| a.id.clone()).collect();
    ids.sort();
    assert_eq!(ids.len(), 2, "{ids:?}");
    session.invoke("Swap", None).unwrap();
    let second = session.render("V").unwrap();
    let mut ids2: Vec<_> = second.actions.iter().map(|a| a.id.clone()).collect();
    ids2.sort();
    assert_eq!(ids, ids2, "reordering changed action ids");
}

#[test]
fn slash_in_a_for_key_is_escaped_and_empty_keys_fail() {
    let src = r#"
struct Item { id: String }
state S { items: [Item] = [{ id: "a/b#c" }] }
mutation Go on S { apply { items: $items } }
view V on S {
  Column {
    for $item in $items key $item.id {
      Button(label: "x") { on click: Go }
    }
  }
}
"#;
    let mut session = render(src, "V");
    let id = session.render("V").unwrap().actions[0].id.clone();
    assert!(id.contains("%2F") && id.contains("%23"), "{id}");
    assert!(!id.contains("a/b"), "{id}");

    let bad = r#"
state S { items: [String] = [""] }
mutation Nop on S { apply { items: $items } }
view V on S {
  Column { for $item in $items key $item { Text("x") } }
}
"#;
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(bad)
        .unwrap();
    let err = session.render("V").unwrap_err().to_string();
    assert!(err.contains("non-empty string or number"), "{err}");
}

#[test]
fn else_branch_and_for_index_render() {
    let src = r#"
state S { show: Bool = false, items: [String] = ["a", "b"] }
mutation Nop on S { apply { show: $show } }
view V on S {
  Column {
    if $show { Text("yes") } else { Text("no") }
    for $item, $index in $items key $index {
      Text("{$index}:{$item}")
    }
  }
}
"#;
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(src)
        .unwrap();
    let snap = session.render("V").unwrap();
    let texts: Vec<_> = snap.ir["root"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n.pointer("/props/content").and_then(|v| v.as_str()))
        .collect();
    assert_eq!(texts, ["no", "0:a", "1:b"], "{:?}", snap.ir);
}
