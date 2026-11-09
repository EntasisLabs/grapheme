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
    let commit = session.invoke_action(&id).unwrap();
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
        by_action.invoke_action(&id).unwrap();
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

    session.invoke_action(&increment).unwrap();
    // `Add(by: $count + 1)` is now bound to a different argument.
    let err = session.invoke_action(&old_add).unwrap_err().to_string();
    assert!(err.contains("stale_action"), "{err}");
    assert_eq!(session.revision("Counter"), Some(1));

    let second = session.render("CounterView").unwrap();
    let new_add = action(&second, "CounterView/add");
    assert_ne!(new_add.id, old_add);
    assert_eq!(new_add.bound_args["by"], 2);
    let commit = session.invoke_action(&new_add.id.clone()).unwrap();
    assert_eq!(commit.committed.unwrap().diff["count"], json!([1, 3]));
}

#[test]
fn unknown_action_ids_and_unrendered_views_are_rejected() {
    let mut session = session();
    let err = session
        .invoke_action("CounterView/increment#click@1")
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
