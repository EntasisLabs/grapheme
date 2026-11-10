use grapheme_sdk::GraphemeEngine;
use serde_json::json;

const COUNTER: &str = r#"
enum Mode { idle, busy, done }

state_machine Work on Mode {
  transition idle -> busy
  transition busy -> done
  terminal done
}

state Counter {
  count: Int = 0
  const step: Int = 2
  transient pulse: Int = 0
  mode: Mode = idle
  label: String = "c"
  derive double: Int = $count * 2
  derive quad: Int = $double * 2
}

mutation Increment on Counter {
  apply { count: $count + $step, pulse: $pulse + 1 }
}

mutation Pulse on Counter {
  apply { pulse: $pulse + 1 }
}

mutation Add($by: Int) on Counter {
  apply { count: $count + $by, label: "added {$count}" }
}

mutation Twice on Counter {
  apply { count: $count + 1 }
  |> apply { count: $count + 1 }
}

mutation Noop on Counter {
  apply { count: $count }
}

mutation Start on Counter {
  transition $mode -> busy
}

mutation Finish on Counter {
  transition $mode -> done
}

query Total on Counter {
  set { total: $count + $double, mode: $mode }
}

query Doubled on Counter -> Int { $double }

query IsBusy on Counter -> Bool { $mode == busy }

query Describe($prefix: String) on Counter -> String { $prefix + $label }

query PerStep on Counter -> Int { 10 / $count }

mutation Bump on Counter {
  if $count > 3 then Start else Increment
}
"#;

fn session() -> grapheme_sdk::GraphemeStoreSession {
    GraphemeEngine::builder()
        .build()
        .store_session(COUNTER)
        .expect("store session")
}

#[test]
fn revision_zero_is_built_from_defaults_with_derives() {
    let session = session();
    assert_eq!(session.store_names(), vec!["Counter".to_string()]);
    assert_eq!(
        session.snapshot("Counter").unwrap(),
        json!({"count": 0, "step": 2, "pulse": 0, "mode": "idle", "label": "c", "double": 0, "quad": 0})
    );
    assert!(session.revisions().is_empty());
}

#[test]
fn each_invoke_commits_one_revision_with_runtime_and_history_diffs() {
    let mut session = session();
    let commit = session.invoke("Increment", None).unwrap();
    let revision = commit.committed.expect("committed");
    assert_eq!(revision.revision, 1);
    assert_eq!(revision.cause, "Increment");
    assert_eq!(
        serde_json::Value::Object(revision.diff),
        json!({"count": [0, 2], "pulse": [0, 1]})
    );
    assert_eq!(
        serde_json::Value::Object(revision.history_diff),
        json!({"count": [0, 2]})
    );

    let pulse = session.invoke("Pulse", None).unwrap().committed.unwrap();
    assert_eq!(pulse.revision, 2);
    assert_eq!(
        serde_json::Value::Object(pulse.diff),
        json!({"pulse": [1, 2]})
    );
    assert!(pulse.history_diff.is_empty());

    let snapshot = session.snapshot("Counter").unwrap();
    assert_eq!(snapshot["double"], json!(4));
    assert_eq!(snapshot["quad"], json!(8));
    assert_eq!(session.revisions().len(), 2);
}

#[test]
fn applies_in_one_mutation_are_one_transaction_reading_the_latest_writes() {
    let mut session = session();
    let revision = session.invoke("Twice", None).unwrap().committed.unwrap();
    assert_eq!(revision.revision, 1);
    assert_eq!(
        serde_json::Value::Object(revision.diff),
        json!({"count": [0, 2]})
    );
}

#[test]
fn args_and_interpolation_read_store_fields() {
    let mut session = session();
    session.invoke("Add", Some(json!({"by": 5}))).unwrap();
    let snapshot = session.snapshot("Counter").unwrap();
    assert_eq!(snapshot["count"], json!(5));
    assert_eq!(snapshot["label"], json!("added 0"));
}

#[test]
fn unchanged_run_commits_no_revision() {
    let mut session = session();
    let commit = session.invoke("Noop", None).unwrap();
    assert_eq!(commit.revision, 0);
    assert!(commit.committed.is_none());
}

#[test]
fn lifecycle_fields_follow_state_machine_edges() {
    let mut session = session();
    let err = session.invoke("Finish", None).unwrap_err().to_string();
    assert!(err.contains("invalid_transition"), "{err}");
    assert_eq!(session.revision("Counter"), Some(0));

    session.invoke("Start", None).unwrap();
    session.invoke("Finish", None).unwrap();
    assert_eq!(session.snapshot("Counter").unwrap()["mode"], json!("done"));
    let err = session.invoke("Start", None).unwrap_err().to_string();
    assert!(err.contains("terminal"), "{err}");
}

