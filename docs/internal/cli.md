# CLI Reference

The CLI binary is `grapheme` (package `grapheme-cli`, crate path `crates/grapheme-cli`).

## Usage

```bash
grapheme <file.gr>
grapheme parse <file.gr> [--yaml|--json]
grapheme compile <file.gr> [--emit ast|hir|mir|artifact|aot] [--aot-stage stage_a|stage_b] [--type-policy warn|strict] [--yaml|--json]
grapheme build <file.gr> [--aot-stage stage_a|stage_b] [--out path] [--yaml|--json]
grapheme plugins build [all|core|docs|io|http|memory|tcp|smtp|secrets ...]
grapheme examples [list] [--yaml|--json] [--query q] [--tag tag] [--complexity level] [--native-only]
grapheme examples show <name> [--summary] [--raw] [--yaml|--json]
grapheme examples init [--out dir]
grapheme run <file.gr> [--bind module=path.wasm ...] [--json] [--native-modules] [--aot-stage stage_a|stage_b] [--type-policy warn|strict] [--strict-stage-b] [--allow-stage-b-fallback] [--stream-steps]
                    [--trace-profile lean|debug] [--trace-steps N]
                    [--trace-projection minimal|full] [--trace-max-string-bytes N]
grapheme modules [--yaml|--json]
grapheme modules search <query> [--explain] [--detail concise|full] [--top N] [--min-score X] [--include-experimental] [--yaml|--json]
grapheme modules ops <query> [--yaml|--json]
grapheme modules info <module> [--yaml|--json]
grapheme modules types <module> [--yaml|--json]
grapheme modules examples <module> [--yaml|--json]
grapheme modules scan [paths...] [--yaml|--json]
grapheme modules activate <module> [--yaml|--json]
grapheme modules rollback <module> [--yaml|--json]
grapheme modules status [--yaml|--json]
grapheme telemetry [summarize|export] [--out path] [--yaml|--json]
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
- `--type-policy warn` is the default and preserves compatibility-mode lint behavior.
- `--type-policy strict` enables strict mutation-boundary enforcement at compile time.
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
cargo run -- run examples/pdf-generate.gr --bind pdf=modules/pdf.wasm
```

Auto-bind on run (0.6.0+): when a workflow imports a Wasm-backed capability module (`pdf`, `image`, `plot`) and a matching artifact was discovered via scan paths or hotload store, the runtime binds it without an explicit `--bind`.

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

### `examples`

List bundled examples with quick guidance:

```bash
cargo run -- examples list
```

Machine-readable list output:

```bash
cargo run -- examples list --yaml
cargo run -- examples list --json
```

Filter examples by intent/tags/complexity:

```bash
cargo run -- examples list --query web
cargo run -- examples list --tag routing --complexity advanced
cargo run -- examples list --query mutation --complexity intermediate
```

Show summary + usage guidance + source:

```bash
cargo run -- examples show web-provider-routing
```

The default `examples show` output now includes:

- summary and `use_when` guidance
- complexity and tags
- whether native modules are required
- ready-to-run command hint

Show summary-only metadata:

```bash
cargo run -- examples show web-provider-routing --summary --yaml
```

Show raw source only (legacy behavior):

```bash
cargo run -- examples show web-provider-routing --raw
```

Initialize bundled examples in your current directory (or a target dir):

