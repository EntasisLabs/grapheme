//! RFC-0007 Part H / I.2: UI glyph mount and headless app session.

use grapheme_sdk::GraphemeEngine;
use serde_json::json;

const APP: &str = r#"
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

query Read on Counter -> Int { $count }

view CounterView on Counter {
  Column {
    value: Text("{$count}")
    increment: Button(label: "Increment") { on click: Increment }
  }
}

glyph CounterApp on Counter {
  expose Increment, Read
  CounterView
}
"#;

fn session() -> grapheme_sdk::GraphemeStoreSession {
    GraphemeEngine::builder()
        .build()
        .store_session(APP)
        .expect("session")
}

#[test]
fn mount_returns_ir_tree_actions_and_expose_actions() {
    let mut session = session();
    let app = session.mount(None).unwrap();
    assert_eq!(app.glyph, "CounterApp");
    assert_eq!(app.store, "Counter");
    assert_eq!(app.revision, 0);
    assert_eq!(app.ir["$ir"], "grapheme-ui/1");
    assert_eq!(app.ir["root"]["key"], "CounterView");
    assert_eq!(app.ir["root"]["children"][0]["props"]["content"], "0");

    let tree = app
        .actions
        .iter()
        .find(|a| a.key == "CounterView/increment")
        .expect("tree action");
    assert_eq!(tree.target, "Increment");
    assert!(tree.id.ends_with("#click@1"));

    let exposed = app
        .actions
        .iter()
        .find(|a| a.id == "expose:Increment")
        .expect("expose action");
    assert_eq!(exposed.event, "invoke");
    assert_eq!(exposed.target, "Increment");
    // Exposed queries are readable via query(), not listed as invoke actions.
    assert!(app.actions.iter().all(|a| a.id != "expose:Read"));
}

#[test]
fn mounted_tree_action_and_expose_action_commit_the_same_way() {
    let mut via_tree = session();
    let mut via_expose = session();
    let tree_id = via_tree
        .mount(None)
        .unwrap()
        .actions
        .into_iter()
        .find(|a| a.key == "CounterView/increment")
        .unwrap()
        .id;
    via_tree.invoke_action(&tree_id).unwrap();
    via_expose.mount(None).unwrap();
    via_expose.invoke_action("expose:Increment").unwrap();
    assert_eq!(
        via_tree.snapshot("Counter").unwrap(),
        via_expose.snapshot("Counter").unwrap()
    );
    assert_eq!(via_tree.revisions(), via_expose.revisions());
    assert_eq!(via_tree.revisions()[0].diff["count"], json!([0, 1]));
}

#[test]
fn expose_action_with_required_params_is_rejected() {
    let mut session = session();
    session.mount(None).unwrap();
    // Add is not exposed; inventing an expose id for it must fail as stale.
    let err = session
        .invoke_action("expose:Add")
        .unwrap_err()
        .to_string();
    assert!(err.contains("stale_action"), "{err}");
}

#[test]
fn app_snapshot_refreshes_after_an_invoke() {
    let mut session = session();
    let first = session.mount(Some("CounterApp")).unwrap();
    let id = first
        .actions
        .iter()
        .find(|a| a.key == "CounterView/increment")
        .unwrap()
        .id
        .clone();
    session.invoke_action(&id).unwrap();
    let second = session.app_snapshot().unwrap();
    assert_eq!(second.revision, 1);
    assert_eq!(second.ir["root"]["children"][0]["props"]["content"], "1");
}
