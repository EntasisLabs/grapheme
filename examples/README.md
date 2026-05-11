# Examples

The examples directory is curated by intent so you can find runnable programs quickly.

Use this page as the source of truth for which examples to demo and learn first.

## Canonical Starter Set

If you only run a handful of programs, use these.

### Primitive Language Examples

- `hello-world.gr`
- `core-merge.gr`
- `core-filter.gr`
- `core-validate-schema.gr`
- `fibonacci-sequence.gr` (static loop/each list walk)
- `fibonacci-computed.gr` (computed loop progression)

### Module-Using Examples

- `http-get.gr`
- `tcp-connect.gr`
- `smtp-send.gr`
- `io-list.gr`
- `memory-roundtrip.gr`
- `secrets-handle.gr`
- `secrets-sign.gr`
- `request-transform-output.gr`
- `docs-native-modules.gr`

### Workflow/Control-Plane Showcases

- `showcase/release-control-tower-compact.gr`
- `showcase/blue-green-cutover.gr`
- `showcase/feature-flag-progressive-rollout.gr`
- `showcase/oncall-escalation-ladder.gr`

See `examples/showcase/README.md` for scenario details and live-demo toggle ideas.

## Extended Sets

- Transform cookbook: `examples/transform-cookbook/README.md`
- Additional showcase programs: `examples/showcase/README.md`

## Verification and Historical Files

These are kept for compatibility checks, regression fixtures, and earlier design exploration.

- `fixtures/v1-*.gr` files: parser/verifier/runtime fixtures and policy edge cases.
- `fixtures/type-error-http-url.gr`: negative type-check fixture.

## Run Commands

Run a root-level example:

```bash
cargo run -- run examples/<file>.gr --native-modules
```

Run any showcase program:

```bash
cargo run -- run examples/showcase/<file>.gr --native-modules
```

Optional output formats:

```bash
cargo run -- run examples/showcase/<file>.gr --native-modules --json
cargo run -- run examples/showcase/<file>.gr --native-modules --stream-steps
```

## Quick Smoke Path

```bash
cargo run -- run examples/hello-world.gr --native-modules
cargo run -- run examples/io-list.gr --native-modules
cargo run -- run examples/showcase/release-control-tower-compact.gr --native-modules
```
