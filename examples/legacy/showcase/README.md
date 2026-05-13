# Showcase Programs

These examples stress language/runtime features beyond basic hello-world flows.

They are designed for live demos of realistic AI workflow orchestration in compact `.gr` programs.

## Files

- `fibonacci-threshold-loop.gr`
  - Iterative Fibonacci progression with arithmetic state transitions and branch-based early return by index.
- `fibonacci-threshold-typed.gr`
  - Same Fibonacci threshold flow with first-class `struct` and typed executable signatures.
- `fibonacci-threshold-namespaced.gr`
  - Uses `import types` and `Namespace::Type` signatures/initializers for cross-file struct reuse.
- `types-domain.gr`
  - Shared type declarations for namespaced examples.
- `types-ops-domain.gr`
  - Shared domain types plus state-machine lifecycle contract for operational demos.
- `job-coordinator-lifecycle.gr`
  - Typed namespaced long-running job coordinator with lifecycle state-machine checks, retry/timeout directives, escalation, and completion paths.
- `types-release-domain.gr`
  - Deployment/release domain enum, state_machine transitions, and typed state model for control-plane orchestration.
- `release-control-tower.gr`
  - Complex deployment control-flow graph with namespaced types, nested `match` lifecycle routing, fragment-based transition extraction, canary/rollback branches, and runtime retry/timeout hooks.
- `release-control-tower-compact.gr`
  - Token-optimized variant of release orchestration using shorter identifiers plus `set`, `transition`, `@r/@t`, and `@core_default` sugar.
- `blue-green-cutover.gr`
  - Blue/green release flow with warmup checks, traffic shift, monitoring gate, and automatic rollback path.
- `oncall-escalation-ladder.gr`
  - Incident escalation demo from L1 to manager with acknowledgment/exhaustion branching and compact control-flow targets.
- `feature-flag-progressive-rollout.gr`
  - Progressive feature-flag rollout (1/10/50/100) with canary-score rollback guards.
- `poll-until-ready.gr`
  - Loop-until control flow with branch handlers, state mutation, and `@core_default` shorthand for bare core ops.
- `queue-triage-each.gr`
  - `@loop(each)` over object arrays, multi-case `match` dispatch, and append merge mode.
- `transform-router.gr`
  - Native transform chain (`yaml.to_json` -> `json.parse`) plus control-flow routing.

## Suggested Live Demo Order

1. `release-control-tower-compact.gr` (full language ergonomics pass)
2. `blue-green-cutover.gr` (deployment gate + rollback)
3. `feature-flag-progressive-rollout.gr` (progressive rollout narrative)
4. `oncall-escalation-ladder.gr` (incident escalation narrative)

This four-file set is the canonical showcase path for presentations and onboarding.

## Extended Showcase Files

These remain valuable, but are not part of the primary demo path:

- `release-control-tower.gr` (verbose counterpart to compact control tower)
- `job-coordinator-lifecycle.gr`
- `poll-until-ready.gr`
- `queue-triage-each.gr`
- `transform-router.gr`
- `fibonacci-threshold-loop.gr`
- `fibonacci-threshold-typed.gr`
- `fibonacci-threshold-namespaced.gr`

## Quick Scenario Toggles

- Blue/green rollback path:
  - In `blue-green-cutover.gr`, set `error_threshold` below `error_rate` in the query initializer.
- Feature-flag rollback path:
  - In `feature-flag-progressive-rollout.gr`, set `canary_score` below `min_score` in the query initializer.
- On-call exhaustion path:
  - In `oncall-escalation-ladder.gr`, set `ack_at` above the manager wait window.

Run any showcase:

```bash
cargo run -- run examples/legacy/showcase/<file>.gr --native-modules
```

Optional step-level trace:

```bash
cargo run -- run examples/legacy/showcase/<file>.gr --native-modules --stream-steps
```
