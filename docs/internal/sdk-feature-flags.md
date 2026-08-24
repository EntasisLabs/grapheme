# SDK and CLI feature flags (0.7.1+)

Grapheme 0.6.0 introduced **opt-in capability modules** for embedders while the CLI ships everything by default. The 0.7.0 train keeps that matrix and ships Stage B / tagged-variable work on top.

The 0.7.1 train adds an explicit **slim embedded profile** for iOS, Wasm, and
other products that only need the core compiler/runtime.

As of RFC-0005, `grapheme-stdlib` also layers **host** vs **wasm** profiles so Stage B / `wasm32-wasip1` work does not pull TLS/DB stacks.

## Quick reference

| Crate | Version (0.7.1 train) | Default features | Full stack |
| --- | --- | --- | --- |
| `grapheme-cli` | 0.7.1 | `full` | All capabilities + WASIX runtime |
| `grapheme-lsp` | 0.7.1 | `full` | Editor gets full signatures |
| `grapheme-sdk` | 0.7.1 | `host` + `stage-b` | `slim`, `full`, or selected modules |
| `grapheme-stdlib` | 0.7.1 | `host` | `host` + `data`, `pdf`, `image`, `plot`, `media` |
| `grapheme-signatures` | 0.7.1 | *(none)* | Capability op metadata |
| `grapheme-runtime` | 0.7.1 | `stage-b` | Execution engine + AOT support |
| `grapheme-compiler` | 0.7.1 | `stage-b` | Compiler pipeline + AOT support |
| `grapheme-aot-container` | 0.7.1 | Wasm-safe walker | Stage B WASI binary |
| `grapheme-wasm` | 0.7.1 | compiler + runtime (no Wasix) | Runtime-in-Wasm WASI binary |
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

## Slim embedded profile (iOS / Wasm)

Use this profile when the host application supplies its own integrations and
only needs core/json compilation and execution:

```toml
[dependencies]
grapheme-sdk = { version = "0.7.1", default-features = false, features = ["slim"] }
```

The slim graph excludes the host capability stack, transforms, AOT container,
and Wasix runtime. It is suitable for `aarch64-apple-ios`, iOS simulator
targets, and `wasm32-unknown-unknown`.

## Runtime-in-Wasm (RFC-0006)

To **run Grapheme inside Wasm** (WASI / edge), use `grapheme-wasm` rather than
enabling `wasix-runtime` on the native runtime crate. Wasix is a host that
*runs* Wasm plugins; it cannot and should not compile to Wasm.

```bash
# Crates already compile:
cargo check -p grapheme-runtime --target wasm32-wasip1
cargo check -p grapheme-compiler --no-default-features --target wasm32-unknown-unknown
cargo check -p grapheme-sdk --no-default-features --features slim --target wasm32-wasip1

# Product entrypoint (compiler + RuntimeEngine + wasm stdlib):
cargo test -p grapheme-wasm
bash scripts/build-runtime-wasm.sh
```

Do **not** pass `--features wasix-runtime` on a Wasm target (`aws-lc-sys` / Wasmer).

See `docs/internal/rfc/rfc-0006-runtime-in-wasm-v1.md` and `crates/grapheme-wasm/README.md`.

## SDK (embedders)

```toml
[dependencies]
grapheme-sdk = { version = "0.7.1", default-features = false, features = ["full"] }
```

Per-module opt-in:

```toml
grapheme-sdk = { version = "0.7.1", default-features = false, features = ["data", "pdf"] }
```

Available flags on `grapheme-sdk`:

- `full` — enables all capability modules, WASIX runtime, and matching compiler/signatures/runtime flags
- `slim` — core/json compiler and runtime only; no host stack, AOT container, or Wasix
- `host` — enables the stdlib host capability profile
- `stage-b` — enables Stage B AOT helpers and the AOT container dependency
- `data` — Polars-native dataframe ops (`read_csv`, `filter`, `group_by`, `aggregate`, `schema`, `to_json`)
- `pdf` — PDF generate/extract (Wasm path when bound; stdlib scaffold fallback)
- `image` — resize/convert/metadata (Wasm path)
- `plot` — line/bar/scatter charts as SVG (Wasm path)
- `media` — probe/transcode (native ffmpeg/ffprobe CLI bridge)
- `wasix-runtime` — Wasm module execution backend (included in `full`)

Without capability features, module discovery and compile-time verification will not include `data.*`, `pdf.*`, etc. The normal SDK default includes the stdlib `host` edge; the explicit `slim` profile does not.

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
- 0.7.1 cut checklist: `docs/internal/release/release-0.7.1.md`
- Wasm-compilable stdlib RFC: `docs/internal/rfc/rfc-0005-wasm-compilable-stdlib-v1.md`
- Runtime-in-Wasm RFC: `docs/internal/rfc/rfc-0006-runtime-in-wasm-v1.md`
- Wasm sidecar manifest: `docs/internal/runtime/wasm-module-manifest-v1.md`
- CLI module commands: `docs/internal/cli.md`
- SDK API overview: `docs/internal/sdk.md`
