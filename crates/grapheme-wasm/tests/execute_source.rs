use grapheme_wasm::{ExecuteRequest, execute, parse_stdin_request};
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
    // `cargo test --workspace` unifies `grapheme-stdlib` features with the host
    // CLI, which compiles `http` into this crate's stdlib. The wasm-profile
    // rejection only applies when that op is not registered. Isolated
    // `cargo test -p grapheme-wasm` still covers the rejection.
    if grapheme_stdlib::registry::is_registered_op("http", "get") {
        return;
    }

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

#[test]
fn value_expressions_evaluate_in_set_patches() {
    let source = r#"
enum Density { compact, comfortable }

struct Counter {
    count: Int
    density: Density
    ratio?: Float
    half?: Int
    label?: String
    big?: Bool
}

query Bump on Counter -> Counter {
    set {
        count: $state.count + 1,
        density: if $state.density == compact then comfortable else compact,
        ratio: $state.count / 4.0,
        half: $state.count / 4,
        label: "n=" + "{$state.count}",
        big: $state.count >= 10 && !($state.count > 100)
    }
}
"#;
    let response = execute(&ExecuteRequest {
        source: Some(source.to_string()),
        artifact: None,
        initial_current: Some(json!({ "count": 10, "density": "compact" })),
        args: None,
        entrypoint: None,
    });

    assert!(
        response.ok,
        "expected ok, error={:?}",
        response.error.as_ref().map(|e| &e.message)
    );
    let current = response
        .final_state
        .as_ref()
        .and_then(|state| state.get("current"))
        .cloned();
    assert_eq!(
        current,
        Some(json!({
            "count": 11,
            "density": "comfortable",
            "ratio": 2.5,
            "half": 2,
            "label": "n=10",
            "big": true
        }))
    );
}

#[test]
fn value_expression_errors_fail_the_step() {
    let source = r#"
struct Counter { count: Int }

query Divide on Counter -> Counter {
    set { count: 10 / $state.count }
}
"#;
    let response = execute(&ExecuteRequest {
        source: Some(source.to_string()),
        artifact: None,
        initial_current: Some(json!({ "count": 0 })),
        args: None,
        entrypoint: None,
    });

    let body = serde_json::to_string(&response).expect("serialize response");
    assert!(!response.ok || body.contains("EXPRESSION_ERROR"), "{body}");
    assert!(body.contains("division by zero in '/'"), "{body}");
}

#[test]
fn count_property_reads_list_and_string_length() {
    let source = r#"
struct Inbox {
    items: [String]
    title: String
    total?: Int
    long?: Bool
}

query Count on Inbox -> Inbox {
    set { total: $state.items.count, long: $state.title.count > 3 }
}
"#;
    let response = execute(&ExecuteRequest {
        source: Some(source.to_string()),
        artifact: None,
        initial_current: Some(json!({ "items": ["a", "b"], "title": "mail" })),
        args: None,
        entrypoint: None,
    });

    assert!(
        response.ok,
        "expected ok, error={:?}",
        response.error.as_ref().map(|e| &e.message)
    );
    let current = response
        .final_state
        .as_ref()
        .and_then(|state| state.get("current"))
        .cloned();
    assert_eq!(
        current,
        Some(json!({ "items": ["a", "b"], "title": "mail", "total": 2, "long": true }))
    );
}
