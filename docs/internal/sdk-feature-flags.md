# SDK and CLI feature flags (0.6.0+)

Grapheme 0.6.0 introduces **opt-in capability modules** for embedders while the CLI ships everything by default.

## Quick reference

| Crate | Version (0.6.0 train) | Default features | Full stack |
| --- | --- | --- | --- |
| `grapheme-cli` | 0.6.0 | `full` | All capabilities + WASIX runtime |
| `grapheme-lsp` | 0.6.0 | `full` | Editor gets full signatures |
| `grapheme-sdk` | 0.6.0 | *(none)* | Enable `full` or pick modules |
| `grapheme-stdlib` | 0.6.0 | *(none)* | `data`, `pdf`, `image`, `plot`, `media` |
| `grapheme-signatures` | 0.6.0 | *(none)* | Capability op metadata |
| `grapheme-runtime` | 0.6.0 | *(none)* | Execution engine |
| `grapheme-compiler` | 0.6.0 | *(none)* | Compiler pipeline |
| `grapheme-artifact` | 0.2.0 | — | MIR envelope (independent semver) |

## SDK (embedders)

```toml
[dependencies]
grapheme-sdk = { version = "0.6", default-features = false, features = ["full"] }
```

Per-module opt-in:

```toml
grapheme-sdk = { version = "0.6", default-features = false, features = ["data", "pdf"] }
```

Available flags on `grapheme-sdk`:

- `full` — enables all capability modules, WASIX runtime, and matching compiler/signatures/runtime flags
- `data` — Polars-native dataframe ops (`read_csv`, `filter`, `group_by`, `aggregate`, `schema`, `to_json`)
- `pdf` — PDF generate/extract (Wasm path when bound; stdlib scaffold fallback)
- `image` — resize/convert/metadata (Wasm path)
- `plot` — line/bar/scatter charts as SVG (Wasm path)
- `media` — probe/transcode (native ffmpeg/ffprobe CLI bridge)
- `wasix-runtime` — Wasm module execution backend (included in `full`)

Without capability features, module discovery and compile-time verification will not include `data.*`, `pdf.*`, etc.

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
grapheme-stdlib = { version = "0.6", default-features = false, features = ["data"] }
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
- Wasm sidecar manifest: `docs/internal/runtime/wasm-module-manifest-v1.md`
- CLI module commands: `docs/internal/cli.md`
- SDK API overview: `docs/internal/sdk.md`
