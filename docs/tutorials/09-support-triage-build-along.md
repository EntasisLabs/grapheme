# 09: Build-Along - Support Ticket Triage + Queue Digest

Goal: automate daily support triage so humans do less routing and more resolution.

You will implement one practical workflow:

1. read incoming ticket objects,
2. route each ticket into explicit queues,
3. emit a digest email for support leads.

## Why This Matters

Most teams still triage support tickets manually in Slack/Jira threads.

This workflow creates one reviewable source of truth for routing policy.

## Final Workflow File

- `examples/realworld/support-ticket-triage-digest.gr`

## Business Rules

Current routing logic:

- `critical` -> `incident-oncall`
- `high` -> `priority-response`
- everything else -> `standard-backlog`

Each line includes:

- ticket id
- chosen queue
- priority
- customer tier
- SLA minutes
- topic

## Run It

CLI path:

```bash
GRAPHEME_ALLOWED_SMTP_DOMAINS=example.com \
  grapheme run examples/realworld/support-ticket-triage-digest.gr --json
```

Workspace path:

```bash
GRAPHEME_ALLOWED_SMTP_DOMAINS=example.com \
  cargo run -- run examples/realworld/support-ticket-triage-digest.gr --json
```

## Expected Output Signals

Look for:

- `"outcome": "succeeded"`
- SMTP accepted state
- digest body containing queue assignments for all tickets

## Required Failure Drills

1. Remove SMTP allow-list env var.
- Observe failure and document remediation.

2. Add a new `critical` ticket row.
- Verify it routes to `incident-oncall`.

3. Downgrade a `high` ticket to `normal`.
- Verify route changes to `standard-backlog`.

## Production Adaptation

1. Replace inline ticket list with your ticket source integration.
2. Add business-hour and region routing rules.
3. Add VIP customer overrides.
4. Add escalation summary metrics in email footer.

This is the practical pattern: explicit routing policy in code, predictable digest output for operators.
