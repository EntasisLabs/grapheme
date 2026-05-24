# Scenario Playbooks (v1)

Status: active guidance set  
Last updated: 2026-05-24

## Purpose

Provide task-first, production-like scenarios that go beyond first-run onboarding and can be improved weekly using telemetry signals.

## Playbook 1: Web Research Report

Goal: run a full search-to-report pipeline and inspect structured output.

Primary example:

- `examples/websearch-report.gr`

Run:

```bash
cargo run -- run examples/websearch-report.gr --json
```

Expected outcome:

- Output includes `"outcome": "succeeded"`.
- Final state includes report-oriented fields derived from fetched materials.

Common failure stages:

- `policy` when outbound web access is denied by policy setup.
- `runtime` for provider/network-level failures.

Telemetry hook:

- Weekly review should compare `runtime` and `policy` failure_stage_counts for `run` commands.
- If `policy` dominates, improve preflight instructions in `docs/runtime-policy.md`.

## Playbook 2: SQL Transaction Safety

Goal: execute transactional SQL workflows and verify rollback/safe behavior.

Primary examples:

- `examples/sql-transaction.gr`
- `examples/sql-transaction-rollback.gr`

Run:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
  cargo run -- run examples/sql-transaction.gr --json

GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
  cargo run -- run examples/sql-transaction-rollback.gr --json
```

Expected outcome:

- Successful command outcomes in JSON mode.
- Rollback scenario demonstrates failed unit behavior without committing unintended writes.

Common failure stages:

- `policy` when SQL connection ids are missing from allow-lists.
- `runtime` for connection string/provider issues.

Telemetry hook:

- Track weekly top error classes for `runtime_error` and `policy_denied`.
- If repeated SQL policy misses occur, prioritize safer defaults/presets docs.

## Playbook 3: Secrets Signing Flow

Goal: use secret handles/signing flow safely with explicit allow-list policy.

Primary examples:

- `examples/secrets-handle.gr`
- `examples/secrets-sign.gr`

Run:

```bash
GRAPHEME_ALLOWED_SECRETS=api_key \
  cargo run -- run examples/secrets-handle.gr --native-modules --json

GRAPHEME_ALLOWED_SECRETS=api_key \
  cargo run -- run examples/secrets-sign.gr --native-modules --json
```

Expected outcome:

- Output includes `"outcome": "succeeded"`.
- Secret operations succeed without leaking raw secret values.

Common failure stages:

- `policy` when secret names are not allow-listed.
- `runtime` for plugin/module binding errors.

Telemetry hook:

- Monitor policy failure concentration in weekly rollups.
- If secrets policy denials trend up, tighten docs examples around exact env var setup.

## Weekly Improvement Loop

1. Run these three scenarios in docs smoke checks or manual validation.
2. Compare weekly telemetry rollup failure stages and top error classes.
3. Update one playbook or one troubleshooting section each week based on dominant failures.
4. Log the linked change in roadmap updates.

## Related Docs

- `docs/getting-started.md`
- `docs/cli.md`
- `docs/runtime-policy.md`
- `docs/quality/telemetry-reporting-cadence.md`