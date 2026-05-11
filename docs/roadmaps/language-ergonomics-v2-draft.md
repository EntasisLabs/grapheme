# Grapheme Lang Ergonomics v2 Draft

Status: draft
Owner: language + runtime + lsp
Horizon: 3-4 sprints

## Why This Exists

Grapheme Lang is now reliable for complex orchestration, but authoring large workflows is still verbose. This roadmap targets a major ergonomics lift for both human and LLM code generation while preserving deterministic runtime and policy safety.

## Product Goals

1. Reduce orchestration boilerplate by at least 30 percent in common workflows.
2. Increase compile-time guidance for lifecycle correctness.
3. Improve LSP assist quality for branch-heavy and typed workflows.
4. Preserve current runtime determinism and policy controls.

## Feature Set (Do It All)

### 1) State Machines (first-class)

Add declarative lifecycle specs with allowed transitions.

Sketch:

```grapheme
state_machine JobLifecycle {
  state queued
  state running
  state blocked
  state done terminal
  state timeout terminal

  transition queued -> running
  transition running -> blocked
  transition blocked -> running
  transition running -> done
  transition running -> timeout
}
```

Benefits:

- Removes repeated status branch checks.
- Enables verifier checks for illegal transitions.
- Improves completion and quick-fix suggestions.

### 2) Local Bindings in Pipelines

Add ephemeral local values that do not mutate current state.

Sketch:

```grapheme
let nextAttempt = $current.attempt + 1
```

Benefits:

- Reduces temporary field churn in state.
- Improves readability of intermediate calculations.

### 3) Compact Conditional Syntax (sugar over flow.branch)

Add readable if/else sugar lowered to current branch instruction.

Sketch:

```grapheme
if $current.attempt >= $current.max_attempts then MarkTimeout else EvaluateSignal
```

Benefits:

- Shorter branch expression for common binary routing.
- Lower parse and generation friction in large graphs.

### 4) Enums and Tagged Unions

Add typed lifecycle/status domains.

Sketch:

```grapheme
enum JobStatus { queued, running, blocked, done, timeout }
```

Benefits:

- Eliminates stringly-typed status drift.
- Enables exhaustive branch verification.

### 5) Built-in Retry and Timeout Policies

Add high-level retry/timeout directives lowered to existing runtime model.

Sketch:

```grapheme
@retry(max: 5, backoff_ms: 200, on_fail: Escalate)
@timeout(ms: 30000, on_timeout: MarkTimeout)
```

Benefits:

- Encodes common orchestration patterns directly.
- Reduces repeated iterator scaffolding.

### 6) Exhaustiveness and Reachability Checks

Verifier additions:

- Missing terminal branch detection.
- Unreachable iterator path detection.
- Non-exhaustive enum branch diagnostics.

### 7) LSP Quick Actions and Scaffolds

Code actions:

- Generate missing branch handlers.
- Generate state transition skeletons.
- Extract selected pipeline into iterator.
- Fill missing required output fields.

### 8) Capability Schema Registry

Machine-readable operation schemas consumed by compiler + lsp.

Benefits:

- Better arg validation and signature help.
- Better completion for chained transforms and core ops.

### 9) Scenario and Profile Test Packs

Add profile matrix runner for bounded/unbounded, timeout, failure, and retry paths.

Benefits:

- Faster regression confidence for orchestration behaviors.

## Architecture Notes

1. New syntax should lower to current MIR primitives when possible.
2. Runtime should remain policy-first and deterministic.
3. Incremental rollout should keep backward compatibility across all existing .gr examples.

## Risks

1. Syntax expansion can increase parser ambiguity.
2. State-machine and enum checks may require richer CFG representation.
3. Retry and timeout sugar can hide complexity unless trace metadata remains explicit.

## Definition of Done (v2)

1. At least three production-grade showcase workflows rewritten with less boilerplate.
2. Verifier catches illegal transitions and non-exhaustive typed branches.
3. LSP quick actions available for lifecycle-heavy files.
4. Policy profile tests cover success, retry, timeout, and bounded-failure modes.

## Proposed Sprint Sequence

1. Sprint 1: compact conditional syntax + enum foundations + lsp quick-action scaffold.
2. Sprint 2: state-machine declarations + transition verifier.
3. Sprint 3: retry/timeout sugar + capability schema registry integration.
4. Sprint 4: exhaustive checks + scenario profile packs + docs hardening.

## Iterator and Fragment Rollout (Additive)

Goal:

- Keep `iterator` as the runtime primitive.
- Introduce `fragment` as an inline composition primitive.
- Reduce verbosity without changing runtime determinism.

Scope split:

- `iterator`: directives, runtime boundaries, callable target.
- `fragment`: no directives, inline-expanded, non-callable at runtime.

Migration-safe phases:

1. Phase A: parser + AST support for `fragment` (no behavior change to existing iterator code).
2. Phase B: HIR inliner for fragment expansion with source-span tracking.
3. Phase C: verifier constraints (no directives on fragments, cycle detection in fragment expansion).
4. Phase D: LSP support (completion/hover/definition for fragment usage and expansion diagnostics).
5. Phase E: optional codemod to convert directive-free helper iterators into fragments.

Acceptance criteria:

1. Existing `.gr` examples compile and run unchanged.
2. At least two showcase workflows drop boilerplate by 20 percent or more after fragment adoption.
3. State-machine transition and typed output checks remain identical before and after fragment expansion.
4. Runtime traces remain stable for iterator boundaries; fragments do not create new runtime frames.

Verbosity reduction backlog after fragment:

1. `match` multi-value case groups (`case planned, validating => Prepare`).
2. State patch sugar for common lifecycle updates.
3. LSP code action: "extract selected steps into fragment".
