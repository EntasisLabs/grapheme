# Executable Kinds v1 Draft

Status: draft
Owner: language + compiler + runtime + lsp
Horizon: 2-3 sprints

## Why This Exists

Grapheme already treats `query` and `iterator` as meaningful executable declarations, but they are still too close to generic function shape.

This draft makes executable kinds first-class contracts so intent, mutability, and effect boundaries are explicit to both humans and LLMs.

## Product Goals

1. Make mutation intent explicit and verifiable at declaration level.
2. Improve pipeline shape predictability by separating control state from payload data.
3. Enable stronger compile-time checks and better LSP guidance.
4. Preserve backward compatibility with existing `.gr` programs during rollout.

## Proposed Kind Model

### Kind: query

Purpose: read/derive pipeline output.

Contract:

1. Defaults to non-mutating behavior.
2. Can call other executables.
3. Can perform external effects if policy allows.
4. Must not perform implicit state writes in strict mode.

### Kind: mutation (new)

Purpose: explicit state/data writes.

Contract:

1. Write-intent executable kind for state changes.
2. Mutation operations must be explicit constructs (no accidental write-by-op shape changes).
3. Transition and mutation semantics must compose without ambiguity.

### Kind: iterator/node

Purpose: orchestration and control flow.

Contract:

1. Supports `@loop`, branching, and transition-heavy control logic.
2. May delegate to query/mutation kinds.
3. In strict mode, writes require explicit mutation call targets or mutation constructs.

## Mutation Boundary Proposal

Key idea: writes should be explicit at language level, similar to `transition`.

Candidate syntax forms (evaluation set):

1. `update { field: value }`
2. `patch { field: value }`
3. `apply state { field: value }`

Selection criteria:

1. Clear write intent for humans and LLMs.
2. Easy lowering to MIR.
3. Compatible with state_machine transitions.

## State/Data Convention (Language-Level)

Target runtime shape convention:

1. `$current.state` for control and lifecycle state.
2. `$current.data` for payload.

Compiler/lint behavior:

1. Warn when control fields and payload fields are mixed at root in strict profiles.
2. Prefer helper ops and explicit mutation forms that preserve lane separation.

## Compiler and Runtime Semantics

### Compiler

1. Add executable-kind policy table.
2. Enforce write permissions by kind.
3. Enforce explicit mutation boundary in strict mode.
4. Emit structured lint warnings for implicit mutation/clobber patterns.

### Runtime/MIR

1. Introduce distinct MIR category for mutation-like operations (or tagged call metadata if instruction split is deferred).
2. Keep transition instructions separate from generic calls.
3. Preserve deterministic execution and policy checks.

### LSP

1. Show kind contract in hover/signature help.
2. Offer quick-fix to convert implicit write patterns into explicit mutation forms.
3. Warn when query kind performs write-like steps under strict profile.

## Rollout Plan

### Phase 0: Baseline and docs

1. Publish kind contract and mutation-boundary design.
2. Keep behavior additive and warning-only.

### Phase 1: mutation kind (additive)

1. Add `mutation` declaration parsing and lowering.
2. No strict enforcement yet.

### Phase 2: explicit mutation constructs (additive)

1. Implement selected syntax (`update`/`patch`/`apply`).
2. Lower to existing state patch mechanics.

### Phase 3: strict kind profiles

1. Add opt-in strict mode enforcing non-mutation in query by default.
2. Keep compatibility mode as default until migration stabilizes.

### Phase 4: default hardening

1. Flip strict profile as default for new projects.
2. Keep legacy mode flag for old scripts.

## Migration Strategy

1. Existing scripts continue to run unchanged in compatibility mode.
2. Structured lint warnings identify implicit mutation and shape clobber locations.
3. Codemod path can convert common `core.set_fields` patterns to explicit mutation forms.

## Acceptance Criteria

1. `mutation` declarations compile and execute.
2. At least two showcase examples are rewritten with explicit mutation boundary.
3. Strict mode catches implicit writes in query/iterator where disallowed.
4. LSP exposes kind contracts and at least one mutation quick-fix.

## Open Questions

1. Best mutation keyword: `update`, `patch`, or `apply`?
2. Should iterator allow local writes by default, or only via mutation call targets?
3. Do we split MIR instructions now, or start with call metadata tagging first?
4. Should strict kind enforcement be runtime-profile-dependent or compile-flag-only?
