# Getting Started

## Prerequisites

- Rust stable toolchain
- `rustup`
- `wasm32-wasip1` target for plugin builds
- Node.js + npm (for extension packaging and local extension development)

Install the Wasm target:

```bash
rustup target add wasm32-wasip1
```

## Common Developer Loop

From the repo root:

```bash
cargo check --workspace
cargo run -- parse examples/hello-world.gr
cargo run -- compile examples/hello-world.gr --emit mir
cargo run -- run examples/hello-world.gr
```

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

## Next Steps

- Embedded SDK guide: `docs/sdk.md`
- Architecture overview: `docs/architecture.md`
- CLI command reference: `docs/cli.md`
- Runtime policy guardrails: `docs/runtime-policy.md`
