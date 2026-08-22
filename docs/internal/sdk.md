# Embedded SDK

Use the SDK crate to compile and execute workflows in-process without shelling out to the CLI.

This path is for Rust embedders building Grapheme into services, agents, or internal tooling.

## Recommended Reading Order

1. This page (`docs/sdk.md`)
2. `docs/architecture.md`
3. `docs/runtime-policy.md`
4. `docs/governance/rustdoc-readiness.md`

Crate:

- `crates/grapheme-sdk` (version **0.7.1**)

## Feature flags (0.7.1+)

The normal SDK default preserves the host + Stage B runtime. For iOS, Wasm,
and other embedded targets, use the explicit slim profile:

```toml
[dependencies]
grapheme-sdk = { version = "0.7.1", default-features = false, features = ["slim"] }
```

The slim profile contains the core/json compiler and runtime only. It excludes
the host capability stack, transforms, AOT container, and Wasix runtime.

For a full host build, enable capabilities explicitly:

```toml
[dependencies]
grapheme-sdk = { version = "0.7.1", default-features = false, features = ["full"] }
```

Per-module opt-in:

```toml
grapheme-sdk = { version = "0.7.1", default-features = false, features = ["data", "pdf"] }
```

Available flags: `slim`, `host`, `stage-b`, `data`, `pdf`, `image`, `plot`, `media`, `wasix-runtime`, `full`.

See `docs/internal/sdk-feature-flags.md` for CLI vs SDK defaults and envelope shape.

## Entrypoint parameters (0.7.0)

Bind named executable parameters before run:

```rust
use grapheme_sdk::GraphemeEngine;
use serde_json::json;

fn main() {
    let engine = GraphemeEngine::builder()
        .with_entrypoint_args(json!({ "label": "grapheme" }))
        .build();

    let source = r#"import core from "grapheme/core"

query ParamsCallBind($label: String = "world") {
  call Greet(label: $label)
}

iterator Greet($label: String) on Any {
  core.echo(message: "hello {$label}")
}
"#;

    let result = engine.execute_source(source).expect("execute");
    println!("{:?}", result.execution.outcome);
}
```

Canonical example: `examples/params-call-bind.gr`. Author extract: `docs/internal/language/params-and-tags-v1.md`.

## Stage B AOT helpers (0.7.0)

```rust
use grapheme_sdk::GraphemeEngine;

fn main() {
    let engine = GraphemeEngine::builder()
        .with_strict_stage_b_container_execution(true)
        // .with_prefer_stage_b_wasix(true)  // optional Wasix multi-round
        .build();

    let source = r#"import core from "grapheme/core"

query Hello {
  core.echo(message: "hello") { state { current } }
}
"#;

    let stage_a = engine.compile_source_to_aot(source).expect("stage_a");
    let stage_b = engine
        .compile_source_to_aot_stage_b_default(source)
        .expect("stage_b");

    let _ = engine.execute_aot(&stage_a).expect("run stage_a");
    let _ = engine.execute_aot(&stage_b).expect("run stage_b");
}
```

Build the container wasm when you need real Stage B emission / Wasix bytes: `./scripts/build-aot-container.sh`.

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

## Embedded host integration

An embedding application can add host-backed MirV1 modules without supplying a
Wasm sidecar. Configure the registry before building the engine:

```rust
use grapheme_runtime::ModuleRegistry;
use grapheme_sdk::GraphemeEngine;

fn build_engine(manifest: grapheme_runtime::ModuleManifest) -> GraphemeEngine {
    GraphemeEngine::builder()
        .configure_module_registry(|registry: &mut ModuleRegistry| {
            registry.register_host_module(manifest);
        })
        .build()
}
```

For request-scoped state, use `execute_source_with_initial_state`. The supplied
JSON value seeds `state.current` for that execution only; it does not alter a
later call made through the same engine:

```rust
let result = engine
    .execute_source_with_initial_state(source, Some(serde_json::json!({ "tenant": "ios" })))
    .expect("execute source");
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

## Module Discovery APIs

The SDK now exposes module discovery/search helpers that mirror CLI discovery behavior.

```rust
use grapheme_sdk::{
    discover_module_manifests,
    modules_search_payload,
    modules_ops_payload,
    modules_types_payload,
    modules_examples_payload,
    ModuleSearchDetail,
    ModuleSearchOptions,
};

