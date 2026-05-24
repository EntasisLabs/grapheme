# 08: Build-Along - Weekly SQL KPI Report + Anomaly Summary + Email

Goal: build a workflow your team could actually run every week, not a toy syntax demo.

You will implement one practical automation end to end:

1. gather KPI rows from SQL,
2. compute anomaly flags,
3. generate a human-readable digest,
4. send the digest by email.

## Why This Is A Good Grapheme Use Case

Most teams already do this manually:

- run a SQL report,
- scan for outliers,
- summarize for ops/product,
- send a weekly update.

Grapheme turns that into one explicit, reviewable workflow source.

## Business Requirements

Your workflow should:

1. produce a weekly report line per day,
2. flag anomalies using clear thresholds,
3. include a summary line with anomaly count and averages,
4. send one email payload that is readable by humans,
5. remain deterministic for local practice runs.

## Final Workflow File

Reference implementation:

- `examples/realworld/weekly-sql-kpi-report-alert.gr`

## Step 1: Build A Deterministic SQL Dataset

Use an inline SQL CTE (`values (...)`) so the tutorial works without external DB setup.

Why:

- every learner gets the same output,
- anomaly behavior is reproducible,
- no hidden infrastructure dependency.

## Step 2: Encode Anomaly Rules In Query Logic

In the SQL layer, flag each row when KPI conditions cross thresholds.

Current rule used in the example:

- anomaly when `churn_rate > 0.050` OR `mrr < 11000`

Why this matters:

- explicit rules are auditable,
- policy and business thresholds stay easy to review.

## Step 3: Generate Operator-Readable Lines

Use SQL `printf(...)` to pre-format per-day report lines and one summary line.

Then in Grapheme:

1. extract `rows`,
2. map `line`,
3. join lines into one email body.

This keeps payload shaping explicit in workflow source.

## Step 4: Send Email Digest

Send one digest email using `smtp.send_mail` with:

- `to`
- `subject`
- `body`

Workflow source already includes this final step.

## Run It

CLI path:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
GRAPHEME_ALLOWED_SMTP_DOMAINS=example.com \
grapheme run examples/realworld/weekly-sql-kpi-report-alert.gr --json
```

Workspace path:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
GRAPHEME_ALLOWED_SMTP_DOMAINS=example.com \
cargo run -- run examples/realworld/weekly-sql-kpi-report-alert.gr --json
```

## Expected Output Signals

Look for:

- `"outcome": "succeeded"`
- SMTP result object with acceptance state
- email body lines ending with a `SUMMARY | ...` row

## Failure Drills (Required)

Run these intentionally to learn operations, not just happy-path demos.

1. Remove SQL allow-list env var.
- Expect policy denial or connection rejection.

2. Remove SMTP allow-list env var.
- Expect mail send denial/failure.

3. Tighten anomaly threshold in SQL.
- Verify anomaly_count rises in summary line.

4. Loosen anomaly threshold.
- Verify anomaly_count falls.

## What To Customize For Real Teams

1. Replace inline CTE with your real KPI table query.
2. Externalize threshold values (churn/mrr targets) as workflow inputs.
3. Add recipients for product, finance, and on-call channels.
4. Add a second branch for paging/escalation when anomaly_count exceeds a threshold.

## Production Readiness Checklist

Before adopting in production-like operations:

1. Confirm output shape contract in JSON mode.
2. Verify anomaly thresholds with domain owners.
3. Run both success and failure drills.
4. Document policy env requirements beside run command.
5. Add this workflow to weekly telemetry review notes.

This is where Grapheme moves from language capability to operational leverage.