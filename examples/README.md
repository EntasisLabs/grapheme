# Examples

This directory now prioritizes a small, high-signal set of canonical examples for the stdlib surface.

Canonical examples prefer the newer language sugar when behavior is stateful/branching (`set`, `transition`, inline `if/match`, bare iterator invocation).

Authoring convention:

- seed request/state with `set` at query start
- pass values through explicit `$current.*` refs
- keep pipelines linear and avoid legacy `state { current }` wrappers in normal examples

## Canonical Stdlib Set

Run these first:

- `main.gr` (project default entrypoint from `grapheme.toml`)

- `hello-world.gr`
- `core-merge.gr`
- `core-filter.gr`
- `core-validate-schema.gr`
- `mutation-update-preferences.gr`
- `mutation-state-machine-apply.gr`
- `resilience-composition.gr`
- `subscription-heartbeat-readable.gr`
- `http-get.gr`
- `request-transform-output.gr`
- `websearch-materials.gr`
- `websearch-report.gr`
- `web-provider-catalog.gr`
- `web-provider-routing.gr`
- `web-xaviv-planned.gr`
- `io-list.gr`
- `memory-roundtrip.gr`
- `tcp-connect.gr`
- `smtp-send.gr`
- `secrets-handle.gr`
- `secrets-sign.gr`
- `sql-query.gr`
- `sql-query-params.gr`
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

Pipeline tip:

- Every step rewrites `$current`, so prefer extracting fields (`core.get_path`, `core.pick`) before replacing shape-heavy payloads with `core.echo`.

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

For SQL examples, allow the selected connection id in policy:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
	cargo run -- run examples/sql-query-params.gr --json
```
