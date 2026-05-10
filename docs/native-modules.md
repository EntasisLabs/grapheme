# Native Modules

This guide covers how Grapheme native modules work and how to author new ones.

## What Is A Native Module

A native module is a Wasm binary executed through Grapheme's Wasix runtime path.

At runtime, module operations are resolved by `module/op` and invoked with a JSON request envelope:

- stdin request: `{ "op": "<operation>", "args": { ... } }`
- stdout response: any JSON value

## Current Native Module Catalog

- `core`
- `docs`
- `io`
- `http`
- `tcp`
- `smtp`
- `memory`
- `secrets`

Inspect live manifests:

```bash
cargo run -- modules
```

## Authoring A New Module (Rust)

1. Create a standalone crate under `plugins/<name>-rs`.
2. Add a binary entrypoint and parse stdin request JSON.
3. Dispatch by `op` and return JSON responses.
4. Build for Wasm target:

```bash
cargo build --manifest-path plugins/<name>-rs/Cargo.toml --release --target wasm32-wasip1
```

5. Copy/bind the generated `.wasm` file:

```bash
cargo run -- run examples/<some>.aql --bind <module>=plugins/<name>.wasm
```

Or wire it in CLI plugin specs and use:

```bash
cargo run -- run examples/<some>.aql --native-modules
```

## Operation Type Support

The compiler now validates argument types for known module operations.

Examples of enforced checks:

- missing required args (for example `http.get(url: ...)` requires `url`)
- wrong JSON types (for example `fields` for `core.pick` must be an array)
- unknown operation names for known modules

This helps both humans and models avoid generating invalid operation calls.

## Native Docs Module

The `docs` native module is a built-in knowledge source for module authoring and discovery.

Supported ops:

- `native_module_guide(topic?: string)`
- `native_module_registry()`
- `native_module_example(module?: string)`

Example query:

```aql
import Docs from "grapheme/docs"

query NativeModuleDocsDemo {
  Docs.native_module_guide(topic: "steps") {
    state { current }
  }
}
```

Run it:

```bash
cargo run -- run examples/docs-native-modules.aql --native-modules
```

## Next Evolution (Planned)

- signed registry metadata for modules
- versioned schemas for op args/outputs
- richer docs search for model-driven retrieval
- external module publishing workflow
