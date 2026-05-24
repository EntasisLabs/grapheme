# Real-World Automation Examples

This folder is the primary outcome-focused example pack.

These programs are not syntax demos. They model practical operational workflows you can run, inspect, and adapt.

## Scenarios

1. `automated-release-control-tower.gr`
- End-to-end release orchestration with validation/build/test/canary/roll/rollback lifecycle.

2. `blue-green-cutover-guarded.gr`
- Blue/green deployment cutover with warmup checks, monitor gate, and automatic rollback.

3. `feature-flag-progressive-rollout.gr`
- Progressive rollout (1/10/50/100) with canary score guardrails.

4. `oncall-escalation-ladder.gr`
- Incident escalation from L1 to manager with acknowledgment/exhaustion branching.

5. `research-decision-loop.gr`
- Web evidence collection and control-loop decisioning to produce citation-ready output.

6. `weekly-sql-kpi-report-alert.gr`
- Weekly SQL KPI digest with anomaly detection and email dispatch.

7. `support-ticket-triage-digest.gr`
- Daily support-ticket routing digest for critical/high/standard response queues.

8. `invoice-intake-approval-digest.gr`
- Accounts-payable intake routing digest with auto-approve/finance-review/needs-PO paths.

9. `typed-support-sla-escalation.gr`
- Strongly typed support SLA lifecycle with explicit transition rules, breach handling, and input override support.

10. `multi-query-web-scrape-digest.gr`
- Multi-query web scraping/research digest that loops topic jobs into per-topic summaries plus an executive digest block.

## Why This Pack Exists

Use these workflows to answer product questions quickly:

- Can Grapheme encode realistic stateful operations cleanly?
- Can teams review and reason about behavior before running it?
- Can governance and runtime controls coexist with fast iteration?
- Can we automate recurring operating reports that teams already run every week?
- Can we automate daily support triage so queue ownership is explicit?
- Can we automate invoice routing decisions with auditable review criteria?
- Can typed workflow contracts make escalation behavior safer to evolve?
- Can one workflow scrape multiple research topics into one digest artifact?

## Run

Run any scenario:

```bash
grapheme run examples/realworld/<file>.gr --json
```

From source workspace:

```bash
cargo run -- run examples/realworld/<file>.gr --json
```

## Explore Success and Failure Paths

Each scenario contains toggleable thresholds and gates (for example score/error/ack values).

Practice both paths:

1. run happy path,
2. force rollback/failure path,
3. compare output contracts and timelines.

For KPI reporting scenario drills:

1. increase anomaly sensitivity threshold in SQL logic,
2. rerun and compare summary line,
3. verify email body remains operator-readable.

That is where Grapheme's production value becomes obvious.
