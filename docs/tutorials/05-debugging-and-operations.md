# 05: Debugging and Operations

Goal: make workflows observable and operable in team environments.

## Useful command modes

```bash
grapheme run examples/realworld/blue-green-cutover-guarded.gr --json
grapheme run examples/realworld/blue-green-cutover-guarded.gr --stream-steps
grapheme run examples/realworld/automated-release-control-tower.gr --json
```

## What to inspect

- status fields and timeline markers
- branch decisions
- failure notes

## Telemetry and improvement loop

```bash
grapheme telemetry summarize --json
grapheme telemetry export --json
```

Aggregate weekly reports:

```bash
scripts/telemetry-weekly-rollup.sh .grapheme/telemetry/reports/2026-W21 --out /tmp/week.md
```

Use weekly results to improve docs and scenario defaults.
