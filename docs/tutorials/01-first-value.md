# 01: First Value in 10 Minutes

Goal: prove Grapheme value fast by running one useful workflow and inspecting structured output.

## Step 1: Run the flagship workflow

```bash
grapheme run examples/websearch-report.gr --json
```

From source workspace:

```bash
cargo run -- run examples/websearch-report.gr --json
```

## Step 2: Confirm outcome contract

Success signals:

- `"outcome": "succeeded"`
- report-like content in final state

## Step 3: Change one input, keep output shape

Edit query/topic in the example and run again.

Objective:

- prove intent is easy to evolve,
- output contract stays stable.

## Why this matters

This is the core Grapheme promise:

- explicit workflow intent,
- governed execution model,
- predictable machine-readable outputs.
