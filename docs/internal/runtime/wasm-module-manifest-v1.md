# Wasm Module Manifest v1

Status: **draft (0.6.0)**
Schema: `schemas/wasm-module-manifest.schema.json`

## Purpose

Every dynamically discovered Grapheme Wasm capability module MUST ship a manifest sidecar so the runtime, compiler, policy layer, and LSP share one contract.

## File layout

For a module file `modules/pdf.wasm`, the manifest lives at:

```
modules/pdf.wasm
modules/pdf.module.json
```

Naming rule: `<wasm_basename>.module.json` adjacent to the Wasm file.

Alternative (explicit bind in `grapheme.toml`):

```toml
[modules.bind]
pdf = "modules/pdf.wasm"
```

When bound explicitly, the manifest is still resolved from `modules/pdf.module.json`.

## Manifest shape

```json
{
  "schema": "grapheme.module.manifest/v1",
  "module_id": "pdf",
  "version": "0.1.0",
  "abi": "wasix_v1",
  "wasm": "pdf.wasm",
  "entrypoint": "pdf.main",
  "exported_ops": [
    {
      "op": "generate",
      "effect": "network",
      "input_schema_ref": "pdf.generate.input.v1",
      "output_schema_ref": "pdf.generate.output.v1"
    }
  ],
  "required_capabilities": ["pdf.generate"],
  "limits": {
    "max_cpu_ms": 5000,
    "max_memory_mb": 256,
    "max_io_bytes": 10485760,
    "max_network_calls": 10
  }
}
```

## Field rules

| Field | Required | Notes |
| --- | --- | --- |
| `schema` | yes | Must be `grapheme.module.manifest/v1` |
| `module_id` | yes | Lowercase identifier; matches language `module.op` prefix |
| `version` | yes | Semver of this Wasm artifact |
| `abi` | yes | `wasix_v1` for Wasm plugins; `mir_v1` is host-only |
| `wasm` | yes | Filename relative to manifest directory |
| `entrypoint` | no | Reserved for future export naming |
| `exported_ops` | yes | Non-empty; each op must exist in `grapheme-signatures` or be experimental-tagged |
| `required_capabilities` | yes | Policy admission names (`module.action.scope`) |
| `limits` | no | Defaults to runtime standard limits when omitted |

## Discovery algorithm

1. Read `[modules].scan` paths from `grapheme.toml` (relative to project root).
2. For each directory, enumerate `**/*.wasm`.
3. Resolve sibling `<stem>.module.json`.
4. Parse and validate against JSON schema.
5. Verify Wasm file exists and hash matches optional `content_sha256` (future).
6. Register as **candidate generation** (not active until activation passes compatibility checks).

## Activation compatibility (v1)

Hard requirements before a generation becomes active:

1. `module_id` matches the target slot (new modules create a new slot).
2. `abi` is `wasix_v1`.
3. Every `exported_ops[].op` is known to signatures OR marked experimental in manifest.
4. `required_capabilities` ⊆ policy allowlist for the runtime profile.
5. Wasm module compiles and is WASI/WASIX compatible.

On failure: generation marked `Failed`; active pointer unchanged.

## Stdin/stdout contract (Wasm plugins)

Unchanged from existing plugin model:

- **stdin:** `{ "op": "<operation>", "args": { ... } }`
- **stdout:** capability result JSON (normalized to `{ data, meta, error }` by host adapter)

## Relationship to host-native modules

Host-native capability modules (`data`, `media`) do not use this sidecar format at runtime — they register via `grapheme-stdlib` and `grapheme-signatures`. Wasm manifests are for **extension** modules (`pdf`, `image`, `plot`, community modules).

## Example discovery output

`grapheme modules scan`:

```json
{
  "count": 2,
  "modules": [
    {
      "module_id": "pdf",
      "version": "0.1.0",
      "abi": "wasix_v1",
      "wasm_path": "modules/pdf.wasm",
      "manifest_path": "modules/pdf.module.json",
      "exported_ops": ["generate", "extract_text"]
    }
  ]
}
```
