# Fixtures

This directory contains validation and compatibility fixtures.

These files are not part of the curated onboarding/demo path; they are used by compiler/runtime checks, policy-profile scripts, and edge-case verification.

## Included Fixture Types

- `v1-*.gr`: control-flow and policy fixtures from the v1 language/runtime conformance effort.
- `type-error-http-url.gr`: negative type-check fixture.

## Typical Usage

Policy profile checks:

```bash
bash scripts/policy-profile-checks.sh
```

Step-4 checks:

```bash
bash scripts/step4-checks.sh
```

Manual fixture run:

```bash
cargo run -- run examples/fixtures/<file>.gr --native-modules --json
```
