//! Hot reload migration (RFC-0007 A.9).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use grapheme_sdk::GraphemeEngine;
use serde_json::json;

const BEFORE: &str = r#"
state Panel {
  count: Int = 0
  const step: Int = 2
  transient pulse: Int = 0
  label: String = "keep"
  derive shown: Int = $count + $pulse
}

mutation Edit on Panel {
  apply { count: $count + 1, pulse: $pulse + 4, label: "edited" }
}

task Work on Panel {
  apply { pulse: $pulse + 1 }
  apply { count: $count + 1 }
}
"#;

const AFTER: &str = r#"
state Panel {
  count: Int = 0
  const step: Int = 9
  transient pulse: Int = 0
  label: Int = 3
  extra: Int = 7
  derive shown: Int = $count * 10
}

mutation Edit on Panel {
  apply { count: $count + 1, pulse: $pulse + 4 }
}

task Work on Panel {
  apply { pulse: $pulse + 1 }
  apply { count: $count + 1 }
}
"#;

const LIFECYCLE: &str = r#"
enum Mode { idle, pulsing }

state_machine Pulse on Mode {
  transition idle -> pulsing
  transition pulsing -> idle
  transition pulsing -> pulsing
}

state Panel {
  count: Int = 0
  mode: Mode = idle
  transient phase: Mode = idle
}

mutation Go on Panel {
  transition $mode -> pulsing
  apply { count: 4 }
}

mutation Phase on Panel {
  transition $phase -> pulsing
}
"#;

#[test]
fn reload_keeps_plain_fields_resets_transient_and_is_one_revision() {
    let engine = GraphemeEngine::builder().build();
    let mut session = engine.store_session(BEFORE).unwrap();
    session.invoke("Edit", None).unwrap();
    assert_eq!(session.revision("Panel"), Some(1));

    let artifact = engine.compile_artifact(AFTER).unwrap();
    let outcome = session.reload(artifact).unwrap();
    assert_eq!(outcome.revisions.len(), 1, "{outcome:?}");
    let revision = &outcome.revisions[0];
    assert_eq!(revision.cause.kind, "reload");
    assert!(revision.cause.name.is_none());
    assert_eq!(revision.revision, 2, "revision counter continues");
    assert_eq!(session.revision("Panel"), Some(2));
    assert_eq!(session.revisions().len(), 2);

    assert!(
        !revision.history_diff.contains_key("pulse"),
        "transient pulse stays out of the history diff: {:?}",
        revision.history_diff
    );
    assert!(revision.diff.contains_key("pulse"));
    assert_eq!(revision.diff["pulse"], json!([4, 0]));
    assert_eq!(revision.diff["step"], json!([2, 9]));
    assert_eq!(revision.diff["extra"], json!([null, 7]));
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.field == "label" && d.message.contains("type changed")),
        "{:?}",
        outcome.diagnostics
    );

    let snap = session.snapshot("Panel").unwrap();
    assert_eq!(snap["count"], json!(1), "plain field kept");
    assert_eq!(snap["pulse"], json!(0), "transient reset");
    assert_eq!(snap["step"], json!(9), "const follows a new default");
    assert_eq!(snap["label"], json!(3), "type change resets the field");
    assert_eq!(snap["extra"], json!(7), "added field takes its default");
    assert_eq!(
        snap["shown"],
        json!(10),
        "derive recomputed from the new expression"
    );
    assert!(outcome.cancelled.is_empty());
}

#[test]
fn reload_while_a_task_is_active_fences_it_and_keeps_plain_fields() {
    let engine = GraphemeEngine::builder().build();
    let mut session = engine.store_session(BEFORE).unwrap();
    session.invoke("Edit", None).unwrap();
    let artifact = engine.compile_artifact(AFTER).unwrap();
    let once = Arc::new(AtomicBool::new(false));
    let artifact_for_hook = artifact.clone();
    session.set_commit_observer(Some(Arc::new(move |runtime, revision| {
        if revision.cause.kind != "task" || once.swap(true, Ordering::SeqCst) {
            return;
        }
        runtime.reload(artifact_for_hook.clone()).expect("reload");
    })));

    let commit = session.invoke("Work", None).unwrap();
    assert_eq!(commit.status.as_deref(), Some("cancelled"));
    let revisions = session.revisions();
    let reloads: Vec<_> = revisions
        .iter()
        .filter(|revision| revision.cause.kind == "reload")
        .collect();
    assert_eq!(reloads.len(), 1, "{revisions:?}");
    assert_eq!(reloads[0].revision, 3);

    let snap = session.snapshot("Panel").unwrap();
    assert_eq!(snap["count"], json!(1), "fenced apply did not write count");
    assert_eq!(snap["pulse"], json!(0), "transient reset by reload");
    assert_eq!(snap["shown"], json!(10));
    assert_eq!(snap["extra"], json!(7));
}

#[test]
fn reload_resets_transient_lifecycle_and_keeps_plain_lifecycle() {
    let engine = GraphemeEngine::builder().build();
    let mut session = engine.store_session(LIFECYCLE).unwrap();
    session.invoke("Go", None).unwrap();
    session.invoke("Phase", None).unwrap();
    let before = session.revision("Panel").unwrap();
    let artifact = engine.compile_artifact(LIFECYCLE).unwrap();
    let outcome = session.reload(artifact).unwrap();
    assert_eq!(outcome.revisions.len(), 1);
    assert_eq!(outcome.revisions[0].cause.kind, "reload");
    assert_eq!(outcome.revisions[0].revision, before + 1);

    let snap = session.snapshot("Panel").unwrap();
    assert_eq!(snap["count"], json!(4), "plain field kept");
    assert_eq!(snap["mode"], json!("pulsing"), "plain lifecycle kept");
    assert_eq!(snap["phase"], json!("idle"), "transient lifecycle reset");
}
