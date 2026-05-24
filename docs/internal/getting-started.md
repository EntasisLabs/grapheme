# Getting Started

Grapheme is an AI workflow language and governed runtime platform.

## 10-Minute Success Path

Goal: run, inspect, and execute a real `.gr` program with expected outputs.

Estimated time: 10 minutes.

## Choose Your Path

- Language author: start here, then read `docs/language-contract.md`, then `docs/cli.md`.
- Runtime operator: start here, then read `docs/runtime-policy.md`, `docs/modules.md`, and `docs/native-modules.md`.
- SDK embedder (Rust): skim this page, then read `docs/sdk.md` and `docs/architecture.md`.
- Editor user: use `docs/lsp/quickstart.md` and `extensions/grapheme-vscode/README.md`.

## Prerequisites

- Rust stable toolchain
- `rustup`
- `wasm32-wasip1` target for plugin builds
- Node.js + npm (for extension packaging and local extension development)

Install the Wasm target:

```bash
rustup target add wasm32-wasip1
```

## Step 1: Verify Workspace Build

```bash
cargo check --workspace
```

Expected outcome:

- Cargo completes successfully.
- You can proceed to parse and run examples.

## Step 2: Parse A Program

```bash
cargo run -- parse examples/hello-world.gr
```

Expected outcome:

- Output includes a `definitions` section.
- A query named `HelloWorld` is present.

## Step 3: Execute A Program

```bash
cargo run -- run examples/hello-world.gr --json
```

Expected outcome:

- Output includes `"outcome": "succeeded"`.
- Final state includes `"message": "LETS GO?!!!!!"`.

## Step 4: Discover Runtime Modules

```bash
cargo run -- modules
```

Expected outcome:

- A module list is printed (for example: `core`, `io`, `http`, `sql`, `websearch`).

## Common Developer Loop

From the repo root:

```bash
cargo check --workspace
cargo run -- parse examples/hello-world.gr
cargo run -- compile examples/hello-world.gr --emit mir
cargo run -- run examples/hello-world.gr
```

Use this loop after you complete the 10-minute path above.

## Running with Wasm Plugins

Use native module mode to build and bind known plugins automatically:

```bash
cargo run -- run examples/core-merge.gr --native-modules
```

You can also bind manually:

```bash
cargo run -- run examples/http-get.gr \
  --bind http=plugins/http-rs.wasm
```

## View Available Runtime Modules

```bash
cargo run -- modules
```

## Explore Example Programs

The `examples/` directory includes:

- core operations (`core-merge.gr`, `core-filter.gr`, `core-validate-schema.gr`)
- io/http/tcp/smtp/secrets demos
- basic memory roundtrip demo (`memory-roundtrip.gr`)

## Generate JSON Output for Automation

```bash
cargo run -- run examples/core-merge.gr --native-modules --json
```

## Troubleshooting

If any step fails, start with:

- `docs/troubleshooting.md`

High-frequency issues covered there include:

- missing toolchain/targets
- policy allow-list denials
- Stage B strict-mode constraints
- entrypoint ambiguity and parser errors

## Next Steps

- Language contract: `docs/language-contract.md`
- Embedded SDK guide: `docs/sdk.md`
- Architecture overview: `docs/architecture.md`
- CLI command reference: `docs/cli.md`
- Runtime policy guardrails: `docs/runtime-policy.md`
- Troubleshooting: `docs/troubleshooting.md`
- Scenario playbook pack: `docs/quality/scenario-playbooks-v1.md`
- Full docs index: `docs/README.md`
