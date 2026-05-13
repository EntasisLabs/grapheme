# Grapheme

Grapheme is an AI workflow language and governed runtime platform.

It compiles `.gr` programs into verified MIR artifacts and executes capability calls through a governed runtime with host-backed and Wasix-backed module paths.

Status: production-hardening phase.

## Choose Your Path

- Language author: `docs/getting-started.md` -> `docs/language-contract.md` -> `docs/cli.md`
- Runtime operator: `docs/runtime-policy.md` -> `docs/modules.md` -> `docs/native-modules.md`
- SDK embedder (Rust): `docs/sdk.md` -> `docs/architecture.md` -> `docs/governance/rustdoc-readiness.md`
- Editor user (LSP/VS Code): `docs/lsp/quickstart.md` -> `extensions/grapheme-vscode/README.md`

## Why Grapheme

- Language-first workflow modeling with explicit control flow.
- Compiler/runtime boundary with artifact verification.
- Capability-governed execution and runtime policy controls.
- Strong ergonomics for compact automation code (`match`, `if`, `set`, `transition`, `@loop`, `@r`, `@t`).
- Tooling stack included: CLI, LSP, and VS Code extension.

## Quick Start

### Prerequisites

- Rust stable + Cargo
- `wasm32-wasip1` target
- Node.js + npm (extension packaging and local extension work)

```bash
rustup target add wasm32-wasip1
```

### Common developer loop

```bash
cargo check --workspace
cargo run -- parse examples/hello-world.gr
cargo run -- compile examples/hello-world.gr --emit artifact
cargo run -- run examples/hello-world.gr
```

Project-level default entrypoint:

```bash
cargo run -- run
cargo run --
```

Both commands resolve `project.main` from `grapheme.toml` (currently `examples/main.gr`).

### Install CLI with Cargo

Install from local workspace:

```bash
cargo install --path crates/grapheme-cli --locked
```

Install from git (private or public repository):

```bash
cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme
```

When installed outside this repository, scaffold bundled examples:

```bash
grapheme examples list
grapheme examples init --out .
grapheme run examples/main.gr
```

### Run with native modules

```bash
cargo run -- run examples/core-merge.gr --native-modules
```

### Explore the new showcase set

```bash
cargo run -- run examples/legacy/showcase/release-control-tower-compact.gr --native-modules
cargo run -- run examples/legacy/showcase/blue-green-cutover.gr --native-modules
cargo run -- run examples/legacy/showcase/feature-flag-progressive-rollout.gr --native-modules
```

Optional machine-readable output:

```bash
cargo run -- run examples/legacy/showcase/feature-flag-progressive-rollout.gr --native-modules --json
```

## Repository Layout

- `crates/grapheme-compiler`: parser + lowering (AST/HIR/MIR) + verifier integration.
- `crates/grapheme-artifact`: artifact envelope and MIR contracts.
- `crates/grapheme-runtime`: runtime engine, capability dispatch, policy enforcement, Wasix path.
- `crates/grapheme-cli`: `grapheme` CLI (`parse`, `compile`, `build`, `run`, `modules`).
- `crates/grapheme-lsp`: language server for `.gr` authoring.
- `plugins/*-rs`: plugin implementations compiled to Wasm.
- `extensions/grapheme-vscode`: VS Code extension wiring for LSP workflow.
- `examples/`: runnable examples and showcase demos.
- `docs/`: language, architecture, runtime policy, and release docs.

## Runtime Policy Controls

Allow-list environment variables:

- `GRAPHEME_ALLOWED_HTTP_DOMAINS`
- `GRAPHEME_ALLOWED_TCP_TARGETS`
- `GRAPHEME_ALLOWED_SMTP_DOMAINS`
- `GRAPHEME_ALLOWED_SECRETS`

Runtime tuning variables:

- `GRAPHEME_WASIX_CACHE_MAX_MODULES` (default `8`)
- `GRAPHEME_RUNTIME_TIMING` (`1` or `true` to emit timing summary)
- `GRAPHEME_RUNTIME_MAX_STEPS` (`none` or `unbounded` to disable)
- `GRAPHEME_RUNTIME_MAX_CALL_DEPTH` (`none` or `unbounded` to disable)

Example policy scoping:

```bash
GRAPHEME_ALLOWED_HTTP_DOMAINS=example.com \
  cargo run -- run examples/http-get.gr --native-modules
```

## Documentation

- Main index: `docs/README.md`
- Getting started: `docs/getting-started.md`
- Language contract: `docs/language-contract.md`
- Showcase examples: `examples/legacy/showcase/README.md`
- General examples index: `examples/README.md`

## Tooling and Release

- LSP quickstart: `docs/lsp/quickstart.md`
- VS Code extension: `extensions/grapheme-vscode/README.md`
- LSP/VSIX release flow: `docs/release/lsp-release.md`
- Loop benchmark: `scripts/benchmark-loop.sh`
- Step-4 validation bundle: `scripts/step4-checks.sh`

## License

Apache-2.0. See `LICENSE`.
