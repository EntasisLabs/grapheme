# Telemetry Export Schema (v1)

Status: draft contract  
Last updated: 2026-05-24

## Purpose

Define a stable, shareable schema for `grapheme telemetry export` output so users can safely report usage outcomes without leaking local file paths.

## Command Surface

```bash
grapheme telemetry export --json
grapheme telemetry export --yaml
grapheme telemetry export --out /tmp/grapheme-report.json --json
```

Default output paths:

- JSON: `.grapheme/telemetry/report.json`
- YAML: `.grapheme/telemetry/report.yaml`

## Top-Level Shape

```json
{
  "generated_at_ms": 1779645409595,
  "source_path": "/tmp/grapheme-telemetry-e2e.jsonl",
  "summary": { "...": "see summary contract" },
  "events": [
    {
      "event": "command_result",
      "timestamp_ms": 1779645407809,
      "command": "run",
      "success": true,
      "duration_ms": 5,
      "error_class": null,
      "run_target": "examples/hello-world.gr",
      "funnel_stage": null
    }
  ]
}
```

## Summary Contract

`summary` includes aggregate KPI fields:

- `path`: source events file path used for aggregation
- `event_count`: all parseable events read
- `command_result_count`: number of `command_result` events
- `success_count`: successful command results
- `failure_count`: failed command results
- `success_rate`: `success_count / command_result_count`
- `avg_duration_ms`: average command duration where present
- `first_success_count`: count of `first_success` events
- `ttfs_start_count`: count of onboarding funnel start events (`ttfs_start`)
- `ttfs_success_count`: count of onboarding funnel success events (`ttfs_success`)
- `ttfs_failure_count`: count of onboarding funnel failure events (`ttfs_failure`)
- `ttfs_success_rate`: `ttfs_success_count / ttfs_start_count`
- `command_counts`: map of `command -> count` across all events
- `failure_stage_counts`: map of onboarding failure stage counts
- `top_error_classes`: sorted top error classes for failed command results

## Event Contract

Each event in `events` includes:

- `event`: event name
  - known values: `session_start`, `command_result`, `first_success`, `ttfs_start`, `ttfs_success`, `ttfs_failure`
- `timestamp_ms`: unix epoch milliseconds
- `command`: root CLI command (`run`, `parse`, `help`, etc.)
- `success`: optional boolean outcome
- `duration_ms`: optional elapsed milliseconds
- `error_class`: optional coarse category (`parse_error`, `policy_denied`, `runtime_error`, `invalid_args`, `invalid_command`, `other`)
- `run_target`: optional run target path (redacted by export rules)
- `funnel_stage`: optional onboarding stage label (`run`, `parse`, `runtime`, `policy`, `args`, `command`, `other`)

## Redaction Rules

`telemetry export` applies redaction to `run_target`:

- Preserve targets under `examples/...`
- Redact non-example paths as `<redacted>/<file-name>`
- If file name cannot be resolved, fallback to `<redacted>/target.gr`

## Compatibility Guidance

- Additive fields are allowed in minor revisions.
- Existing fields should not be removed or renamed without a version bump.
- Consumers should ignore unknown fields for forward compatibility.