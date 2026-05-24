# 10: Build-Along - Invoice Intake Validation + Approval Routing

Goal: automate AP intake routing with clear, auditable decision criteria.

You will implement one practical workflow:

1. query invoice intake rows,
2. apply routing rules (`auto_approve`, `finance_review`, `needs_po`),
3. send an approval-routing digest email.

## Why This Matters

Accounts-payable intake is repetitive and error-prone when routed manually.

This workflow makes the policy explicit and reviewable.

## Final Workflow File

- `examples/realworld/invoice-intake-approval-digest.gr`

## Business Rules

Current example rules:

- `amount_usd > 5000` -> `finance_review`
- missing PO number -> `needs_po`
- otherwise -> `auto_approve`

Digest includes:

- per-invoice line with route decision
- summary counts per route class

## Run It

CLI path:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
GRAPHEME_ALLOWED_SMTP_DOMAINS=example.com \
  grapheme run examples/realworld/invoice-intake-approval-digest.gr --json
```

Workspace path:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
GRAPHEME_ALLOWED_SMTP_DOMAINS=example.com \
  cargo run -- run examples/realworld/invoice-intake-approval-digest.gr --json
```

## Expected Output Signals

Look for:

- `"outcome": "succeeded"`
- SMTP accepted state
- digest body with `SUMMARY | total=...` line

## Required Failure Drills

1. Remove SQL allow-list env var.
- Observe policy/connectivity failure behavior.

2. Remove SMTP allow-list env var.
- Observe mail send failure behavior.

3. Raise or lower finance threshold in SQL.
- Confirm `finance_review` count changes in summary.

4. Populate missing PO value for one invoice.
- Confirm `needs_po` count drops.

## Production Adaptation

1. Replace inline CTE with real AP intake table/view.
2. Externalize thresholds and routing policy.
3. Add approver metadata in output lines.
4. Add branch for paging finance-oncall when review queue spikes.

This pattern turns finance routing policy into executable, testable workflow logic.
