# Modules and Plugins

## Built-in Module Catalog

The runtime ships with core manifests for:

- `core`
- `docs`
- `io`
- `http`
- `sql`
- `surreal`
- `tcp`
- `smtp`
- `email`
- `memory`
- `runtime`
- `secrets`
- `policy`

**0.6.0 capability modules** (feature-gated; CLI ships `full` by default):

| Module | Route | Notes |
| --- | --- | --- |
| `data` | native (`mir_v1`) | Polars CSV ingest + JSON frame pipeline |
| `media` | native (`mir_v1`) | ffmpeg/ffprobe CLI bridge |
| `pdf` | Wasm (`wasix_v1`) | `plugins/pdf-rs` + `modules/pdf.module.json` |
| `image` | Wasm | `plugins/image-rs` |
| `plot` | Wasm | `plugins/plot-rs` |

Inspect the live manifest set:

```bash
cargo run -- modules
cargo run -- modules types data
cargo run -- modules types pdf
```

## Wasm Discovery and Hotload (0.6.0+)

Capability Wasm modules ship with a sidecar manifest (`*.module.json`) next to the `.wasm` artifact.

```bash
bash plugins/build-plugins.sh
cargo run -- modules scan
cargo run -- modules activate pdf
cargo run -- modules status --yaml
```

Configure scan roots in `grapheme.toml`:

```toml
[modules]
scan = ["modules", "plugins"]
```

Persistence:

- `.grapheme/modules/hotload.json` — active/previous generations (hotload v1)
- `.grapheme/modules/bindings.json` — legacy path bindings (fallback)

See `docs/internal/runtime/wasm-module-manifest-v1.md` and `docs/internal/cli.md`.

## ABI Dispatch

Each module has a declared ABI (`mir_v1` or `wasix_v1`).

Resolution rule:

- if a wasm path is bound for a module, runtime dispatches that module via Wasix (`WasixV1`)
- otherwise runtime dispatches to the module's declared manifest ABI

Current defaults set `http`, `tcp`, and `smtp` to `mir_v1` so they use host-backed real network adapters unless explicitly bound to Wasm.

Capability modules (`pdf`, `image`, `plot`) prefer Wasm when activated or auto-bound from discovery.

## Plugin Source Crates

Plugin implementations are under `plugins/*-rs` and intentionally excluded from workspace members to keep top-level workspace operations focused.

**Legacy / host-adjacent plugins:**

- `plugins/core-echo-rs`
- `plugins/io-rs`
- `plugins/http-rs`
- `plugins/memory-rs`
- `plugins/tcp-rs`
- `plugins/smtp-rs`
- `plugins/secrets-rs`
- `plugins/docs-rs`

**0.6.0 capability plugins:**

- `plugins/pdf-rs`
- `plugins/image-rs`
- `plugins/plot-rs`

Catalog: `plugins/README.md`.

## Build Output Convention

`grapheme plugins build` copies legacy release wasm binaries to canonical paths under `plugins/`:

- `plugins/core-echo-rs.wasm`
- `plugins/io-rs.wasm`
- `plugins/http-rs.wasm`
- etc.

**0.6.0 capability build** uses `plugins/build-plugins.sh`, which copies artifacts to `modules/*.wasm` for discovery:

- `modules/pdf.wasm`
- `modules/image.wasm`
- `modules/plot.wasm`

These outputs are what `modules scan`, `modules activate`, `--bind`, and auto-bind on `grapheme run` use.

`--native-modules` currently auto-builds and auto-binds known **legacy** Wasm plugins except host-preferred network modules (`http`, `tcp`, `smtp`). Capability plugins use the discovery/hotload path instead.

## Capability and Policy Layers

Modules interact with two governance layers:

1. Capability policy: checks whether a capability token can execute.
2. Policy guard: checks argument-level restrictions for selected network and secrets operations.

`media.transcode` is policy-gated (`media.transcode.workspace` capability token in manifest).

For policy details and env configuration, see `docs/runtime-policy.md`.

## Result Envelope

Capability ops return `{ data, meta, error }` (schema `grapheme.host.result.envelope/v1`). See `docs/internal/sdk-feature-flags.md`.
