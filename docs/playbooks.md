# Playbooks

These are practical, outcome-first paths for common Grapheme jobs.

Before running this page, complete `tutorials/README.md` if you want the full end-to-end learning path.

## 0) Flagship Workflow: Research to Action Brief

Start here for the canonical end-to-end story:

- `hero-workflow.md`

## 1) Web Research and Report

Use when you need source-grounded research output.

Run:

```bash
grapheme run examples/websearch-report.gr --json
```

Look for:

- successful outcome marker,
- report-oriented final state,
- stable behavior under repeated runs.

## 2) SQL Transaction Safety

Use when you need controlled read/write workflows with rollback behavior.

Run:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
  grapheme run examples/sql-transaction.gr --json

GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
  grapheme run examples/sql-transaction-rollback.gr --json
```

Look for:

- success markers,
- expected rollback behavior in failure path,
- no unintended persistence side effects.

## 3) Secrets Signing Flow

Use when you need secret-backed signing operations with explicit policy.

Run:

```bash
GRAPHEME_ALLOWED_SECRETS=api_key \
  grapheme run examples/secrets-sign.gr --native-modules --json
```

Look for:

- successful execution,
- no raw secret leakage,
- clear policy-denied errors when allow-lists are missing.

## 4) Capability Discovery Before Authoring

Use when you need to design a workflow from available modules.

Run:

```bash
grapheme modules search routing --explain --detail full --yaml
grapheme modules ops sql
grapheme modules examples websearch
```

Look for:

- module fit by use case,
- operation-level stability cues,
- curated examples that shorten implementation time.

## Playbook Rhythm

For each new use case:

1. discover modules,
2. run nearest example,
3. adapt incrementally,
4. lock policy boundaries,
5. validate output shape in JSON mode.

## Real-World Scenario Pack

For deeper operational workflows, use:

- `examples/realworld/README.md`

This pack includes release control, blue/green cutover, incident escalation, progressive rollout, and research decision loops.
