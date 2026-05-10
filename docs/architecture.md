# Architecture

## High-Level Model

Grapheme is structured around a strict compile/execute split:

1. Source (`.aql`) is parsed and lowered by the compiler.
2. The compiler emits MIR and an artifact envelope.
3. The runtime validates artifact compatibility and integrity.
4. Runtime executes MIR calls through module resolution and policy checks.

## Workspace Crates

- `grapheme-compiler`: parser + AST/HIR/MIR lowering + compilation API.
- `grapheme-artifact`: artifact schema and execution result contracts.
- `grapheme-runtime`: execution engine and governance layer.
- `grapheme`: CLI wrapper over compiler/runtime.
- `grapheme-lsp`: language server implementation for `.aql`.

## Runtime Execution Path

`RuntimeEngine::execute_artifact` performs:

1. format compatibility check (`grapheme.mir.v1` expected)
2. integrity hash verification (sha256 over MIR payload)
3. entrypoint resolution
4. per-call capability allow/deny check
5. module + operation resolution via `ModuleRegistry`
6. policy guard evaluation (`PolicyGuard`)
7. ABI dispatch:
   - `MirV1`: host call path
   - `WasixV1`: Wasix backend (`WasiRunner`) path

## Module Model

Runtime module metadata is expressed as manifests with:

- module id + version
- ABI (`mir_v1` or `wasix_v1`)
- exported operations
- required capabilities
- resource limits

By default, built-in core manifests are loaded. When a wasm path is bound for a module, call resolution upgrades dispatch to `WasixV1` for that module.

## WIT Direction (V1.5)

The V1.5 direction is to formalize runtime-module interop using WIT contracts:

- Runtime keeps semantic authority (control flow, state ownership, policy checks).
- Wasm modules are capability providers behind typed WIT boundaries.
- JSON remains acceptable at external I/O edges and as transitional payload encoding.

See: `docs/runtime/wit-contract-v1.5.md`

## Security and Governance

Current controls:

- artifact integrity verification
- compile/runtime capability policy checks
- runtime policy pre-checks by operation and arguments
- strict module/op lookup before dispatch

Planned hardening areas:

- stronger provenance/signing for module artifacts
- richer schema/type checking for op arguments
- persistent memory backend and data lifecycle policy

## Editor and Distribution

- `grapheme-lsp` provides diagnostics + formatting.
- VS Code extension runs a prebuilt LSP binary (never Cargo at runtime).
- Extension can auto-download LSP binaries from GitHub Releases.
- Release scripts produce platform binaries and VSIX artifacts.
