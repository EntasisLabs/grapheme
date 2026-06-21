# SDK and CLI feature flags (0.6.0+)

Grapheme 0.6.0 introduces **opt-in capability modules** for embedders while the CLI ships everything by default.

## Quick reference

| Crate | Default features | Full stack |
| --- | --- | --- |
| `grapheme-cli` | `full` | All capabilities + WASIX runtime |
| `grapheme-sdk` | *(none)* | Enable `full` or pick modules |
| `grapheme-stdlib` | *(none)* | `data`, `pdf`, `image`, `plot`, `media` |
| `grapheme-lsp` | `full` | Editor gets full signatures |

## SDK (embedders)

```toml
[dependencies]
grapheme-sdk = { version = "0.4", default-features = false, features = ["full"] }
```

Per-module opt-in:

```toml
grapheme-sdk = { version = "0.4", default-features = false, features = ["data", "pdf"] }
```

Available flags on `grapheme-sdk`:

- `full` — enables all capability modules, WASIX runtime, and matching compiler/signatures/runtime flags
- `data` — Polars-native dataframe ops (scaffold → implementation during 0.6.0)
- `pdf` — PDF generate/extract (Wasm path)
- `image` — resize/convert/metadata (Wasm path)
- `plot` — line/bar/scatter charts (Wasm path)
- `media` — probe/transcode (native ffmpeg bridge)
- `wasix-runtime` — Wasm module execution backend (included in `full`)

Without capability features, module discovery and compile-time verification will not include `data.*`, `pdf.*`, etc.

## CLI

The CLI binary is built with `default = ["full"]`. End users get all modules without configuring Cargo features.

```bash
grapheme modules info data   # requires CLI built with full (default)
grapheme modules scan        # discovers Wasm sidecars under plugins/ or [modules].scan
```

## Stdlib-only consumers

If you depend on `grapheme-stdlib` directly (tests, custom hosts):

```toml
grapheme-stdlib = { version = "0.4", default-features = false, features = ["data"] }
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

Scaffold responses (pre-implementation) set `data.ok = false`, `data.status = "scaffold"`.

## Related docs

- Release plan: `docs/internal/roadmaps/release-0.6.0-extensible-platform.md`
- Wasm sidecar manifest: `docs/internal/runtime/wasm-module-manifest-v1.md`
- SDK API overview: `docs/internal/sdk.md`
