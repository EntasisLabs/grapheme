# SDK and CLI feature flags (0.7.0+)

Grapheme 0.6.0 introduced **opt-in capability modules** for embedders while the CLI ships everything by default. The 0.7.0 train keeps that matrix and ships Stage B / tagged-variable work on top.

As of RFC-0005, `grapheme-stdlib` also layers **host** vs **wasm** profiles so Stage B / `wasm32-wasip1` work does not pull TLS/DB stacks.

## Quick reference

| Crate | Version (0.7.0 train) | Default features | Full stack |
| --- | --- | --- | --- |
| `grapheme-cli` | 0.7.0 | `full` | All capabilities + WASIX runtime |
| `grapheme-lsp` | 0.7.0 | `full` | Editor gets full signatures |
| `grapheme-sdk` | 0.7.0 | *(none)*; stdlib edge enables `host` | Enable `full` or pick modules |
| `grapheme-stdlib` | 0.7.0 | `host` | `host` + `data`, `pdf`, `image`, `plot`, `media` |
| `grapheme-signatures` | 0.7.0 | *(none)* | Capability op metadata |
| `grapheme-runtime` | 0.7.0 | *(none)* | Execution engine |
| `grapheme-compiler` | 0.7.0 | *(none)* | Compiler pipeline |
| `grapheme-aot-container` | 0.7.0 | Wasm-safe walker | Stage B WASI binary |
| `grapheme-artifact` | 0.3.0 | — | MIR envelope (independent semver) |

## Stdlib host vs Wasm profiles

| Feature | What it enables | `wasm32-wasip1` |
| --- | --- | --- |
| *(always)* | `core`, `json`, `envelope`, `capability` | yes |
| `transforms` / `wasm` | `csv`, `yaml`, `html` | yes |
| `host` (default) | transforms + `http`, `web`, `net`, `email`, `sql`, `surreal` | no |
| `data` / `media` / … | existing capability modules | no (host/plugin paths) |

Wasm recipe (Stage B prerequisite):

```bash
cargo check -p grapheme-stdlib --no-default-features --features wasm --target wasm32-wasip1
```

See `docs/internal/rfc/rfc-0005-wasm-compilable-stdlib-v1.md`.

## SDK (embedders)

```toml
[dependencies]
grapheme-sdk = { version = "0.7", default-features = false, features = ["full"] }
```

Per-module opt-in:

```toml
grapheme-sdk = { version = "0.7", default-features = false, features = ["data", "pdf"] }
```

Available flags on `grapheme-sdk`:

- `full` — enables all capability modules, WASIX runtime, and matching compiler/signatures/runtime flags
- `data` — Polars-native dataframe ops (`read_csv`, `filter`, `group_by`, `aggregate`, `schema`, `to_json`)
- `pdf` — PDF generate/extract (Wasm path when bound; stdlib scaffold fallback)
- `image` — resize/convert/metadata (Wasm path)
- `plot` — line/bar/scatter charts as SVG (Wasm path)
- `media` — probe/transcode (native ffmpeg/ffprobe CLI bridge)
- `wasix-runtime` — Wasm module execution backend (included in `full`)

Without capability features, module discovery and compile-time verification will not include `data.*`, `pdf.*`, etc. Network/DB host modules remain available via the SDK→stdlib `host` edge.

### Hotload in embedders

```rust
GraphemeEngine::builder()
    .with_default_hotload_store() // .grapheme/modules/hotload.json
    .build();
```

Session helpers: `activate_discovered_module`, `save_default_hotload_store`, `rollback_module_generation` (auto-persists).

## CLI

The CLI binary is built with `default = ["full"]`. End users get all modules without configuring Cargo features.

```bash
grapheme modules info data   # requires CLI built with full (default)
grapheme modules scan        # discovers Wasm sidecars under plugins/ or [modules].scan
grapheme modules activate pdf
grapheme modules status
grapheme run examples/platform-release-060.gr
```

## Stdlib-only consumers

If you depend on `grapheme-stdlib` directly (tests, custom hosts):

```toml
# Host product path (default)
grapheme-stdlib = { version = "0.7" }

# Capability add-ons
grapheme-stdlib = { version = "0.7", features = ["data"] }

# Wasm / Stage B container profile
grapheme-stdlib = { version = "0.7", default-features = false, features = ["wasm"] }
```

Dispatch via `grapheme_stdlib::registry::dispatch` returns `None` for disabled modules.

## Result envelope

Capability modules normalize responses to:

```json
{
  "data": { "...": "payload" },
  "meta": { "schema": "grapheme.host.result.envelope/v1" },
  "error": null
}
```

Access payload fields in pipelines with `$current.data.<field>` after capability ops. LSP completion documents known fields per op.

Legacy flat JSON objects are still accepted during migration (`meta.legacy_flat` when coerced).

## Related docs

- Release plan: `docs/internal/roadmaps/release-0.6.0-extensible-platform.md`
- 0.7.0 cut checklist: `docs/internal/release/release-0.7.0.md`
- Wasm-compilable stdlib RFC: `docs/internal/rfc/rfc-0005-wasm-compilable-stdlib-v1.md`
- Wasm sidecar manifest: `docs/internal/runtime/wasm-module-manifest-v1.md`
- CLI module commands: `docs/internal/cli.md`
- SDK API overview: `docs/internal/sdk.md`
