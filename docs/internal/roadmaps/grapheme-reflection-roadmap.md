# Grapheme Reflection + Hotmodule + LSP SDK Roadmap

Status: completed
Last updated: 2026-05-24
Session: grapheme-reflection

## Progress Snapshot

1. Phase 1: completed
2. Phase 2: completed
3. Phase 3: completed
4. Phase 4: completed
5. Phase 5: completed
6. Phase 6: completed

## Objective

Add first-class SDK support for:

1. Module reflection and function/executable reflection.
2. Wasm hotmodule load and rollback controls.
3. Embeddable/programmatic LSP usage through an LSP SDK surface.

## Current Baseline

### Already implemented

1. SDK module reflection/discovery payloads are present.
2. Runtime hotmodule lifecycle core is present:
- Generation activation
- Rollback
- Lifecycle events
- Generation metadata in registry
- In-flight execution pinning via cloned module registry
3. Compiler/artifact metadata required for function reflection is present:
- HIR executable definitions and signatures
- MIR function list/kinds
4. LSP server exists as a binary and VS Code extension integration exists.

### Missing first-class surfaces

1. SDK function/executable reflection API.
2. SDK hotmodule activation/rollback/event API.
3. Dedicated LSP SDK/library boundary for embedding and programmatic startup.

## Guiding Constraints

1. Preserve existing behavior and APIs where possible.
2. Keep payload contracts stable and machine-friendly.
3. Validate compatibility and policy checks before activation paths.
4. Keep execution pinning semantics deterministic across concurrent runs.

## Phase Plan

## Phase 1: Reflection Contract Freeze

Goal: lock public SDK reflection data contracts before broader implementation.

Deliverables:

1. Public DTOs for module and function reflection payloads.
2. Serde-stable output schema and rustdoc.
3. Golden payload snapshots.

Acceptance criteria:

1. Payload shape is explicit and version-safe.
2. Tests verify stable serialization and key ordering assumptions where relevant.

## Phase 2: Function Reflection in SDK

Goal: provide first-class executable/function reflection over source and compiled artifacts.

Deliverables:

1. SDK APIs to list/query executable metadata.
2. Source-backed reflection path using compiler outputs.
3. Artifact-backed reflection path using MIR metadata.

Acceptance criteria:

1. Query/mutation/subscription/iterator kinds are reflected.
2. Input/output signature metadata and entrypoint metadata are exposed.
3. Unknown symbol/entrypoint handling is explicit and tested.

## Phase 3: Stateful SDK Runtime Session + Hotmodule Controls

Goal: expose runtime hotmodule lifecycle to embedders via SDK.

Deliverables:

1. Stateful SDK session type (persistent runtime state).
2. SDK methods for activation, rollback, lifecycle event retrieval.
3. SDK error mapping for module load/compat/policy failures.

Acceptance criteria:

1. SDK can activate a module generation and execute with updated bindings.
2. SDK rollback restores prior generation semantics.
3. Lifecycle events are accessible from SDK APIs.

## Phase 4: Hotmodule Hardening and Concurrency Tests

Goal: prove safety semantics under failure and concurrency.

Deliverables:

1. Integration tests for execution pinning across activation boundaries.
2. Failure-injection tests for incompatible ABI/policy denial.
3. Snapshot contract coverage for lifecycle events.

Acceptance criteria:

1. In-flight runs remain pinned while post-activation runs use new generation.
2. Failed activation never corrupts active generation pointer.
3. Rollback behavior is deterministic and repeatable.

## Phase 5: LSP SDK Extraction

Goal: provide an embeddable LSP library surface while preserving existing binary behavior.

Deliverables:

1. Extract reusable server construction from lsp main binary into library APIs.
2. Maintain thin binary wrapper for current editor tooling.
3. Add a minimal programmatic startup entrypoint for embedding use cases.

Acceptance criteria:

1. Existing CLI/editor integration remains unchanged.
2. New library API can be used without shelling out to a binary.
3. Build/test validates both binary and library paths.

## Phase 6: Docs, Examples, and Release Alignment

Goal: finish rollout with operational clarity and adoption support.

Deliverables:

1. SDK docs for reflection and hotmodule workflows.
2. LSP SDK usage doc with one minimal embedding example.
3. Release notes and roadmap updates across docs.

Acceptance criteria:

1. Docs reflect shipped behavior exactly.
2. Example code compiles and runs in CI.
3. No unresolved contract drift between docs and implementation.

## Work Breakdown (Suggested Iteration Order)

1. Phase 1 then Phase 2.
2. Phase 3 then Phase 4.
3. Phase 5.
4. Phase 6 continuously with completion pass at end.

## Risks and Mitigations

1. Risk: API churn during early phase.
Mitigation: freeze DTO contracts in Phase 1 with snapshot tests.

2. Risk: subtle hotmodule regressions under concurrent execution.
Mitigation: dedicated pinning/failure/rollback integration tests in Phase 4.

3. Risk: LSP extraction impacts existing editor path.
Mitigation: preserve binary wrapper and add parity tests for startup behavior.

## Definition of Done

1. SDK exposes first-class module + function reflection.
2. SDK exposes first-class hotmodule activation/rollback/events.
3. LSP SDK library path exists and is documented.
4. Conformance and regression tests are green.
5. Docs and examples match production behavior.
