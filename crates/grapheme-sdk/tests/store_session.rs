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
