# 06: Real-World Scenario Lab

Goal: run production-like automation scenarios end to end.

Scenario pack:

- `examples/realworld/automated-release-control-tower.gr`
- `examples/realworld/blue-green-cutover-guarded.gr`
- `examples/realworld/feature-flag-progressive-rollout.gr`
- `examples/realworld/oncall-escalation-ladder.gr`
- `examples/realworld/research-decision-loop.gr`
- `examples/realworld/typed-support-sla-escalation.gr`
- `examples/realworld/multi-query-web-scrape-digest.gr`

## Lab sequence

1. Run each scenario in happy path mode.
2. Force one failure/rollback path by adjusting threshold inputs.
3. Compare status/timeline outputs.
4. Document operational decisions implied by each path.

## Success criteria

You can answer:

- What state transitions are allowed?
- What causes rollback/escalation?
- Which policy controls are required?
- What output shape can downstream systems rely on?
