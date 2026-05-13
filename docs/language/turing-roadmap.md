# Turing Completeness Roadmap

Status: draft for private development milestones.

## Goal

Reach a language/runtime contract that is:

1. Semantically unbounded at the language level.
2. Operationally safe through runtime policy limits.
3. Verifiable through conformance programs.

## Current Baseline

Implemented today:

- Iterator loops with `@loop(...)`.
- Iterator calls (explicit `call` and bare iterator-invocation sugar).
- Conditional branch dispatch through `flow.branch` lowering.
- Recursive directive sugar via `@recursive(max_depth: N)`.
- State threading with `$current` interpolation.

Current bounds:

- Runtime default step budget.
- Optional loop max in syntax.
- Call-depth guardrails.

## Target Contract

Language semantics:

- `@loop` can be unbounded (no required max).
- Recursion can be expressed without mandatory compile-time depth bound.
- State model remains the abstract machine memory model.

Runtime policy:

- Step budget (`max_steps`) controls termination in practice.
- Time/memory/call-depth limits remain runtime controls, not language requirements.
- Exceeding a budget must return deterministic structured runtime failure.

## Sprint Plan

### Sprint 1 (now)

- Make `@loop max` optional.
- Introduce runtime step-budget enforcement.
- Expose step-budget policy in CLI/runtime docs.
- Add examples demonstrating collection-driven loops and recursive branching.

### Sprint 2

- Make recursive directive max depth optional.
- Move recursion depth constraints fully to runtime policy.
- Add runtime policy knob for default call-depth budget.

### Sprint 3

- Add minimal arithmetic/comparison primitives for machine-style programs.
- Add canonical while-like control-flow examples.

Current status:

- Implemented core arithmetic/comparison primitives: `add`, `sub`, `inc`, `dec`, `eq`, `lt`, `gt`.
- Added while-like recursive counter example: `examples/fixtures/v1-while-counter.gr`.

### Sprint 4

- Add conformance programs (counter machine / small evaluator).
- Record expected outcomes under bounded and unbounded policy profiles.

Current status:

- Added while-like counter conformance program: `examples/fixtures/v1-while-counter.gr`.
- Added partial-function divergence program: `examples/fixtures/v1-partial-diverge.gr`.
- Extended private policy matrix and one-command checker to cover bounded and unbounded profiles for both termination and deterministic budget failure.
- Added first 2-counter Minsky-style transfer program: `examples/fixtures/v1-minsky-transfer.gr` with bounded/unbounded profile checks.
- Added second branching 2-counter Minsky pattern: `examples/fixtures/v1-minsky-branching.gr` with bounded/unbounded profile checks.
- Added native data-transformation modules for agent workflows: `html.to_md`, `json.parse`, `csv.to_list`, `yaml.to_json`.

## Conformance Gate

Declare practical Turing-complete readiness when all are true:

1. Language permits unbounded loops/recursion forms.
2. Runtime policy can bound or unbound execution intentionally.
3. Counter-machine style program executes correctly under unbounded profile.
4. Same program fails predictably under bounded profile with budget error.

Current private baseline matrix:

- `docs/runtime/policy-profile-matrix.md`
- `scripts/policy-profile-checks.sh`

### Sprint 5 (typed programming ergonomics)

1. Add first-class typed record declarations (`struct`).
2. Add typed executable signatures (`on InputType -> OutputType`).
3. Add verifier checks for field existence and call arg compatibility.
4. Add LSP inferred-shape hover and field completion in typed scopes.

Design draft:

- `docs/language/typed-records-v1.md`
