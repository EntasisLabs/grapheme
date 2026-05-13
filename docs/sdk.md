# Embedded SDK

Use the SDK crate to compile and execute workflows in-process without shelling out to the CLI.

Crate:

- `crates/grapheme-sdk`

## Basic Execution

```rust
use grapheme_sdk::GraphemeEngine;

fn main() {
    let source = r#"import core from \"grapheme/core\"

query Hello {
  core.echo(message: \"hello from sdk\") {
    state { current }
  }
}
"#;

    let engine = GraphemeEngine::builder().build();
    let result = engine.execute_source(source).expect("execute source");
    println!("{}", result.final_state);
}
```

## Structured Output

```rust
use grapheme_sdk::{GraphemeEngine, StructuredMode};

fn main() {
    let source = r#"import core from \"grapheme/core\"

query Hello {
  core.echo(message: \"hello\") {
    state { current }
  }
}
"#;

    let engine = GraphemeEngine::builder().build();
    let result = engine.execute_source(source).expect("execute source");

    let json = engine
        .format_result(&result, StructuredMode::Json)
        .expect("format json");
    let yaml = engine
        .format_result(&result, StructuredMode::Yaml)
        .expect("format yaml");

    println!("{}", json);
    println!("{}", yaml);
}
```

## Capability Observer Hook

Use this when you want observability over capability calls while preserving normal dispatch.

```rust
use grapheme_sdk::GraphemeEngine;

fn main() {
    let engine = GraphemeEngine::builder()
        .with_capability_observer(|call| {
            eprintln!(
                "capability call module={:?} op={} step={}",
                call.module, call.op, call.step_index
            );
        })
        .build();

    let _ = engine.execute_source("import core from \"grapheme/core\"\nquery Q { core.echo(message: \"x\") { state { current } } }");
}
```

## Capability Interceptor Hook

Use this when you want to override selected capability calls.

```rust
use grapheme_runtime::HostCallError;
use grapheme_sdk::GraphemeEngine;
use serde_json::json;

fn main() {
    let engine = GraphemeEngine::builder()
        .with_capability_interceptor(|call| {
            if call.op == "echo" {
                return Some(Ok(json!({"message": "intercepted"})));
            }
            None
        })
        .build();

    let _ = engine.execute_source("import core from \"grapheme/core\"\nquery Q { core.echo(message: \"x\") { state { current } } }");

    let _unused: Option<Result<serde_json::Value, HostCallError>> = None;
}
```

## Custom Host Factory

Use this when you need full control over capability dispatch.

```rust
use grapheme_runtime::{CapabilityCall, CapabilityHost, HostCallError};
use grapheme_sdk::GraphemeEngine;
use serde_json::Value as JsonValue;

struct MyHost;

impl CapabilityHost for MyHost {
    fn call(&mut self, call: &CapabilityCall) -> Result<JsonValue, HostCallError> {
        Err(HostCallError::Fatal(format!("unsupported op {}", call.op)))
    }
}

fn main() {
    let engine = GraphemeEngine::builder()
        .with_host_factory(|| Box::new(MyHost))
        .build();

    let _ = engine.execute_source("import core from \"grapheme/core\"\nquery Q { core.echo(message: \"x\") { state { current } } }");
}
```