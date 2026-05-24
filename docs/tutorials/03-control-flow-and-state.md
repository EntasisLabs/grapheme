# 03: Control Flow and Stateful Lifecycles

Goal: model long-running and branching operations without losing clarity.

## Recommended examples

```bash
grapheme run examples/resilience-composition.gr --json
grapheme run examples/realworld/feature-flag-progressive-rollout.gr --json
grapheme run examples/realworld/oncall-escalation-ladder.gr --json
```

## Patterns

- loop with bounded max
- gate conditions for promotion/rollback
- explicit terminal states

## Exercise

For feature flag rollout:

1. lower canary score below threshold,
2. rerun,
3. compare `done` vs `rolled_back` paths.

This teaches safe progression and failure semantics.
