# 07: Adoption Playbook

Goal: move from successful demos to team-level production adoption.

## Team rollout plan

1. Choose one real workflow with clear business owner.
2. Start from nearest `examples/realworld` scenario.
3. Define explicit output contract in JSON mode.
4. Gate side effects with policy allow-lists.
5. Add weekly telemetry review and docs updates.

## Governance checklist

- Named owner for each workflow
- Expected status/timeline states documented
- Rollback/escalation conditions defined
- Policy environment requirements documented
- Troubleshooting mapping for top errors

## Practical anti-patterns

Avoid:

- giant single executable doing unrelated jobs
- hidden assumptions on side effects
- no failure-path drill before production handoff

Prefer:

- explicit state transitions
- small composable steps
- documented guardrails and rollout conditions

## Done state

Adoption is healthy when:

- one workflow is running reliably in production-like conditions,
- failure paths were tested intentionally,
- operators and authors share the same workflow understanding.
