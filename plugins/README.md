# Grapheme Wasm capability plugins

Reference Wasm plugins for 0.6.0 capability modules. Each plugin reads JSON from stdin:

```json
{ "op": "<operation>", "args": { ... } }
```

and writes a host result envelope to stdout:

```json
{ "data": { ... }, "meta": { "schema": "grapheme.host.result.envelope/v1" }, "error": null }
```

## Capability plugins (0.6.0)

| Module | Plugin dir | Manifest | Ops |
| --- | --- | --- | --- |
| `pdf` | `plugins/pdf-rs` | `modules/pdf.module.json` | `generate`, `extract_text` |
| `image` | `plugins/image-rs` | `modules/image.module.json` | `metadata`, `resize`, `convert` |
| `plot` | `plugins/plot-rs` | `modules/plot.module.json` | `line`, `bar`, `scatter` |

## Build

```bash
rustup target add wasm32-wasip1
bash plugins/build-plugins.sh
```

This copies built artifacts into `modules/*.wasm` for discovery via `grapheme modules scan`.

## Run

```bash
grapheme modules scan
grapheme modules activate image
grapheme run examples/image-metadata.gr
```

Or rely on auto-bind during `grapheme run` when the workflow references a discovered module.

## Legacy core plugins

Older host-adjacent plugins (`io-rs`, `core-echo-rs`, …) remain under `plugins/` for `--native-modules` builds.
