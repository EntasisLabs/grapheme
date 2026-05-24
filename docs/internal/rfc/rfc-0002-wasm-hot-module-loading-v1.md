# RFC-0002: Wasm Hot Module Loading v1

Status: draft
Authors: runtime
Created: 2026-05-12
Target release window: next 1-2 sprints after SDK extraction

## Summary

Introduce safe hot module loading for Wasm capability modules with versioned generations, compatibility checks, in-flight execution pinning, and rollback support.

This enables CLR/DLL-like runtime behavior while preserving Grapheme policy safety and deterministic execution.

## Motivation

Current strengths:

1. Capability module manifests and runtime policy model exist.
2. Wasm plugin path already integrated.
3. Signature/manifest/registry conformance checks already in place.

Current gaps:

1. Module replacement requires restart semantics.
2. No formal activation lifecycle.
3. No safe in-flight pinning model.

Desired outcome:

- Runtime can load new module versions without interrupting active workflows.

## Goals

1. Support loading a new module version at runtime.
2. Keep in-flight executions bound to previously resolved module generations.
3. Activate new generations only after compatibility and policy checks pass.
4. Provide rollback to prior generation after failed activation.
5. Emit lifecycle events for observability and audit.

## Non-Goals

1. Distributed module registry service.
2. Automatic semantic migration of op contracts.
3. Cross-process shared module cache in v1.

## Concepts

### Module Generation

A module generation is an immutable loaded instance identified by:

1. `module_id` (for example `http`)
2. `version` (semver from manifest)
3. `content_hash` (wasm bytes hash)
4. `generation_id` (runtime monotonic id)

### Activation Pointer

Each module id has one active generation pointer used for new executions.

### Execution Pinning

When a workflow starts, module resolution produces a per-execution binding table. That table is immutable for the execution lifetime.

Result:

- New activations do not mutate already-running executions.

## Proposed Runtime Model

### Module Manager

Add a module manager component with responsibilities:

1. Load and validate candidate generations.
2. Perform compatibility checks.
3. Run activation transaction.
4. Track references for in-flight pinning.
5. Retire drained generations.

### Lifecycle States

1. `Loaded` (bytes loaded, not active)
2. `Validated` (manifest/compat/policy checks passed)
3. `Active` (selected for new executions)
4. `Draining` (superseded but referenced by in-flight executions)
5. `Retired` (no references)
6. `Failed` (activation or runtime guard failure)

### Activation Transaction

Activation steps:

1. Load candidate bytes and parse manifest.
2. Verify ABI compatibility.
3. Verify signature compatibility for exported ops.
4. Verify required capability policy admission.
5. Set active pointer to candidate generation.
6. Mark previous active generation as `Draining`.
7. Emit activation event.

On failure at any step:

1. Candidate marked `Failed`.
2. Active pointer unchanged.
3. Failure event emitted.

## Compatibility Rules (v1)

Hard requirements for activation:

1. `module_id` must match target slot.
2. ABI kind must match expected runtime ABI.
3. Exported op names required by current signature scope must exist.
4. Existing required op schemas must be backward compatible.
5. Required capability set must not exceed policy allowlist.

Compatibility modes:

1. `Strict` default:
- No required op removal.
- No incompatible input/output schema changes for existing ops.

2. `Permissive` optional for local dev:
- Allows additive ops.
- Still rejects removal/incompatible changes for existing required ops.

## Rollback

Rollback behavior:

1. Keep previous active generation metadata until candidate proven active.
2. On runtime health check failure within probation window, revert active pointer to prior generation.
3. Candidate transitions to `Failed` and is excluded from auto-reactivation.

## Cache and Artifact Interaction

1. Existing artifact execution pins resolved generation ids at start.
2. New executions resolve against current active pointer.
3. Artifact-level cache keys include module generation fingerprints.

## Policy and Security

1. Activation requires policy admission using required capabilities in manifest.
2. Module hash must be recorded in trace/audit stream.
3. Optional signed manifest verification may be introduced in v1.1.

## Observability

Emit runtime events:

1. `module.loaded`
2. `module.validated`
3. `module.activated`
4. `module.activation_failed`
5. `module.draining`
6. `module.retired`
7. `module.rollback`

Each event includes:

1. `module_id`
2. `version`
3. `generation_id`
4. `content_hash`
5. Reason fields for failures/rollback

## API Sketch (draft)

```rust
pub enum CompatibilityMode {
    Strict,
    Permissive,
}

pub struct LoadModuleRequest {
    pub module_id: String,
    pub wasm_path: std::path::PathBuf,
    pub compatibility_mode: CompatibilityMode,
}

pub struct ActivationResult {
    pub module_id: String,
    pub generation_id: u64,
    pub version: String,
    pub content_hash: String,
}

pub trait ModuleManager {
    fn load_and_activate(&self, req: LoadModuleRequest) -> Result<ActivationResult, ModuleLoadError>;
    fn rollback(&self, module_id: &str) -> Result<(), ModuleLoadError>;
    fn active_generation(&self, module_id: &str) -> Option<u64>;
}
```

## Implementation Plan

### Phase 1: Scaffolding

1. Add module manager data model and generation registry.
2. Add lifecycle events and tracing hooks.
3. Add execution pinning table at runtime start.

### Phase 2: Compatibility and Activation

1. Implement strict compatibility validator.
2. Implement activation transaction and draining state.
3. Implement rollback path.

### Phase 3: Hardening

1. Add conformance tests for compatibility edge cases.
2. Add failure-injection tests for rollback and draining.
3. Add benchmark for activation overhead and execution impact.

## Testing Strategy

1. Unit tests:
- Compatibility checks.
- State transitions.

2. Integration tests:
- Activate while execution in flight.
- Validate pinning behavior.
- Rollback on activation/runtime failure.

3. Contract tests:
- Event schema snapshots.
- Manifest/signature conformance under activation.

## Open Questions

1. Should rollback be automatic by default or policy-configurable?
2. Should probation window be wall-clock or execution-count based?
3. Which compatibility metadata belongs in signatures vs manifests?

## Acceptance Criteria

1. Runtime can activate new module generation without stopping process.
2. In-flight workflows continue using pinned generations.
3. Incompatible module update is rejected with clear diagnostics.
4. Rollback path restores prior active generation deterministically.
5. Lifecycle events are emitted and queryable in traces.