```bash
cargo run -- examples init
cargo run -- examples init --out .
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

Default `modules search` now returns compact metadata per hit:

- `module_id`
- `summary`
- `op_count`
- `effects`
- `matching_ops`
- `related_examples`

Explain search matches with quick guidance (recommended for discovery and agent planning):

```bash
cargo run -- modules search web --explain --yaml
cargo run -- modules search web --detail concise --yaml
cargo run -- modules search web --detail concise --top 1 --min-score 90 --yaml
```

Opt in to experimental operation matches when needed:

```bash
cargo run -- modules search xaviv --include-experimental --yaml
```

`--explain` includes guidance fields for each match such as:

- `why_matched`
- `score` (relevance score; higher is more relevant)
- `summary`
- `use_when`
- `avoid_when`
- `related_examples`

`--detail concise` returns a smaller payload for fast ranking/selection.
`--detail full` (default explain detail tier) returns full guidance fields.
`--top` limits match count after ranking.
`--min-score` filters low-relevance matches.
`--include-experimental` opts into matching experimental operations; default behavior prefers stable operations.

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

#### Wasm discovery and hotload (0.6.0+)

Discover Wasm capability modules from sidecar manifests (`modules/*.module.json`) and built artifacts:

```bash
bash plugins/build-plugins.sh
cargo run -- modules scan
cargo run -- modules scan plugins modules --json
```

Scan paths resolve from (in order):

1. explicit CLI path arguments
2. `grapheme.toml` `[modules].scan`
3. defaults: `modules/`, `plugins/`

Activate a discovered module generation (persists hotload + legacy bindings):

```bash
cargo run -- modules activate pdf
cargo run -- modules activate image
```

Inspect persisted generation slots:

```bash
cargo run -- modules status --yaml
```

Roll back to the previous generation for a module:

```bash
cargo run -- modules rollback pdf
```

Persistence paths:

- `.grapheme/modules/hotload.json` — generation slots (primary, v1 schema `grapheme.modules.hotload/v1`)
- `.grapheme/modules/bindings.json` — legacy wasm path bindings (fallback when hotload store is absent)

`grapheme run` hydrates module state from the hotload store when present and auto-binds discovered modules referenced in the workflow. Override per run with `--bind module=path.wasm`.

Example capability workflow (requires built Wasm plugins for `plot`/`pdf`):

```bash
cargo run -- modules activate plot
cargo run -- modules activate pdf
cargo run -- run examples/platform-release-060.gr
```

Native capability examples (no Wasm activate required):

```bash
cargo run -- run examples/data-read-csv.gr
cargo run -- run examples/media-probe.gr   # requires ffmpeg/ffprobe on PATH
```

See also: `plugins/README.md`, `docs/internal/runtime/wasm-module-manifest-v1.md`, `docs/internal/sdk-feature-flags.md`.

### `telemetry`

Summarize local opt-in CLI telemetry captured during command usage.

```bash
cargo run -- telemetry summarize --yaml
cargo run -- telemetry summarize --json
```

Export a redacted, shareable report bundle (summary + redacted events):

```bash
cargo run -- telemetry export --json
cargo run -- telemetry export --yaml
cargo run -- telemetry export --out /tmp/grapheme-report.json --json
```

Telemetry defaults to local JSONL at `.grapheme/telemetry/events.jsonl`.

`telemetry summarize` includes TTFS funnel metrics:

- `ttfs_start_count`
- `ttfs_success_count`
- `ttfs_failure_count`
- `ttfs_success_rate`
- `failure_stage_counts`

`telemetry export` writes report files to:

- `.grapheme/telemetry/report.json` for `--json`
- `.grapheme/telemetry/report.yaml` for `--yaml`

Export redaction behavior:

- `examples/...` run targets are preserved
- non-example run targets are redacted to `<redacted>/<file-name>`

## Environment Variables

Runtime policy env vars consumed by CLI:

- `GRAPHEME_ALLOWED_HTTP_DOMAINS` (comma-separated hosts)
- `GRAPHEME_ALLOWED_SQL_CONNECTIONS` (comma-separated connection ids)
- `GRAPHEME_ALLOWED_SURREAL_CONNECTIONS` (comma-separated connection ids)
- `GRAPHEME_ALLOWED_TCP_TARGETS` (comma-separated host:port entries)
- `GRAPHEME_ALLOWED_SMTP_DOMAINS` (comma-separated domains)
- `GRAPHEME_ALLOWED_SECRETS` (comma-separated secret names)

Telemetry env vars:

- `GRAPHEME_TELEMETRY` (`1|true|yes|on` to enable local telemetry capture)
- `GRAPHEME_TELEMETRY_PATH` (optional JSONL output path override)

Example:

```bash
GRAPHEME_ALLOWED_SECRETS=api_key \
  cargo run -- run examples/secrets-sign.gr --native-modules
```
