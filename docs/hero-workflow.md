# Hero Workflow: Research to Action Brief

This is the flagship Grapheme story: gather external signals, shape them into a useful report, and keep the process readable and governable.

## Why This Workflow Matters

Teams often need one reliable flow that can:

- collect external information,
- turn it into structured output,
- remain safe to run in production contexts.

This tutorial demonstrates that in one pass.

## What You Will Run

Primary example:

- examples/websearch-report.gr

Command:

```bash
grapheme run examples/websearch-report.gr --json
```

If you are running from workspace source:

```bash
cargo run -- run examples/websearch-report.gr --json
```

## Success Signals

Look for:

- "outcome": "succeeded"
- structured report-like fields in final state
- repeatable output shape across runs

## What This Demonstrates

1. Intent-first workflow source
The workflow expresses what should happen in a readable sequence.

2. Capability composition
Multiple module capabilities can be orchestrated without collapsing into glue code chaos.

3. Governed execution model
Policy can constrain side effects while preserving workflow source logic.

## If It Fails

Typical stages:

- policy: missing allow-list configuration for side-effecting capabilities
- runtime: provider/network or execution environment problems

Use:

- docs/internal/troubleshooting.md
- docs/internal/runtime-policy.md

## Make It Your Own

After first success:

1. swap the research prompt or scope,
2. keep output shape stable,
3. rerun in JSON mode,
4. compare result consistency.

This gives you a production-like loop: evolve intent, preserve contract.

## Where To Go Next

- language-tour.md for syntax and mental model
- playbooks.md for SQL safety and secrets signing scenarios
- docs/internal/quality/telemetry-reporting-cadence.md for weekly improvement loop
