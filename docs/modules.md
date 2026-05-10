# Modules and Plugins

## Built-in Module Catalog

The runtime ships with core manifests for:

- `core`
- `docs`
- `io`
- `http`
- `tcp`
- `smtp`
- `memory`
- `runtime`
- `secrets`
- `policy`

Inspect the live manifest set:

```bash
cargo run -- modules
```

## ABI Dispatch

Each module has a declared ABI (`mir_v1` or `wasix_v1`).

Resolution rule:

- if a wasm path is bound for a module, runtime dispatches that module via Wasix (`WasixV1`)
- otherwise runtime dispatches to the module's declared manifest ABI

Current defaults set `http`, `tcp`, and `smtp` to `mir_v1` so they use host-backed real network adapters unless explicitly bound to Wasm.

## Plugin Source Crates

Plugin implementations are under `plugins/*-rs` and intentionally excluded from workspace members to keep top-level workspace operations focused.

Known plugin crates:

- `plugins/core-echo-rs`
- `plugins/io-rs`
- `plugins/http-rs`
- `plugins/memory-rs`
- `plugins/tcp-rs`
- `plugins/smtp-rs`
- `plugins/secrets-rs`

## Build Output Convention

`grapheme plugins build` copies release wasm binaries to canonical paths:

- `plugins/core-echo-rs.wasm`
- `plugins/io-rs.wasm`
- `plugins/http-rs.wasm`
- `plugins/memory-rs.wasm`
- `plugins/tcp-rs.wasm`
- `plugins/smtp-rs.wasm`
- `plugins/secrets-rs.wasm`

These outputs are what `--bind` and `--native-modules` use.

`--native-modules` currently auto-builds and auto-binds known Wasm plugins except host-preferred network modules (`http`, `tcp`, `smtp`).

## Capability and Policy Layers

Modules interact with two governance layers:

1. Capability policy: checks whether a capability token can execute.
2. Policy guard: checks argument-level restrictions for selected network and secrets operations.

For policy details and env configuration, see `docs/runtime-policy.md`.