fn main() {
    let manifests = discover_module_manifests();
    println!("modules: {}", manifests.len());

    let payload = modules_search_payload(
        "web",
        &ModuleSearchOptions {
            explain: true,
            detail: ModuleSearchDetail::Concise,
            top: Some(1),
            min_score: Some(100.0),
            include_experimental: false,
        },
    );

    println!("{}", serde_json::to_string_pretty(&payload).unwrap());

    let ops = modules_ops_payload("web");
    println!("{}", serde_json::to_string_pretty(&ops).unwrap());

    let types = modules_types_payload("core").expect("core module types");
    println!("{}", serde_json::to_string_pretty(&types).unwrap());

    let examples = modules_examples_payload("websearch").expect("websearch examples");
    println!("{}", serde_json::to_string_pretty(&examples).unwrap());
}
```

Use these APIs when building agent tooling that needs the same discovery semantics as the CLI without shelling out to subprocesses.

Module operation rows in `modules_ops`, `modules_info.exported_ops`, and `modules_types.types`
now include a `stability` tag (`stable`, `experimental`, or `deprecated`) so embedders can
enforce policy or ranking rules based on release maturity.

## Executable Reflection APIs

The SDK exposes executable reflection for both source and compiled artifacts.

```rust
use grapheme_sdk::{
    executables_reflection_contract_from_source,
    executables_reflection_payload_from_source,
};

fn main() {
    let source = r#"
query Hello {
  state { current }
}
"#;

    let typed = executables_reflection_contract_from_source(source)
        .expect("reflect source executables");
    println!("typed count: {}", typed.count);

    let json = executables_reflection_payload_from_source(source)
        .expect("reflect source payload");
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
```

Artifact reflection path:

```rust
use grapheme_sdk::{
    executables_reflection_contract_from_artifact,
    executables_reflection_payload_from_artifact,
};
use grapheme_compiler::{Compiler, CompilerOptions};

fn main() {
    let source = r#"
query Hello {
  state { current }
}
"#;

    let compiled = Compiler::compile_source(source, CompilerOptions::default())
        .expect("compile");

    let typed = executables_reflection_contract_from_artifact(&compiled.artifact);
    println!("artifact count: {}", typed.count);

    let json = executables_reflection_payload_from_artifact(&compiled.artifact);
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
```

Use source reflection when you need HIR-level signature metadata; use artifact reflection when you already operate on compiled envelopes.

## Hotload persistence (0.6.0+)

Hydrate module manager state from the project hotload store on engine build:

```rust
use grapheme_sdk::GraphemeEngine;

let engine = GraphemeEngine::builder()
    .with_default_hotload_store() // reads .grapheme/modules/hotload.json when present
    .build();
```

Discover, activate, and persist from a session:

```rust
use grapheme_sdk::GraphemeEngine;
use std::path::PathBuf;

let engine = GraphemeEngine::builder().build();
let mut session = engine.runtime_session();

let activation = session
    .activate_discovered_module("pdf", &[PathBuf::from("modules"), PathBuf::from("plugins")])
    .expect("activate pdf from scan roots");

println!("generation {}", activation.generation_id);

// rollback_module_generation() also persists hotload state
session.save_default_hotload_store().expect("persist hotload");
```

Hotload store path: `.grapheme/modules/hotload.json` (schema `grapheme.modules.hotload/v1`).

## Stateful Runtime Session + Hotmodule Lifecycle

Use `runtime_session()` when you need persistent activation state across multiple executions.

```rust
use grapheme_runtime::{CompatibilityMode, LoadModuleRequest, ModuleAbi};
use grapheme_sdk::GraphemeEngine;
use std::path::PathBuf;

fn main() {
    let engine = GraphemeEngine::builder().build();
    let mut session = engine.runtime_session();

    let activation = session
        .activate_module_generation(LoadModuleRequest {
            module_id: "http".to_string(),
            wasm_path: PathBuf::from("plugins/http-rs/target/wasm32-wasip1/release/http_rs.wasm"),
            compatibility_mode: CompatibilityMode::Strict,
            abi: ModuleAbi::MirV1,
            version: Some("0.7.0".to_string()),
        })
        .expect("activate module");

    println!("active generation: {}", activation.generation_id);

    let _result = session.execute_source(
        "import http from \"grapheme/http\"\nquery Q { state { current } }",
    );

    let events = session.module_lifecycle_events();
    println!("events observed: {}", events.len());

    let rollback = session
        .rollback_module_generation("http")
        .expect("rollback module");

    println!("rolled back generation: {}", rollback.generation_id);
}
```

This session API preserves deterministic activation and rollback semantics while keeping in-flight execution pinning behavior inside runtime internals.

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
