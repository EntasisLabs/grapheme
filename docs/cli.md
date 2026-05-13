# CLI Reference

The CLI binary is `grapheme` (package `grapheme-cli`, crate path `crates/grapheme-cli`).

## Usage

```bash
grapheme <file.gr>
grapheme parse <file.gr> [--yaml|--json]
grapheme compile <file.gr> [--emit ast|hir|mir|artifact|aot] [--aot-stage stage_a|stage_b] [--yaml|--json]
grapheme build <file.gr> [--aot-stage stage_a|stage_b] [--out path] [--yaml|--json]
grapheme plugins build [all|core|docs|io|http|memory|tcp|smtp|secrets ...]
grapheme run <file.gr> [--bind module=path.wasm ...] [--json] [--native-modules] [--aot-stage stage_a|stage_b] [--strict-stage-b] [--allow-stage-b-fallback] [--stream-steps]
                    [--trace-profile lean|debug] [--trace-steps N]
                    [--trace-projection minimal|full] [--trace-max-string-bytes N]
grapheme modules [--yaml|--json]
grapheme modules search <query> [--yaml|--json]
grapheme modules ops <query> [--yaml|--json]
grapheme modules info <module> [--yaml|--json]
grapheme modules types <module> [--yaml|--json]
grapheme modules examples <module> [--yaml|--json]
```

## Commands

### `parse`

Parses source and prints AST output. Default output format is YAML; use `--json` for JSON.

```bash
cargo run -- parse examples/hello-world.gr
```

### `compile`

Compiles source and prints selected intermediate or final representation.

```bash
cargo run -- compile examples/hello-world.gr --emit ast
cargo run -- compile examples/hello-world.gr --emit hir
cargo run -- compile examples/hello-world.gr --emit mir
cargo run -- compile examples/hello-world.gr --emit artifact
cargo run -- compile examples/hello-world.gr --emit aot --aot-stage stage_a --json
cargo run -- compile examples/hello-world.gr --emit aot --aot-stage stage_b --json
```

Notes:

- `--aot-stage` is only valid with `--emit aot`.
- For `compile`, default emit target is `mir`.
- For `compile`, default output format is YAML.

### `build`

Compiles source to AOT and writes output files to disk.

Default behavior:

- default AOT stage is `stage_b`
- default output format is JSON
- default output path is `<file>.aot.<json|yaml>`
- also writes `<output>.manifest.json`

Examples:

```bash
cargo run -- build examples/hello-world.gr
cargo run -- build examples/hello-world.gr --aot-stage stage_a --out build/hello-stage-a.aot.json --json
cargo run -- build examples/hello-world.gr --aot-stage stage_b --out build/hello-stage-b.aot.yaml --yaml
```

### `run`

Compiles and executes a program.

Plain text output (best-effort extraction from final state):

```bash
cargo run -- run examples/hello-world.gr
```

By default, plain mode prints actual emitted outputs (for example `core.echo` messages) without step trace prefixes.

Opt-in step trace streaming with prefixes like `[iter 1 | depth 1 | echo]`:

```bash
cargo run -- run examples/legacy/fibonacci-computed.gr --native-modules --stream-steps
```

Structured JSON output:

```bash
cargo run -- run examples/hello-world.gr --json
```

`--json` output now includes `lint_warnings` (non-fatal compile-time advisory lints) so agents can consume warnings structurally instead of parsing stderr text.

Manual module binding:

```bash
cargo run -- run examples/http-get.gr --bind http=plugins/http-rs.wasm
```

Auto-build and auto-bind known modules:

```bash
cargo run -- run examples/core-merge.gr --native-modules
```

`--native-modules` auto-builds/auto-binds known Wasm plugins except `http`, `tcp`, and `smtp`, which are host-backed by default for real socket access in the current runtime.

AOT execution mode:

```bash
cargo run -- run examples/hello-world.gr --aot-stage stage_a --json
cargo run -- run examples/hello-world.gr --aot-stage stage_b --json
```

Stage B strict/fallback behavior:

- Stage B runs default to strict container-first mode.
- `--strict-stage-b` forces strict mode explicitly.
- `--allow-stage-b-fallback` opts out of strict mode for Stage B runs and allows parity fallback.

Trace tuning:

```bash
cargo run -- run examples/core-merge.gr --native-modules --trace-profile debug --trace-steps 512 --trace-projection full --trace-max-string-bytes 2048
```

### `plugins build`

Builds plugin crates for `wasm32-wasip1` and copies canonical outputs to `plugins/*.wasm`.

```bash
cargo run -- plugins build all
cargo run -- plugins build core io http
```

### `modules`

Prints runtime module manifests.

Default output format is YAML; use `--json` for JSON.

```bash
cargo run -- modules
```

Search manifests by module id or op name:

```bash
cargo run -- modules search http
```

Search operations across modules:

```bash
cargo run -- modules ops get
```

Show one manifest:

```bash
cargo run -- modules info http
```

Show op type metadata for one module:

```bash
cargo run -- modules types http
```

Show curated example files for one module:

```bash
cargo run -- modules examples http
```

Web search module example:

```bash
cargo run -- modules examples websearch
```

Run the high-level research pipeline (search -> fetch -> clean -> report):

```bash
cargo run -- run examples/websearch-report.gr --json
```

Run the composable research-materials pipeline (search -> fetch -> markdown + structured payload):

```bash
cargo run -- run examples/websearch-materials.gr --json
```

Use this when an LLM should decide its own output shape (summary, memo, citations, markdown artifact)
from source-grounded materials rather than relying on a fixed built-in report template.

For the expanded core std helpers, discover examples with:

```bash
cargo run -- modules examples core
```

## Environment Variables

Runtime policy env vars consumed by CLI:

- `GRAPHEME_ALLOWED_HTTP_DOMAINS` (comma-separated hosts)
- `GRAPHEME_ALLOWED_SQL_CONNECTIONS` (comma-separated connection ids)
- `GRAPHEME_ALLOWED_SURREAL_CONNECTIONS` (comma-separated connection ids)
- `GRAPHEME_ALLOWED_TCP_TARGETS` (comma-separated host:port entries)
- `GRAPHEME_ALLOWED_SMTP_DOMAINS` (comma-separated domains)
- `GRAPHEME_ALLOWED_SECRETS` (comma-separated secret names)

Example:

```bash
GRAPHEME_ALLOWED_SECRETS=api_key \
  cargo run -- run examples/secrets-sign.gr --native-modules
```
