use grapheme_aot_container::{execute, execute_to_json, parse_stdin_request, ExecuteRequest};
use grapheme_artifact::MirProgram;
use serde_json::json;

fn load_mir(name: &str) -> MirProgram {
    let path = format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let raw = std::fs::read_to_string(path).expect("read fixture");
    serde_json::from_str(&raw).expect("parse mir fixture")
}

#[test]
fn hello_world_walks_core_ops_locally() {
    let mir = load_mir("hello-world.mir.json");
    let request = ExecuteRequest {
        entrypoint: Some("HelloWorld".to_string()),
        mir,
        initial_current: Some(json!({})),
        args: None,
        allowed_imports: None,
    };

    let result = execute(&request).expect("execute");
    assert!(result.ok, "expected ok, error={:?}", result.error);
    assert_eq!(result.steps, 2);
    assert_eq!(
        result.current.get("message").and_then(|v| v.as_str()),
        Some("LETS GO?!!!!!")
    );
    assert!(result.host_calls.is_empty());
}

#[test]
fn http_op_surfaces_host_call_required() {
    let mir = load_mir("host-call-required.mir.json");
    let request = ExecuteRequest {
        entrypoint: Some("NeedsHttp".to_string()),
        mir,
        initial_current: None,
        args: None,
        allowed_imports: None,
    };

    let result = execute(&request).expect("execute returns WalkResult even on host boundary");
    assert!(!result.ok);
    let err = result.error.expect("host error");
    assert_eq!(err.code, "HOST_CALL_REQUIRED");
    assert_eq!(result.host_calls.len(), 1);
    assert_eq!(
        result.host_calls[0]
            .pointer("/error/import")
            .and_then(|v| v.as_str()),
        Some("grapheme.runtime.host.v1::call.capability")
    );
}

#[test]
fn parse_wasix_envelope_extracts_execute_request() {
    let mir = load_mir("hello-world.mir.json");
    let envelope = json!({
        "module": "workflow",
        "op": "_start",
        "args": {
            "entrypoint": "HelloWorld",
            "mir": mir,
            "initial_current": {}
        }
    });
    let raw = serde_json::to_string(&envelope).unwrap();
    let request = parse_stdin_request(&raw).expect("parse wasix envelope");
    let out = execute_to_json(&request);
    assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
}
