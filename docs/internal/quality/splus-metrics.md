# S+ Metrics Contract

Status: active
Last updated: 2026-05-24

This document defines the minimum KPI set used to measure progress toward S+ readiness.

## Primary KPIs

1. Time-to-first-success (TTFS)
- Definition: elapsed time from first command to first successful `hello-world` run with `"outcome": "succeeded"`.
- Measurement path: onboarding flow in `docs/getting-started.md`.
- Target:
  - P50 <= 10 minutes
  - P90 <= 20 minutes

2. Docs drift incidents
- Definition: number of CI failures per week caused by docs smoke checks or grammar contract check.
- Measurement path:
  - `scripts/docs-smoke-checks.sh`
  - `scripts/check-grammar-sync.sh`
  - `.github/workflows/conformance.yml`
- Target:
  - <= 1 incident/week in hardening
  - 0 incidents/week at release cutoff

3. Novice failure rate
- Definition: percentage of first-run sessions that fail before a successful run.
- Scope: first-time users executing getting-started path.
- Target:
  - <= 25% in hardening
  - <= 10% at S+ gate

4. Policy-safe default adoption
- Definition: percentage of successful runs that execute under explicit policy allow-lists for side-effecting capabilities.
- Measurement path: environment-variable policy usage in run telemetry.
- Target:
  - >= 70% in hardening
  - >= 90% at S+ gate

## Secondary KPIs

1. Top-10 error concentration
- Definition: share of failures explained by top 10 known remediation patterns in `docs/troubleshooting.md`.
- Target: >= 80% of failures map to known patterns.

2. Example success ratio
- Definition: pass ratio for canonical examples smoke set.
- Target: >= 95% on `main`; 100% on release branch.

3. Docs command validity
- Definition: percent of sampled docs commands that run successfully in CI smoke checks.
- Target: 100% for getting-started commands.

## Instrumentation Plan

1. CI-backed signals (available now)
- Grammar contract pass/fail.
- Getting-started docs smoke pass/fail.
- Conformance test pass/fail.

2. Runtime telemetry signals (next)
- Session start timestamp.
- First successful run timestamp.
- Failure class taxonomy.
- Policy env usage markers.

3. Reporting cadence
- Weekly S+ scorecard in roadmap updates.
- Release gate includes KPI snapshot from previous 14 days.

## S+ Exit Criteria (Metrics)

S+ metrics gate is satisfied when all conditions hold for two consecutive weeks:

1. TTFS P50 <= 10m and P90 <= 20m.
2. Docs drift incidents = 0 in the final week.
3. Novice failure rate <= 10%.
4. Policy-safe default adoption >= 90%.
5. Getting-started docs smoke checks remain green on all `main` merges.
