# RFC-0003: Database Capability Modules v1 (sqlx + surreal)

Status: draft
Authors: runtime + stdlib
Created: 2026-05-12
Target release window: after SDK extraction baseline

## Summary

Introduce first-class database capability modules for Grapheme:

1. `sql` module for relational access (initially Postgres via sqlx).
2. `surreal` module for SurrealDB operations.

Both modules follow the same capability-governed model as existing network and secret modules, with explicit policy admission, bounded resource controls, deterministic trace shapes, and typed operation contracts.

## Motivation

Current strengths:

1. Capability policy model and runtime guards are already in place.
2. Registry/signature/manifest conformance checks are now enforced in tests.
3. CLI and discovery surfaces are maturing for LLM-native workflows.

Current gaps:

1. No native data-store capability surface in core module set.
2. Workflows needing durable state require out-of-band glue code.
3. No standard transaction semantics for mutation-heavy flows.

Desired outcome:

- Grapheme workflows can safely query and mutate persistent state using explicit policy-managed capability modules.

## Goals

1. Add stable `sql.*` relational ops with sqlx-backed execution.
2. Add stable `surreal.*` ops for SurrealDB.
3. Define capability policy controls for host allowlists and operation classes.
4. Standardize result/error/trace payload shapes for database steps.
5. Preserve deterministic execution behavior and bounded runtime resource usage.

## Non-Goals

1. Full ORM abstraction or schema migration engine.
2. Cross-region replication orchestration.
3. Multi-statement implicit transactions by default.
4. Auto-generated query builders in v1.

## Proposed Modules

### Module: `sql` (Relational)

Initial driver target:

1. Postgres via sqlx.

Planned op surface (v1):

1. `sql.query`
2. `sql.execute`
3. `sql.transaction`
4. `sql.health`

Input/Output contract direction:

1. `sql.query`
- Input: `{ connection, sql, params?, timeout_ms? }`
- Output: `{ rows, row_count, elapsed_ms }`

2. `sql.execute`
- Input: `{ connection, sql, params?, timeout_ms? }`
- Output: `{ rows_affected, elapsed_ms }`

3. `sql.transaction`
- Input: `{ connection, steps, isolation?, timeout_ms? }`
- Output: `{ committed, results, elapsed_ms }`

4. `sql.health`
- Input: `{ connection, timeout_ms? }`
- Output: `{ ok, latency_ms, server_version? }`

### Module: `surreal`

Planned op surface (v1):

1. `surreal.query`
2. `surreal.select`
3. `surreal.create`
4. `surreal.update`
5. `surreal.delete`
6. `surreal.health`

Input/Output contract direction:

1. `surreal.query`
- Input: `{ connection, query, vars?, timeout_ms? }`
- Output: `{ result, elapsed_ms }`

2. `surreal.select`
- Input: `{ connection, thing_or_table, where?, limit?, timeout_ms? }`
- Output: `{ result, elapsed_ms }`

3. `surreal.create|update|delete`
- Input: operation-specific payload with deterministic id fields where applicable.
- Output: `{ result, elapsed_ms }`

4. `surreal.health`
- Input: `{ connection, timeout_ms? }`
- Output: `{ ok, latency_ms, server_version? }`

## Policy Model

Extend runtime policy with DB-specific guardrails:

1. `allowed_sql_connections: Vec<String>`
2. `allowed_db_schemas: Vec<String>` (optional, relational scope)
3. `allowed_db_ops: Vec<String>` (for example `query`, `execute`, `transaction`)
4. `allowed_surreal_namespaces: Vec<String>`
5. `allowed_surreal_databases: Vec<String>`

Admission rules:

1. Connection id must be explicitly allowed.
2. Operation class must be allowed.
3. Transaction op can be disabled independently.
4. Optional schema/namespace/database constraints apply before execution.

## Runtime Limits and Safety

Required limits (v1 defaults):

1. `max_rows_returned` (for example 5_000)
2. `max_result_bytes` (for example 2 MiB)
3. `max_query_time_ms` (for example 10_000)
4. `max_transaction_steps` (for example 50)

Safety behaviors:

1. Hard timeout cancellation on driver side where possible.
2. Deterministic truncation marker when rows are clipped.
3. Optional parameter redaction in traces.
4. SQL text normalization for trace consistency.

## Transactions

`sql.transaction` execution model:

1. Begin transaction with explicit isolation level when requested.
2. Execute step list serially.
3. On first failure, roll back and return structured failure payload.
4. On success, commit and return per-step results.

Compatibility note:

- v1 transaction scope is single-connection, single-request, and not nested.

## Trace and Error Shape

### Success shape

1. Include `elapsed_ms` in all sql/surreal outputs.
2. Include stable aggregate fields (`row_count`, `rows_affected`, `committed`) when relevant.

### Failure shape

Use normalized error payload fields:

1. `kind` (for example `policy_denied`, `connection_error`, `query_error`, `timeout`, `decode_error`)
2. `code` (driver/runtime code when available)
3. `message`
4. `retryable` boolean

### Trace projection

1. Minimal projection redacts params by default.
2. Full projection may include params when `trace_policy` allows and redaction is disabled.

## Signatures and Manifest Conformance

1. Add op signatures in `grapheme-signatures` for all new ops.
2. Add module manifests in runtime for `sql` and `surreal`.
3. Ensure stdlib registry dispatch matches signatures and manifests.
4. Extend existing conformance tests to include both modules.

## Implementation Plan

### Phase 1: Contracts and Stubs

1. Add signatures and manifests.
2. Add stdlib registry entries with explicit stub behavior and deterministic errors.
3. Add policy fields and default deny behavior.

### Phase 2: Driver Integration

1. Integrate sqlx-backed relational adapter.
2. Integrate SurrealDB adapter.
3. Wire runtime limits and timeout controls.

### Phase 3: Hardening

1. Add integration tests for policy-denied and allowlisted paths.
2. Add transaction rollback tests.
3. Add load tests for bounded result handling.

## Testing Strategy

1. Unit tests for normalization, policy checks, and limit enforcement.
2. Contract tests for signature/manifest/registry parity.
3. Integration tests against ephemeral Postgres and SurrealDB services.
4. Snapshot tests for YAML/JSON output consistency in CLI discovery.

## Open Questions

1. Should `sql.query` allow arbitrary SQL in v1 or require operation class allowlists by statement kind?
2. Do we expose prepared statement handles in v1 or keep call-level stateless semantics only?
3. Should Surreal graph relations get dedicated ops in v1 or remain query-driven?

## Acceptance Criteria

1. Workflows can execute policy-allowed relational and Surreal operations through capability modules.
2. Policy-denied operations fail with deterministic structured errors.
3. Transaction rollback behavior is validated and deterministic.
4. Conformance tests prevent signature/manifest/registry drift for new modules.
5. Runtime limit violations are enforced with clear diagnostics.
