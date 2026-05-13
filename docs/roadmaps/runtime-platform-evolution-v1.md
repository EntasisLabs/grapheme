# Grapheme Runtime Platform Evolution v1

Status: in progress
Owner: runtime + compiler + cli
Horizon: 4-6 sprints

Progress snapshot (2026-05-12):

1. Track 1 (Embedded Runtime SDK): complete
2. Track 2 (Database Capability Modules): complete for v1 acceptance scope
3. Track 3 (Wasm Hot Module Loading): complete for v1 acceptance scope
4. Track 4 (AOT to Wasm): in progress (Stage A scaffold landed)

Current value-first execution lane:

1. Start Track 4 planning + Stage A implementation.
2. Keep optional DB live-backend integration runs as non-blocking confidence work.

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

Current execution order (actual):

1. Embedded SDK first.
2. Database capabilities second.
3. Hot module loading next.
4. AOT-to-Wasm last.

Reasoning:

- SDK work hardens public runtime boundaries and removes CLI coupling.
- Hot-loading needs explicit runtime lifecycle ownership.
- Database support should be capability-first modules, not core-runtime coupling.
- AOT benefits from stable module/ABI/runtime contracts.

## Track 1: Embedded Runtime SDK

Status: complete

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

Implementation notes:

1. SDK crate exists at `crates/grapheme-sdk`.
2. CLI `run` path is a thin adapter over SDK execution and formatting.
3. Builder supports policy, tracing, module bindings, execution limits, and host interception hooks.

## Track 2: Database Capability Modules

Status: complete for v1 acceptance scope

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

Remaining high-value closure checklist:

1. Optional: add live-backend integration runs (ephemeral Postgres/Surreal services) for deeper end-to-end confidence.

## Track 3: Wasm Hot Module Loading

Status: complete for v1 acceptance scope

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

Progress implemented:

1. Module manager generation model and lifecycle/event types exist in runtime.
2. Runtime activation and rollback APIs update active module generation metadata.
3. Per-execution module registry pinning is in place so in-flight executions are isolated.
4. Lifecycle events are now surfaced into execution state output.
5. Activation-time compatibility checks now include signature-op coverage and capability policy admission validation.
6. Draining-to-retired lifecycle behavior is implemented with deterministic retirement criteria and tests.
7. Lifecycle event payload contract is stabilized with namespaced event kinds (`module.*`) and schema assertions in runtime tests.
8. Explicit split-generation proof is covered: execution A pinned to generation N while execution B resolves generation N+1 after activation.

Remaining high-value closure checklist:

1. Optional: extend split-proof coverage to multi-threaded stress runs and activation throughput benchmarks.

## Track 4: AOT to Wasm

Status: in progress (Stage A scaffold)

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

Progress implemented:

1. Stage A AOT envelope and compatibility metadata are defined in artifact contracts.
2. Compiler now exposes Stage A AOT compile entrypoints (source -> artifact -> AOT envelope).
3. Stage A parity harness tests verify base artifact shape parity and metadata propagation.
4. SDK parity execution harness validates interpreted artifact vs Stage A base artifact output equivalence for representative workflows.
5. CLI compile emit supports `aot` output, and SDK exposes AOT compile/execute/format helpers for inspection workflows.
6. AOT Stage A contract tests are wired into CI conformance workflow.
7. Stage B scaffold landed: artifact/compiler/SDK now support a workflow Wasm container metadata envelope (`grapheme.aot.stage_b.v1`).
8. Host-interface boundary validation is now enforced for AOT execution (`grapheme.runtime.host.v1::*` import scope only for Stage B metadata).

Remaining Stage A closure checklist:

1. Stage A closure complete; continue into Stage B workflow Wasm container design.

Stage B initial scaffold status:

1. Added Stage B AOT contract shape and container metadata hashing/size contract.
2. Added compiler and SDK helpers to construct Stage B envelopes from Stage A + container bytes.
3. Added runtime-native `execute_aot` Stage B branch with container routing marker event (`aot.stage_b.container_routed`).
4. Added boundary validation tests for allowed host imports and execution-time rejection paths.
5. Stage B metadata now carries optional inline workflow bytes (`inline_wasm_hex`) with hash/length validation at contract boundary.
6. Runtime now attempts direct Stage B container invocation via WASIX backend when feature-enabled; falls back to parity path if container execution is unavailable.
7. Added strict Stage B execution option (`strict_stage_b_container_execution`) to reject parity fallback when direct container runtime is unavailable.
8. Conformance now includes strict Stage B fallback rejection tests in runtime + SDK.
9. Next: promote strict mode from optional to default once lowering emits production-valid workflow Wasm.

Start gate (must be true before Track 4 execution):

1. Trace/event schema stability is locked for runtime + SDK outputs.

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
3. Sprint 3: Track 3 hardening (failure injection, split pinning proof, lifecycle contract snapshots).
4. Sprint 4: Track 2 hardening (DB integration/policy/rollback/load tests).
5. Sprint 5-6: AOT pipeline stage A/B and parity tests (after Track 2 gates).

## Out of Scope (v1)

1. Full ORM behavior with implicit schema migration.
2. Distributed module registry service.
3. Cross-language SDKs beyond Rust.
