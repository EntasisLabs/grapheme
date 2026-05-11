# Showcase Programs

These examples stress language/runtime features beyond basic hello-world flows.

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

Run any showcase:

```bash
cargo run -- run examples/showcase/<file>.gr --native-modules
```

Optional step-level trace:

```bash
cargo run -- run examples/showcase/<file>.gr --native-modules --stream-steps
```