#[test]
fn branches_call_mutations_on_the_same_store() {
    let mut session = session();
    session.invoke("Bump", None).unwrap();
    session.invoke("Bump", None).unwrap();
    assert_eq!(session.snapshot("Counter").unwrap()["mode"], json!("idle"));
    session.invoke("Bump", None).unwrap();
    assert_eq!(session.snapshot("Counter").unwrap()["mode"], json!("busy"));
    assert_eq!(session.revisions().len(), 3);
}

#[test]
fn invoking_a_non_store_executable_fails() {
    let mut session = session();
    let err = session.invoke("Missing", None).unwrap_err().to_string();
    assert!(err.contains("not declared on a state store"), "{err}");
    let err = session.invoke("Total", None).unwrap_err().to_string();
    assert!(err.contains("queries are read-only"), "{err}");
}

const PULSE: &str = r#"
enum PanelMode { idle, pulsing }

state_machine PulseLifecycle on PanelMode {
  transition idle -> pulsing
  transition pulsing -> pulsing
  transition pulsing -> idle
}

enum Light { off, on }

state_machine Switch on Light {
  transition off -> on
  transition on -> off
}

state Panel {
  mode: PanelMode = idle
  light: Light = off
  hits: Int = 0
}

mutation Pulse on Panel {
  transition $mode -> pulsing
}

mutation Flicker on Panel {
  transition $light -> on
  |> transition $light -> off
}

mutation Relight on Panel {
  transition $light -> off
}

mutation Divide($by: Int) on Panel {
  apply { hits: 10 / $by }
}
"#;

#[test]
fn declared_self_edges_allow_reentry_and_undeclared_ones_fail() {
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(PULSE)
        .unwrap();
    session.invoke("Pulse", None).unwrap();
    let again = session.invoke("Pulse", None).unwrap().committed.unwrap();
    assert_eq!(again.revision, 2);
    assert_eq!(
        serde_json::Value::Object(again.diff),
        json!({"mode": ["pulsing", "pulsing"]})
    );

    let err = session.invoke("Relight", None).unwrap_err().to_string();
    assert!(err.contains("no edge off -> off"), "{err}");
}

#[test]
fn every_transition_in_a_mutation_is_validated_but_commits_once() {
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(PULSE)
        .unwrap();
    let revision = session.invoke("Flicker", None).unwrap().committed.unwrap();
    assert_eq!(revision.revision, 1);
    assert_eq!(
        serde_json::Value::Object(revision.diff),
        json!({"light": ["off", "off"]})
    );
    assert_eq!(session.snapshot("Panel").unwrap()["light"], json!("off"));
}

#[test]
fn a_failed_step_commits_nothing() {
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(PULSE)
        .unwrap();
    session.invoke("Divide", Some(json!({"by": 2}))).unwrap();
    assert!(session.invoke("Divide", Some(json!({"by": 0}))).is_err());
    assert_eq!(session.revision("Panel"), Some(1));
    assert_eq!(session.snapshot("Panel").unwrap()["hits"], json!(5));
}

#[test]
fn queries_read_the_latest_revision_and_commit_nothing() {
    let mut session = session();
    session.invoke("Increment", None).unwrap();
    let total = session.query("Total", None).unwrap();
    assert_eq!(total["total"], json!(6));
    assert_eq!(total["mode"], json!("idle"));
    assert_eq!(session.revision("Counter"), Some(1));
    assert_eq!(session.snapshot("Counter").unwrap().get("total"), None);
}

#[test]
fn expression_query_bodies_return_their_value() {
    let mut session = session();
    assert_eq!(session.query("Doubled", None).unwrap(), json!(0));
    assert_eq!(session.query("IsBusy", None).unwrap(), json!(false));
    session.invoke("Increment", None).unwrap();
    session.invoke("Start", None).unwrap();
    assert_eq!(session.query("Doubled", None).unwrap(), json!(4));
    assert_eq!(session.query("IsBusy", None).unwrap(), json!(true));
    assert_eq!(
        session
            .query("Describe", Some(json!({ "prefix": "label: " })))
            .unwrap(),
        json!("label: c")
    );
    assert_eq!(session.revision("Counter"), Some(2));
}

