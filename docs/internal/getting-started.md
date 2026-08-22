# Getting Started

Grapheme is an AI workflow language and governed runtime platform.

## 10-Minute Success Path

Goal: run, inspect, and execute a real `.gr` program with expected outputs.

Estimated time: 10 minutes.

## Choose Your Path

- Language author: start here, then read `docs/internal/language-contract.md`, then `docs/internal/cli.md`.
- Runtime operator: start here, then read `docs/internal/runtime-policy.md`, `docs/internal/modules.md`, and `docs/internal/native-modules.md`.
- SDK embedder (Rust): skim this page, then read `docs/internal/sdk.md`, `docs/internal/sdk-feature-flags.md`, and `docs/internal/architecture.md`.
- Editor user: use `docs/internal/lsp/quickstart.md` and `extensions/grapheme-vscode/README.md`.

## Prerequisites

- Rust **1.92+** (edition 2024; required by Wasmer/WASIX deps); `rust-toolchain.toml` pins `stable`
- `rustup`
- `wasm32-wasip1` target for plugin builds
- Node.js + npm (for extension packaging and local extension development)

Install / sync the workspace toolchain:

```bash
rustup show
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

- A module list is printed (for example: `core`, `io`, `http`, `sql`, `websearch`, and with `full` build: `data`, `pdf`, `image`, `plot`, `media`).

## Step 5: Parameters, Tags, and Stage B (0.7.0)

Executable parameters and entrypoint args:

```bash
cargo run -- run examples/params-call-bind.gr --args-json '{"label":"grapheme"}' --json
```

Tagged variables with scoped `using`:

```bash
cargo run -- run examples/tag-using-scope.gr --json
```

Stage B AOT smoke (build the container asset once per checkout):

```bash
./scripts/build-aot-container.sh
cargo run -- build examples/hello-world.gr --aot-stage stage_b --json
cargo run -- run examples/hello-world.gr --aot-stage stage_b --json
```

Author notes: `docs/internal/language/params-and-tags-v1.md`. Cut checklist: `docs/internal/release/release-0.7.0.md`.

## Step 6 (optional): Capability Path (0.6.0+)

Build Wasm capability plugins and run the platform demo:

```bash
bash plugins/build-plugins.sh
cargo run -- modules scan
cargo run -- modules activate plot
cargo run -- modules activate pdf
cargo run -- run examples/platform-release-060.gr
```

Native-only capability examples (no Wasm activate):

```bash
cargo run -- run examples/data-read-csv.gr
cargo run -- run examples/media-probe.gr   # requires ffmpeg/ffprobe on PATH
```

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

Discover and activate Wasm capability modules (0.6.0+):

```bash
bash plugins/build-plugins.sh
cargo run -- modules scan
cargo run -- modules activate pdf
cargo run -- run examples/pdf-generate.gr
```

Hotload state is written to `.grapheme/modules/hotload.json`.

## View Available Runtime Modules

```bash
cargo run -- modules
```

## Explore Example Programs

The `examples/` directory includes:

- core operations (`core-merge.gr`, `core-filter.gr`, `core-validate-schema.gr`)
- **0.7.0 language:** `params-call-bind.gr`, `tag-using-scope.gr`
- io/http/tcp/smtp/secrets demos
- basic memory roundtrip demo (`memory-roundtrip.gr`)
- **0.6.0+ capabilities:** `data-read-csv.gr`, `pdf-generate.gr`, `image-metadata.gr`, `plot-line.gr`, `media-probe.gr`, `platform-release-060.gr`

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

- Language contract: `docs/internal/language-contract.md`
- Params / tags extract: `docs/internal/language/params-and-tags-v1.md`
- Embedded SDK guide: `docs/internal/sdk.md`
- SDK feature flags (0.7.0): `docs/internal/sdk-feature-flags.md`
- Architecture overview: `docs/internal/architecture.md`
- CLI command reference: `docs/internal/cli.md`
- Runtime policy guardrails: `docs/internal/runtime-policy.md`
- Troubleshooting: `docs/internal/troubleshooting.md`
- 0.7.0 cut checklist: `docs/internal/release/release-0.7.0.md`
- Scenario playbook pack: `docs/internal/quality/scenario-playbooks-v1.md`
- Full docs index: `docs/README.md`
