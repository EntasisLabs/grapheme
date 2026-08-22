# Examples

This directory now prioritizes a small, high-signal set of canonical examples for the stdlib surface.

It now also includes a first-class real-world scenario pack under `examples/realworld/`.

Canonical examples prefer the newer language sugar when behavior is stateful/branching (`set`, `transition`, inline `if/match`, bare iterator invocation).

Authoring convention:

- seed request/state with `set` at query start
- pass values through explicit `$current.*` refs
- keep pipelines linear and avoid legacy `state { current }` wrappers in normal examples

## Canonical Stdlib Set

Run these first:

- `main.gr` (project default entrypoint from `grapheme.toml`)

- `hello-world.gr`

### 0.7.0 language (params + tags)

- `params-call-bind.gr` — executable parameters + `call` bind; try `--args-json '{"label":"grapheme"}'`
- `tag-using-scope.gr` — `tag` schema + scoped `using`

Author extract: `docs/internal/language/params-and-tags-v1.md`.

### Core / control flow

- `core-merge.gr`
- `core-filter.gr`
- `core-validate-schema.gr`
- `mutation-update-preferences.gr`
- `mutation-state-machine-apply.gr`
- `resilience-composition.gr`
- `subscription-heartbeat-readable.gr`

### Host I/O and search

- `http-get.gr`
- `request-transform-output.gr`
- `websearch-basic.gr`
- `websearch-materials.gr`
- `websearch-report.gr`
- `web-provider-catalog.gr`
- `web-provider-routing.gr`
- `web-xaviv-planned.gr`
- `web-tavily.gr` (requires `TAVILY_API_KEY`)
- `web-brave.gr` (requires `BRAVE_API_KEY`)
- `io-list.gr`
- `memory-roundtrip.gr`
- `tcp-connect.gr`
- `smtp-send.gr`
- `email-smtp.gr`
- `secrets-handle.gr`
- `secrets-sign.gr`

### SQL / Surreal

- `sql-query.gr`
- `sql-query-params.gr` (SQL `?` bind params — not RFC-0004 executable params)
- `sql-transaction.gr`
- `sql-transaction-rollback.gr`
- `surreal-select.gr`
- `surreal-query.gr`
- `surreal-select-filtered.gr`
- `surreal-query-vars.gr`
- `surreal-health.gr`
- `surreal-create.gr`
- `surreal-update.gr`
- `surreal-delete.gr`

### Capability modules (0.6.0+, still current)

- `data-read-csv.gr` (requires CLI/SDK `full` or `data` feature)
- `data-filter.gr` (requires CLI/SDK `full` or `data` feature)
- `pdf-generate.gr` (requires `modules/pdf.wasm`; build with `plugins/build-plugins.sh`)
- `image-metadata.gr` (requires `modules/image.wasm`)
- `plot-line.gr` (requires `modules/plot.wasm`)
- `media-probe.gr` (native; requires `ffmpeg`/`ffprobe` on `PATH`)
- `platform-release-060.gr` (data + plot + pdf; build plugins and `modules activate plot pdf`)

Pipeline tip:

- Every step rewrites `$current`, so prefer extracting fields (`core.get_path`, `core.pick`) before replacing shape-heavy payloads with `core.echo`.

## Real-World Scenario Pack

Run these to see production-like automation workflows with meaningful branching and operational outcomes:

- `examples/realworld/automated-release-control-tower.gr`
- `examples/realworld/blue-green-cutover-guarded.gr`
- `examples/realworld/feature-flag-progressive-rollout.gr`
- `examples/realworld/oncall-escalation-ladder.gr`
- `examples/realworld/research-decision-loop.gr`

Scenario index and intent notes:

- `examples/realworld/README.md`

## Legacy and Exploratory

Earlier exploratory material is preserved under `examples/legacy/`.

- `examples/legacy/showcase/`
- `examples/legacy/transform-cookbook/`
- older ad-hoc experiments and transitional files

## Fixtures

Compatibility and verifier fixtures remain in `examples/fixtures/`.

## Namespace Layout

Project config (`grapheme.toml`) declares these example namespaces:

- `core` -> `examples/`
- `showcase` -> `examples/legacy/showcase/`
- `fixtures` -> `examples/fixtures/`

## Run

```bash
cargo run -- run examples/<file>.gr --native-modules
```

0.7.0 language + Stage B:

```bash
cargo run -- run examples/params-call-bind.gr --args-json '{"label":"grapheme"}' --json
cargo run -- run examples/tag-using-scope.gr --json
./scripts/build-aot-container.sh
cargo run -- build examples/hello-world.gr --aot-stage stage_b --json
cargo run -- run examples/hello-world.gr --aot-stage stage_b --json
```

Capability modules (0.6.0+, still current):

```bash
bash plugins/build-plugins.sh
cargo run -- modules scan
cargo run -- modules activate pdf
cargo run -- run examples/pdf-generate.gr
cargo run -- run examples/data-read-csv.gr
cargo run -- run examples/media-probe.gr
cargo run -- run examples/platform-release-060.gr
```

For SQL examples, allow the selected connection id in policy:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
	cargo run -- run examples/sql-query-params.gr --json
```

Run real-world scenarios:

```bash
cargo run -- run examples/realworld/automated-release-control-tower.gr --json
cargo run -- run examples/realworld/blue-green-cutover-guarded.gr --json
```
