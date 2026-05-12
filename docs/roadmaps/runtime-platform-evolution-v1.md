# Grapheme Runtime Platform Evolution v1

Status: proposed
Owner: runtime + compiler + cli
Horizon: 4-6 sprints

## Why This Exists

Grapheme has reached a strong CLI + language baseline. The next step is turning it into an embeddable, production-grade agent runtime platform for Rust applications, with stronger module lifecycle control and richer data-plane integrations.

This plan captures four major bets and the recommended implementation sequence:

1. Embedded runtime SDK for Rust developers.
2. Database capability modules (`sqlx` and SurrealDB).
3. Versioned hot module loading for Wasm plugins.
4. AOT-to-Wasm pipeline for safe portable execution.

## Strategy

Recommended order:

1. Embedded SDK first.
2. Hot module loading second.
3. Database capabilities third.
4. AOT-to-Wasm fourth.

Reasoning:

- SDK work hardens public runtime boundaries and removes CLI coupling.
- Hot-loading needs explicit runtime lifecycle ownership.
- Database support should be capability-first modules, not core-runtime coupling.
- AOT benefits from stable module/ABI/runtime contracts.

## Track 1: Embedded Runtime SDK

Goal:

Enable Rust developers to embed Grapheme directly into agentic projects without shelling out to CLI.

Deliverables:

1. New crate surface for embedding runtime execution.
2. Builder-based engine API for policy, tracing, module registry, and limits.
3. Compile + execute API returning typed result envelopes.
4. Host interception hooks for capability calls and observability.
5. CLI migrated to thin adapter over SDK.

Acceptance criteria:

1. A Rust app can execute `.gr` source and artifact payloads in-process.
2. Runtime behavior matches CLI `run` parity for policy and traces.
3. SDK has structured output modes suitable for LLM orchestration (`yaml` default, `json` optional).

## Track 2: Database Capability Modules

Goal:

Add first-class database operations while preserving policy-first runtime safety.

Design principle:

- Capability modules over ORM magic.
- Explicit operation contracts and typed mappings.

### SQL path (`sqlx`)

Proposed module: `sql`

Initial ops:

1. `sql.query`
2. `sql.execute`
3. `db.begin`
4. `db.commit`
5. `db.rollback`

Required controls:

1. Bound parameters only (no raw string interpolation defaults).
2. Read-only policy mode for `query` executables.
3. Per-op timeout and row/bytes limits.
4. Connection and schema allowlists.

### Surreal path

Proposed module: `surreal`

Initial ops:

1. `surreal.select`
2. `surreal.upsert`
3. `surreal.graph_traverse`
4. `surreal.vector_search`

Required controls:

1. Namespace/database scope guardrails.
2. Predicate, limit, and egress caps.
3. Deterministic operation tracing.

Acceptance criteria:

1. SQL and Surreal modules ship as capability plugins with manifest declarations.
2. Policies can independently gate SQL and Surreal operations.
3. Conformance tests verify registry ↔ manifests ↔ signatures parity.

## Track 3: Wasm Hot Module Loading

Goal:

Adopt CLR/DLL-style module lifecycle behavior with safe runtime swapping.

Core model:

1. Versioned module generations.
2. In-flight execution pinning.
3. New execution activation on compatible generations.
4. Rollback to prior generation on failed activation.

Required runtime primitives:

1. Module generation id and hash tracking.
2. ABI/signature compatibility validator.
3. Module lifecycle events: loaded, activated, drained, retired, failed.
4. Cache invalidation rules for affected artifacts.

Acceptance criteria:

1. Module updates do not break in-flight workflows.
2. Incompatible updates are rejected pre-activation.
3. Activation and rollback events are traceable.

## Track 4: AOT to Wasm

Goal:

Compile Grapheme workflows into portable safe Wasm payloads for sandboxed production execution.

Phased approach:

1. Stage A: AOT lower to optimized artifact while retaining runtime host.
2. Stage B: Emit workflow Wasm container with constrained host interface.
3. Stage C: Add optimization and partial evaluation passes.

Security and safety constraints:

1. No host escape beyond declared capability imports.
2. Policy guard remains mandatory at runtime boundary.
3. Artifact provenance includes compiler/runtime compatibility metadata.

Acceptance criteria:

1. AOT output executes deterministically under bounded policy.
2. Same workflow result parity against interpreted path for reference cases.
3. Packaging is deployable to common Wasm runtimes with policy adapters.

## Cross-Cutting Workstreams

1. Contract governance:
- Maintain signatures ↔ registry ↔ manifest conformance gates.

2. Observability:
- Stable trace schema across embedded/runtime/plugin/AOT paths.

3. Test strategy:
- Golden output contracts for CLI + SDK.
- Lifecycle tests for hot-reload and rollback.
- Compatibility matrix by module ABI and runtime versions.

4. Docs and migration:
- Keep CLI docs additive while SDK becomes preferred integration path.

## Proposed Sprint Plan

1. Sprint 1: SDK foundation and CLI adapter extraction.
2. Sprint 2: Module manager generations + compatibility checks.
3. Sprint 3: DB capability modules (`sql` and `surreal`) with policy gating.
4. Sprint 4-5: AOT pipeline stage A/B and parity tests.

## Out of Scope (v1)

1. Full ORM behavior with implicit schema migration.
2. Distributed module registry service.
3. Cross-language SDKs beyond Rust.
