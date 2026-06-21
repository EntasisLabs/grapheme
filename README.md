# Grapheme

Grapheme is a language and runtime for building governed automation workflows with explicit control flow, typed transitions, and capability-aware execution.

Write workflows in `.gr`, compile them into verified MIR artifacts, and run them with policy controls across host-backed and Wasix-backed module paths.

**0.6.0 (extensible platform):** opt-in capability modules for embedders (`data`, `pdf`, `image`, `plot`, `media`), dynamic Wasm discovery with hotload, and typed `{ data, meta, error }` result envelopes. The CLI ships full by default; the SDK opts in via Cargo features. See [CHANGELOG.md](CHANGELOG.md#060---2026-06-03).

## What You Can Build

- Operational runbooks that branch and recover safely (`if`, `match`, `@loop`, `@retry`, `@timeout`).
- Data and integration pipelines with explicit state transitions (`set`, `transition`, typed state machines).
- Policy-scoped automations that call HTTP, SQL, SMTP, secrets, TCP, memory, and custom modules.
- **Capability pipelines (0.6.0+):** Polars CSV ingest (`data`), Wasm PDF/image/plot plugins, ffmpeg probe/transcode (`media`).
- Developer workflows with built-in CLI + LSP + VS Code support.

## Start In 5 Minutes

### Prerequisites

- Rust stable + Cargo
- `wasm32-wasip1` target
- Node.js + npm (for extension packaging/local extension work)

```bash
rustup target add wasm32-wasip1
```

### First run

```bash
cargo check --workspace
cargo run -- parse examples/hello-world.gr
cargo run -- compile examples/hello-world.gr --emit artifact
cargo run -- run examples/hello-world.gr
```

Project default entrypoint:

```bash
cargo run -- run
cargo run --
```

Both resolve `project.main` from `grapheme.toml`.

### Install CLI

From local workspace:

```bash
cargo install --path crates/grapheme-cli --locked
```

From git:

```bash
cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme
```

Initialize bundled examples outside this repo:

```bash
grapheme examples list
grapheme examples init --out .
grapheme run examples/main.gr
```

## Try Real Workflows

Run with native modules:

```bash
cargo run -- run examples/core-merge.gr --native-modules
```

Run real-world scenarios:

```bash
cargo run -- run examples/realworld/automated-release-control-tower.gr --native-modules
cargo run -- run examples/realworld/blue-green-cutover-guarded.gr --native-modules
cargo run -- run examples/realworld/feature-flag-progressive-rollout.gr --native-modules
```

Machine-readable output:

```bash
cargo run -- run examples/realworld/feature-flag-progressive-rollout.gr --native-modules --json
```

### 0.6.0 capability stack (Wasm + native)

Build reference Wasm plugins and scan discovered modules:

```bash
bash plugins/build-plugins.sh
grapheme modules scan
grapheme modules activate pdf
grapheme run examples/platform-release-060.gr
```

Native data + media (CLI `full` build; `media` requires `ffmpeg`/`ffprobe` on `PATH`):

```bash
grapheme run examples/data-read-csv.gr
grapheme run examples/media-probe.gr
```

Hotload state persists under `.grapheme/modules/hotload.json` across `modules activate`, `modules rollback`, and `grapheme run`. See `docs/internal/cli.md` and `docs/internal/sdk-feature-flags.md`.

## Copy-Paste Examples

### 1) Fetch a page and convert it to markdown

```gr
glyph HttpGetMarkdown {
  set { url: "https://example.com" }
  |> http.get(url: $state.url)
  |> core.pick(fields: ["body"], input: $state)
  |> html.to_md(html: $state.body)
  |> core.echo(message: $state.text)
}
```

### 2) Run a guarded rollout loop with explicit state transitions

```gr
struct FlagState {
  status: String
  rollout: Float
  canary_score: Float
  min_score: Float
  timeline?: String
}

query Rollout on FlagState -> FlagState {
  FlagState {
    status: "planned",
    rollout: 0.0,
    canary_score: 98.0,
    min_score: 95.0
  }
  |> Run
}

iterator Run on FlagState -> FlagState @core_default @loop(max: 10, merge: "replace") {
  match $state.status {
    case done, rolled_back => return
    default => Step
  }
}

iterator Step on FlagState -> FlagState {
  if $state.canary_score < $state.min_score then
    transition $state.status -> rolled_back { timeline: "rollback" }
  else
    transition $state.status -> done { rollout: 100.0, timeline: "complete" }
}
```

## Choose Your Track

- Workflow author: `docs/quickstart.md` -> `docs/language-tour.md` -> `docs/playbooks.md`
- Runtime/operator: `docs/internal/runtime-policy.md` -> `docs/internal/modules.md` -> `docs/internal/native-modules.md`
- SDK embedder (Rust): `docs/internal/sdk.md` -> `docs/internal/sdk-feature-flags.md` -> `docs/internal/architecture.md`
- Editor/LSP user: `docs/internal/lsp/quickstart.md` -> `extensions/grapheme-vscode/README.md`

## Runtime Policy Controls

Allow-list env vars:

- `GRAPHEME_ALLOWED_HTTP_DOMAINS`
- `GRAPHEME_ALLOWED_TCP_TARGETS`
- `GRAPHEME_ALLOWED_SMTP_DOMAINS`
- `GRAPHEME_ALLOWED_SECRETS`

Runtime tuning:

- `GRAPHEME_WASIX_CACHE_MAX_MODULES` (default `8`)
- `GRAPHEME_RUNTIME_TIMING` (`1` or `true`)
- `GRAPHEME_RUNTIME_MAX_STEPS` (`none` or `unbounded` to disable)
- `GRAPHEME_RUNTIME_MAX_CALL_DEPTH` (`none` or `unbounded` to disable)

Example policy scope:

```bash
GRAPHEME_ALLOWED_HTTP_DOMAINS=example.com \
  cargo run -- run examples/http-get.gr --native-modules
```

## Repo Map

- `crates/grapheme-compiler`: parse + lower (AST/HIR/MIR) + verify.
- `crates/grapheme-artifact`: artifact envelope and MIR contracts.
- `crates/grapheme-runtime`: execution engine + policy enforcement + Wasix path.
- `crates/grapheme-cli`: `grapheme` commands (`parse`, `compile`, `build`, `run`, `modules`).
- `crates/grapheme-lsp`: language server for `.gr` authoring.
- `extensions/grapheme-vscode`: VS Code extension.
- `plugins/*-rs`: Wasm plugins (legacy host-adjacent + 0.6.0 capability plugins: `pdf-rs`, `image-rs`, `plot-rs`).
- `modules/`: Wasm sidecar manifests and built `.wasm` artifacts for discovery.
- `examples/`: runnable examples and real-world scenarios.
- `docs/`: primary docs.
- `docs/internal/`: architecture, runtime, release, and contributor docs.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes and version history.

## Documentation

- Main index: `docs/README.md`
- Quickstart: `docs/quickstart.md`
- Hero workflow: `docs/hero-workflow.md`
- Examples index: `examples/README.md`
- Troubleshooting: `docs/internal/troubleshooting.md`
- Language contract: `docs/internal/language-contract.md`

## Tooling and Release

- LSP quickstart: `docs/internal/lsp/quickstart.md`
- VS Code extension: `extensions/grapheme-vscode/README.md`
- LSP/VSIX release guide: `docs/internal/release/lsp-release.md`
- Validation bundle: `scripts/step4-checks.sh`
- Loop benchmark: `scripts/benchmark-loop.sh`

## License

Apache-2.0. See `LICENSE`.
