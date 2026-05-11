# Native Modules

This guide covers how Grapheme native modules work and how to author new ones.

## What Is A Native Module

A native module is a module implementation attached to the runtime. Modules can execute through:

- Wasix-backed Wasm binaries (stdin/stdout JSON contract)
- Host-backed adapters (MirV1 path in the CLI/runtime host)

At runtime, module operations are resolved by `module/op` and invoked with a JSON request envelope:

- stdin request: `{ "op": "<operation>", "args": { ... } }`
- stdout response: any JSON value

## Current Module Catalog

- `core`
- `docs`
- `io`
- `http`
- `tcp`
- `smtp`
- `memory`
- `secrets`

Current execution defaults:

- Wasix-backed by default: `core`, `docs`, `io`, `secrets`
- Host-backed by default: `http`, `tcp`, `smtp`
- Host-backed built-ins: `memory`

Transform modules (host-backed):

- `html.to_md(html?: string)`
- `json.parse(text?: string)`
- `csv.to_list(text?: string)`
- `yaml.to_json(text?: string)`

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
cargo run -- run examples/<some>.gr --bind <module>=plugins/<name>.wasm
```

Or wire it in CLI plugin specs and use:

```bash
cargo run -- run examples/<some>.gr --native-modules
```

Note: `--native-modules` currently auto-builds/auto-binds known Wasm plugins except host-preferred network modules (`http`, `tcp`, `smtp`) so those calls route through real host network adapters.

## Operation Type Support

The compiler now validates argument types for known module operations.

Examples of enforced checks:

- missing required args (for example `http.get(url: ...)` requires `url`)
- wrong JSON types (for example `fields` for `core.pick` must be an array)
- unknown operation names for known modules

This helps both humans and models avoid generating invalid operation calls.

## Transform Chaining Semantics

Transform ops support both explicit and implicit pipeline reuse.

- explicit reuse: pass prior values with `$current.<field>` in args
- implicit reuse: when the primary string arg is omitted, transform ops read from pipeline input in this order:
  - preferred arg key (for example `text` or `html`)
  - `text`
  - `body`
  - `content`
  - `html`
  - `markdown`
  - `data`

Return shapes:

- `html.to_md` -> `{ text: string, markdown: string }`
- `json.parse` -> `JsonValue`
- `csv.to_list` -> `Array<Object<string, string>>`
- `yaml.to_json` -> `JsonValue`

Examples:

```grapheme
query ImplicitChain {
  http.get(url: "https://example.com")
  |> html.to_md()
}
```

```grapheme
query ExplicitChain {
  yaml.to_json(text: "payload: |\n  [1,2,3]\n")
  |> json.parse(text: $current.payload)
}
```

See additional recipes in `examples/transform-cookbook/`.

## Native Docs Module

The `docs` native module is a built-in knowledge source for module authoring and discovery.

Supported ops:

- `native_module_guide(topic?: string)`
- `native_module_registry()`
- `native_module_example(module?: string)`

Example query:

```grapheme
import Docs from "grapheme/docs"

query NativeModuleDocsDemo {
  Docs.native_module_guide(topic: "steps") {
    state { current }
  }
}
```

Run it:

```bash
cargo run -- run examples/docs-native-modules.gr --native-modules
```

## Next Evolution (Planned)

- signed registry metadata for modules
- versioned schemas for op args/outputs
- richer docs search for model-driven retrieval
- external module publishing workflow
