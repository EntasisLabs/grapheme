# Executable Kinds v1 Draft

Status: draft
Owner: language + compiler + runtime + lsp
Horizon: 2-3 sprints

Companion concept file: `docs/roadmaps/executable-kinds-v1-concepts.md`

## Why This Exists

Grapheme already treats `query` and `iterator` as meaningful executable declarations, but they are still too close to generic function shape.

This draft makes executable kinds first-class contracts so intent, mutability, and effect boundaries are explicit to both humans and LLMs.

## Product Goals

1. Make mutation intent explicit and verifiable at declaration level.
2. Improve pipeline shape predictability by separating control state from payload data.
3. Enable stronger compile-time checks and better LSP guidance.
4. Prioritize structural language clarity over prototype-era backward compatibility.

## Proposed Kind Model

### Kind: query

Purpose: read/derive pipeline output.

Contract:

1. Defaults to non-mutating behavior.
2. Can call other executables.
3. Can perform external effects if policy allows.
4. Must not perform state/data writes.

### Kind: mutation (new)

Purpose: explicit state/data writes.

Contract:

1. Write-intent executable kind for state changes.
2. Mutation operations must be explicit constructs (`apply state`, `apply data`).
3. Transition and mutation semantics must compose without ambiguity.

### Kind: iterator/node

Purpose: orchestration and control flow.

Contract:

1. Supports `@loop`, branching, and transition-heavy control logic.
2. May delegate to query/mutation kinds.
3. Does not write directly; write operations must be delegated to mutation kinds.

## Mutation Boundary Proposal

Key idea: writes should be explicit at language level, similar to `transition`.

Candidate syntax forms (evaluation set):

1. `update { field: value }`
2. `patch { field: value }`
3. `apply state { field: value }`

Current preference from concept review:

1. `apply` as the primary write form.
2. Lane-targeted writes: `apply state { ... }`, `apply data { ... }`.
3. `apply` is only valid inside `mutation` declarations.

Selection criteria:

1. Clear write intent for humans and LLMs.
2. Easy lowering to MIR.
3. Compatible with state_machine transitions.

## State/Data Convention (Language-Level)

Target runtime shape convention:

1. `$current.state` for control and lifecycle state.
2. `$current.data` for payload.

Lane write guidance:

1. `query` should avoid writes by default under strict profile.
2. `mutation` should be the primary write boundary.
3. `node`/`iterator` write only by calling `mutation` declarations.
4. Root-level writes are disallowed in v1 strict semantics.

## Operation Surface Evolution

Direction:

1. Move primitive operations toward language-native and type-namespaced forms.
2. Reduce reliance on flat `core.*` for common transforms.

Illustrative target surface:

1. Native/intrinsic: `transition`, `apply`, `get`, `has`.
2. Namespaced methods: `string.join`, `string.split`, `array.map`, `object.set_path`.

Rationale:

1. Makes value-domain intent explicit.
2. Improves signature discoverability and LSP completion.
3. Improves post-step shape inference because operation family is typed by namespace.

Compiler/lint behavior:

1. Warn when control fields and payload fields are mixed at root in strict profiles.
2. Prefer helper ops and explicit mutation forms that preserve lane separation.

## Compiler and Runtime Semantics

### Verifier Rule Matrix (v1 baseline)

| Construct / Behavior | query | mutation | node/iterator |
| --- | --- | --- | --- |
| Read/transform pipeline steps | allow | allow | allow |
| External capability calls (policy-gated) | allow | allow | allow |
| `transition` | allow | allow | allow |
| `apply state { ... }` | deny | allow | deny |
| `apply data { ... }` | deny | allow | deny |
| direct root write constructs | deny | deny | deny |
| write-like std ops (bridge period) | warn/error by profile | allow | warn/error by profile |

Profile behavior:

1. Compatibility profile: emit structured lint warnings for illegal placement of write-like behavior.
2. Strict kinds profile: verifier error for illegal placement.
3. v1 target: strict kinds as default for newly authored scripts.

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
2. Define mutation-only write semantics as normative target.

### Phase 1: mutation kind (additive)

1. Add `mutation` declaration parsing and lowering.
2. Enforce `apply` only within `mutation` declarations.

### Phase 2: explicit mutation constructs (additive)

1. Implement selected syntax (`update`/`patch`/`apply`).
2. Lower to existing state patch mechanics.

### Phase 3: strict kind profiles

1. Add opt-in strict mode enforcing non-mutation in query by default.
2. Keep compatibility mode as default until migration stabilizes.

Revised direction:

1. Treat strict kind semantics as baseline for v1 design.
2. Legacy compatibility behavior is optional and explicitly out-of-scope for first implementation pass.

### Phase 4: default hardening

1. Flip strict profile as default for new projects.
2. Keep legacy mode flag for old scripts.

## Migration Strategy

1. Prototype scripts may require rewrite to adopt mutation-only write semantics.
2. Structured lint/errors identify illegal write placement (`apply` outside `mutation`).
3. Codemod path can convert common `core.set_fields` patterns into `mutation` + `apply` forms.

## Acceptance Criteria

1. `mutation` declarations compile and execute.
2. At least two showcase examples are rewritten with explicit mutation boundary.
3. Strict mode catches implicit writes in query/iterator where disallowed.
4. LSP exposes kind contracts and at least one mutation quick-fix.

## Open Questions

1. Should `update`/`patch` exist as aliases if `apply` is the primary keyword?
2. Should `node` ever support scoped local writes, or stay mutation-call-only permanently?
3. Do we split MIR instructions now, or start with call metadata tagging first?
4. Should strict kind enforcement be runtime-profile-dependent or compile-flag-only?
5. How fast should we migrate flat `core.*` transforms to native/namespaced operation surface?
