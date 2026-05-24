# 04: Integrations and Policy Boundaries

Goal: compose external capabilities while keeping side effects governable.

## Recommended examples

```bash
grapheme run examples/http-get.gr --json
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: grapheme run examples/sql-transaction.gr --json
GRAPHEME_ALLOWED_SECRETS=api_key grapheme run examples/secrets-sign.gr --native-modules --json
```

## Policy mindset

Treat policy as a deployment contract, not as inline workflow logic.

- workflow source defines intent,
- policy env/config defines allowed side effects.

## Exercise

Run once without allow-list env vars, then with them.

Expected learning:

- policy denial is an expected safety signal,
- authorized run path is explicit and auditable.