#[test]
fn a_failing_expression_query_reports_the_error_and_commits_nothing() {
    let mut session = session();
    let err = session.query("PerStep", None).unwrap_err();
    assert!(err.to_string().contains("division by zero"), "{err}");
    session.invoke("Increment", None).unwrap();
    assert_eq!(session.query("PerStep", None).unwrap(), json!(5));
    assert_eq!(session.revision("Counter"), Some(1));
}


const FEED: &str = r#"
struct Post {
  id: String
  title: String
  likes: Int
}
enum FeedStatus { idle, pending, ready, failed }

state Feed {
  status: FeedStatus = idle
  posts: [Post] = []
  failure: String = ""
  likes_total: Int = 0
}

mutation Like($id: String) on Feed {
  apply { likes_total: $likes_total + 1 }
}

task LoadFeed on Feed {
  apply { status: "pending", failure: "" }
  core.value(value: [
    { id: "1", title: "Hello", likes: 0 },
    { id: "2", title: "Tasks", likes: 2 }
  ])
  apply { status: "ready", posts: $current }
}

task FailFeed($message: String) on Feed {
  apply { status: "pending", failure: "" }
  core.value(value: $message)
  apply { status: "failed", failure: $current, posts: [] }
}

glyph FeedApp on Feed {
  expose LoadFeed
  Text("{$status}")
}
"#;

#[test]
fn task_applies_commit_separately_and_capability_result_writes_into_store() {
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(FEED)
        .unwrap();
    let commit = session.invoke("LoadFeed", None).unwrap();
    assert_eq!(commit.revision, 2);
    let last = commit.committed.expect("last apply committed");
    assert_eq!(last.revision, 2);
    assert_eq!(last.cause, "LoadFeed");
    assert!(last.diff.contains_key("posts"), "{:?}", last.diff);
    assert!(last.diff.contains_key("status"), "{:?}", last.diff);
    assert_eq!(session.revisions().len(), 2);
    assert_eq!(
        session.revisions()[0].diff.get("status").unwrap()[1],
        json!("pending")
    );
    let snap = session.snapshot("Feed").unwrap();
    assert_eq!(snap["status"], json!("ready"));
    assert_eq!(snap["posts"].as_array().unwrap().len(), 2);
    assert_eq!(snap["posts"][0]["title"], json!("Hello"));
    assert!(
        !commit.effects.is_empty(),
        "core.value should yield an effect receipt"
    );
}

#[test]
fn task_failure_keeps_earlier_applies() {
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(FEED)
        .unwrap();
    session
        .invoke("FailFeed", Some(json!({"message": "boom"})))
        .unwrap();
    let snap = session.snapshot("Feed").unwrap();
    assert_eq!(snap["status"], json!("failed"));
    assert_eq!(snap["failure"], json!("boom"));
    assert_eq!(snap["posts"], json!([]));
}

#[test]
fn exposed_task_invokes_through_mount() {
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(FEED)
        .unwrap();
    let snap = session.mount(Some("FeedApp")).unwrap();
    assert!(
        snap.actions.iter().any(|a| a.id == "expose:LoadFeed"),
        "{:?}",
        snap.actions
    );
    session.invoke_action("expose:LoadFeed").unwrap();
    assert_eq!(session.snapshot("Feed").unwrap()["status"], json!("ready"));
}

#[test]
fn view_can_bind_click_to_a_task() {
    let source = format!(
        "{FEED}\nview FeedView on Feed {{\n  Button(label: \"Load\") {{ on click: LoadFeed }}\n}}\n"
    );
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(&source)
        .unwrap();
    let rendered = session.render("FeedView").unwrap();
    let id = rendered.actions[0].id.clone();
    session.invoke_action(&id).unwrap();
    assert_eq!(session.snapshot("Feed").unwrap()["status"], json!("ready"));
}

#[test]
fn task_apply_then_capability_records_effect_receipt() {
    let src = r#"
state Feed { hits: Int = 0 }
task Ping on Feed {
  apply { hits: $hits + 1 }
  core.echo(message: "ping")
}
"#;
    let mut session = GraphemeEngine::builder()
        .build()
        .store_session(src)
        .unwrap();
    let commit = session.invoke("Ping", None).unwrap();
    assert_eq!(commit.revision, 1);
    assert_eq!(commit.effects.len(), 1);
    assert_eq!(commit.effects[0].op, "core.echo");
    assert_eq!(commit.effects[0].status, "acknowledged");
    assert_eq!(session.snapshot("Feed").unwrap()["hits"], json!(1));
}
