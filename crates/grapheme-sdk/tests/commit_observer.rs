//! A task `apply` is visible to the commit observer before the next capability
//! runs. Top-level mutations commit once and do not notify.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use grapheme_sdk::GraphemeEngine;
use serde_json::json;

const SOURCE: &str = r#"
state S {
  status: String = "idle"
}

mutation Touch on S {
  apply { status: "touched" }
}

task Load on S {
  apply { status: "pending" }
  core.value(value: 1)
  apply { status: "ready" }
}

view V on S {
  Text($status)
}

glyph App on S {
  expose Load, Touch
  V
}
"#;

#[test]
fn task_apply_is_visible_before_the_next_capability() {
    let phase = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let phase_host = Arc::clone(&phase);
    let phase_obs = Arc::clone(&phase);
    let seen_obs = Arc::clone(&seen);

    let mut session = GraphemeEngine::builder()
        .with_capability_interceptor(move |call| {
            if call.op != "value" {
                return None;
            }
            assert_eq!(
                phase_host.load(Ordering::SeqCst),
                1,
                "pending apply must be observed before core.value runs"
            );
            phase_host.store(2, Ordering::SeqCst);
            Some(Ok(json!(1)))
        })
        .build()
        .store_session(SOURCE)
        .unwrap();
    session.mount(Some("App")).unwrap();
    session.set_commit_observer(Some(Arc::new(move |session, revision| {
        let status = session.snapshot("S").unwrap()["status"].clone();
        let text = session.app_snapshot().unwrap().ir["root"]["props"]["content"].clone();
        assert_eq!(revision.store, "S");
        assert_eq!(text, status);
        if status == json!("pending") {
            phase_obs.store(1, Ordering::SeqCst);
        }
        seen_obs.lock().unwrap().push(status);
    })));

    let commit = session.invoke_action("expose:Load", None).unwrap();
    assert_eq!(commit.revision, 2);
    assert_eq!(phase.load(Ordering::SeqCst), 2);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[json!("pending"), json!("ready")]
    );
    assert_eq!(session.snapshot("S").unwrap()["status"], json!("ready"));
    assert_eq!(session.revisions().len(), 2);

    seen.lock().unwrap().clear();
    session.invoke_action("expose:Touch", None).unwrap();
    assert!(
        seen.lock().unwrap().is_empty(),
        "a top-level mutation commits once and does not notify"
    );
    assert_eq!(session.snapshot("S").unwrap()["status"], json!("touched"));
}
