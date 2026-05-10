# Runtime Policy and Security

## Runtime Validation Stages

Before execution:

1. Artifact format compatibility check.
2. Artifact integrity check (sha256 over MIR payload).
3. Entrypoint lookup in MIR function set.

During execution (per call):

1. Capability allow/deny policy check.
2. Module/op lookup in registry.
3. Policy guard argument check.
4. ABI dispatch (`MirV1` host or `WasixV1` module).

## PolicyGuard Rules

Current operation-specific checks:

- `http.get`, `http.post`: URL host must match `allowed_http_domains` when configured.
- `tcp.connect`: target must match `allowed_tcp_targets` when configured.
- `smtp.send_mail`: recipient domain must match `allowed_smtp_domains` when configured.
- `secrets.get_secret_handle`, `secrets.sign_request`: secret name must match `allowed_secret_names` when configured.

If a configured allowlist does not match, runtime fails the step with `POLICY_DENIED`.

## CLI Environment Mapping

The CLI builds `PolicyGuard` from environment variables:

- `GRAPHEME_ALLOWED_HTTP_DOMAINS`
- `GRAPHEME_ALLOWED_TCP_TARGETS`
- `GRAPHEME_ALLOWED_SMTP_DOMAINS`
- `GRAPHEME_ALLOWED_SECRETS`

Each variable is parsed as comma-separated values.

## Examples

Allow only `example.com` for HTTP:

```bash
GRAPHEME_ALLOWED_HTTP_DOMAINS=example.com \
  cargo run -- run examples/http-get.aql --native-modules
```

Allow only one secret by name:

```bash
GRAPHEME_ALLOWED_SECRETS=api_key \
  cargo run -- run examples/secrets-sign.aql --native-modules
```

## Current Limitations

- Policy checks are focused on selected operations and argument fields.
- Deeper semantic policy (payload schemas, contextual approvals) is planned.
- Memory persistence and long-horizon data governance are still evolving.
