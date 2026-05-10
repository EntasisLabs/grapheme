# Grapheme

Grapheme is a Rust-first compiler and runtime for AgentQL (`.aql`) programs.

It compiles AgentQL source into verified MIR artifacts and executes capability calls through a governed runtime that supports both host-backed and Wasix-backed module execution.

Status: active prototype moving toward production hardening.

## Why Grapheme

- Compiler/runtime split with clear trust boundaries.
- Artifact integrity checks before execution.
- Capability-aware execution and module registry.
- Wasix path for sandboxed module execution.
- LSP + VS Code extension for editor workflow.
- Release scripts and CI for binary + VSIX distribution.

## Repository Layout

- `crates/grapheme-compiler`: parse + lower to AST/HIR/MIR.
- `crates/grapheme-artifact`: artifact envelope + execution contracts.
- `crates/grapheme-runtime`: runtime engine, module registry, policy guard, Wasix backend.
- `crates/grapheme-cli`: `grapheme` CLI (parse, compile, run, modules, plugins build).
- `crates/grapheme-lsp`: language server for `.aql`.
- `plugins/*-rs`: Wasm module plugin implementations (built outside workspace members).
- `extensions/grapheme-vscode`: VS Code extension that runs a prebuilt LSP binary.
- `examples/`: runnable AgentQL examples.
- `scripts/`: release tooling for LSP and VSIX.

## Quick Start

### 1) Prerequisites

- Rust stable + Cargo
- `rustup` target for Wasm plugin builds:

```bash
rustup target add wasm32-wasip1
```

- Node.js + npm (for VS Code extension packaging)

### 2) Build and run a simple program

```bash
cargo run -- parse examples/hello-world.aql
cargo run -- compile examples/hello-world.aql --emit artifact
cargo run -- run examples/hello-world.aql
```

### 3) Run with native Wasm modules

```bash
cargo run -- run examples/core-merge.aql --native-modules
```

### 4) Discover runtime modules

```bash
cargo run -- modules
```

## Runtime Policy Controls

Runtime policy guards can be scoped via environment variables:

- `GRAPHEME_ALLOWED_HTTP_DOMAINS`
- `GRAPHEME_ALLOWED_TCP_TARGETS`
- `GRAPHEME_ALLOWED_SMTP_DOMAINS`
- `GRAPHEME_ALLOWED_SECRETS`

Example:

```bash
GRAPHEME_ALLOWED_HTTP_DOMAINS=example.com \
  cargo run -- run examples/http-get.aql --native-modules
```

## Documentation

Start here for full docs:

- `docs/README.md`

## LSP and VS Code

- LSP quickstart: `docs/lsp/quickstart.md`
- Extension guide: `extensions/grapheme-vscode/README.md`
- Release flow: `docs/release/lsp-release.md`

## Current Scope and Caveats

- Core end-to-end flow is operational: parse -> compile -> artifact -> runtime -> output.
- Wasix plugin execution works through stdin/stdout JSON bridging.
- Some modules are mock/demo semantics today and will be hardened over time.
- Memory module persistence semantics are not finalized for long-lived cross-run storage.

## License

Apache-2.0. See `LICENSE`.
