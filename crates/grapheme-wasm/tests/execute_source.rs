use grapheme_wasm::{execute, parse_stdin_request, ExecuteRequest};
use serde_json::json;

fn hello_world_source() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-world.gr"),
    )
    .expect("read examples/hello-world.gr")
}

#[test]
fn hello_world_source_executes_through_runtime_engine() {
    let response = execute(&ExecuteRequest {
        source: Some(hello_world_source()),
        artifact: None,
        initial_current: Some(json!({})),
        args: None,
        entrypoint: None,
    });

    assert!(
        response.ok,
        "expected ok, error={:?}",
        response.error.as_ref().map(|e| &e.message)
    );
    assert!(response.artifact_id.is_some());
    let current = response
        .final_state
        .as_ref()
        .and_then(|state| state.get("current"));
    assert_eq!(
        current
            .and_then(|v| v.get("message"))
            .and_then(|v| v.as_str()),
        Some("LETS GO?!!!!!")
    );
}

#[test]
fn host_only_op_fails_outside_wasm_stdlib_profile() {
    let source = r#"
import http from "grapheme/http"

query NeedsHttp {
  http.get(url: "https://example.com")
}
"#;
    let response = execute(&ExecuteRequest {
        source: Some(source.to_string()),
        artifact: None,
        initial_current: None,
        args: None,
        entrypoint: None,
    });

    assert!(!response.ok);
    let message = response
        .error
        .as_ref()
        .map(|e| e.message.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("outside the Wasm stdlib profile")
            || message.contains("unsupported capability")
            || message.contains("http"),
        "unexpected error: {message}"
    );
}

#[test]
fn parse_wasix_envelope_extracts_source_request() {
    let envelope = json!({
        "module": "runtime",
        "op": "_start",
        "args": {
            "source": hello_world_source(),
            "initial_current": {}
        }
    });
    let request = parse_stdin_request(&envelope.to_string()).expect("parse wasix envelope");
    let response = execute(&request);
    assert!(response.ok, "expected ok, error={:?}", response.error);
}

#[test]
fn hello_world_artifact_executes_through_runtime_engine() {
    let compiled = grapheme_compiler::Compiler::compile_source(
        &hello_world_source(),
        grapheme_compiler::CompilerOptions::default(),
    )
    .expect("compile");
    let response = execute(&ExecuteRequest {
        source: None,
        artifact: Some(compiled.artifact),
        initial_current: Some(json!({})),
        args: None,
        entrypoint: None,
    });
    assert!(response.ok, "expected ok, error={:?}", response.error);
}

#[test]
fn rejects_source_and_artifact_together() {
    let compiled = grapheme_compiler::Compiler::compile_source(
        &hello_world_source(),
        grapheme_compiler::CompilerOptions::default(),
    )
    .expect("compile");
    let response = execute(&ExecuteRequest {
        source: Some(hello_world_source()),
        artifact: Some(compiled.artifact),
        initial_current: None,
        args: None,
        entrypoint: None,
    });
    assert!(!response.ok);
    assert_eq!(
        response.error.as_ref().map(|e| e.code.as_str()),
        Some("INVALID_REQUEST")
    );
}
