# 05: Debugging and Operations

Goal: make workflows observable and operable in team environments.

## Useful command modes

```bash
grapheme run examples/realworld/blue-green-cutover-guarded.gr --json
grapheme run examples/realworld/blue-green-cutover-guarded.gr --stream-steps
grapheme run examples/realworld/automated-release-control-tower.gr --json
```

## Stage B AOT checks (0.7.0)

```bash
./scripts/build-aot-container.sh
grapheme build examples/hello-world.gr --aot-stage stage_b --json
grapheme run examples/hello-world.gr --aot-stage stage_b --strict-stage-b --json
```

- Strict mode (default for Stage B): container-first; no silent Stage A fallback.
- `--allow-stage-b-fallback` opts out for debugging.
- `GRAPHEME_PREFER_STAGE_B_WASIX=1` selects the Wasix multi-round sandbox when built with `wasix-runtime`.

## What to inspect

- status fields and timeline markers
- branch decisions
- failure notes
- Stage B `runtime_events` (`aot.stage_b.container_routed`, `aot.stage_b.host_fulfilled`)

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
