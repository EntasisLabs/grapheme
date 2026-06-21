# Troubleshooting

This guide covers the most common first-run and development failures with exact remediation steps.

## 1) `cargo run -- run examples/hello-world.gr` fails with target/toolchain errors

Symptoms:

- Cargo reports missing toolchain components.
- Build fails before parsing/execution.

Fix:

```bash
rustup update
rustup target add wasm32-wasip1
cargo check --workspace
```

## 2) `--native-modules` fails while building plugins

Symptoms:

- Plugin build errors during `cargo run -- run ... --native-modules`.
- Missing Wasm target or plugin build prerequisites.

Fix:

```bash
rustup target add wasm32-wasip1
cargo run -- plugins build all
cargo run -- run examples/core-merge.gr --native-modules
```

## 3) HTTP/TCP/SMTP calls fail due to policy denial

Symptoms:

- Runtime denies network calls with policy-related errors.

Fix:

```bash
GRAPHEME_ALLOWED_HTTP_DOMAINS=example.com \
  cargo run -- run examples/http-get.gr --native-modules

GRAPHEME_ALLOWED_TCP_TARGETS=example.com:443 \
  cargo run -- run examples/tcp-connect.gr --native-modules

GRAPHEME_ALLOWED_SMTP_DOMAINS=example.com \
  cargo run -- run examples/smtp-send.gr --native-modules
```

## 4) SQL calls are denied

Symptoms:

- SQL examples fail with allow-list/policy errors.

Fix:

```bash
GRAPHEME_ALLOWED_SQL_CONNECTIONS=sqlite::memory: \
  cargo run -- run examples/sql-query.gr --json
```

## 5) Surreal calls are denied

Symptoms:

- Surreal examples fail with connection allow-list policy errors.

Fix:

```bash
GRAPHEME_ALLOWED_SURREAL_CONNECTIONS=memory \
  cargo run -- run examples/surreal-health.gr --json
```

Use the connection id expected by your example/environment.

## 6) Secret operations fail due to secret allow-list

Symptoms:

- `secrets.*` calls denied even when code is valid.

Fix:

```bash
GRAPHEME_ALLOWED_SECRETS=api_key \
  cargo run -- run examples/secrets-sign.gr --native-modules
```

## 7) Parse error for pipeline or directive syntax

Symptoms:

- Parse errors around `|>`, directive blocks, or inline target syntax.

Fix:

1. Validate syntax against the language contract in `docs/language-contract.md`.
2. Use parser output to localize the issue:

```bash
cargo run -- parse path/to/file.gr
```

3. Compare against canonical examples in `examples/README.md`.

## 8) Ambiguous entrypoint error

Symptoms:

- Compile fails with an error about multiple roots and no glyph.

Fix:

1. Add a `glyph` as composition root.
2. Or pass an explicit entrypoint via the API/CLI flow you are using.

Minimal `glyph` pattern:

```gr
glyph Main {
  A
}

query A {
  set { message: "hello" }
  |> core.echo(message: $current.message)
}
```

## 9) Stage B AOT strict-mode execution fails

Symptoms:

- Stage B execution fails in strict mode due to runtime/container constraints.

Fix:

```bash
cargo run -- run examples/hello-world.gr --aot-stage stage_a --json
```

Or opt into Stage B fallback behavior explicitly:

```bash
cargo run -- run examples/hello-world.gr --aot-stage stage_b --allow-stage-b-fallback --json
```

## 10) LSP starts but editor integration does not behave as expected

Symptoms:

- No diagnostics/completions, or extension does not attach to expected binary.

Fix:

1. Validate LSP binary builds:

```bash
cargo build -p grapheme-lsp --release
```

2. Validate extension binary resolution order and settings via:
- `docs/internal/lsp/quickstart.md`
- `extensions/grapheme-vscode/README.md`

Set `grapheme.lsp.releaseTag` to `v0.6.0` when using auto-download for envelope-hint support.

## 11) Wasm capability module not found on run

Symptoms:

- `grapheme run examples/pdf-generate.gr` fails to resolve `pdf` ops.
- `modules scan` returns empty for expected plugins.

Fix:

1. Build capability plugins:

```bash
bash plugins/build-plugins.sh
```

2. Scan and activate (or rely on auto-bind after activate once):

```bash
grapheme modules scan
grapheme modules activate pdf
grapheme modules status
```

3. Confirm artifacts exist: `modules/pdf.wasm`, `modules/pdf.module.json`.

4. Optional `grapheme.toml` scan paths:

```toml
[modules]
scan = ["modules", "plugins"]
```

See `docs/internal/cli.md` and `plugins/README.md`.

## 12) `media.probe` / `media.transcode` fails

Symptoms:

- Envelope error mentioning `ffprobe not available` or `ffmpeg not available`.

Fix:

1. Install ffmpeg (includes ffprobe) and ensure both are on `PATH`:

```bash
ffmpeg -version
ffprobe -version
```

2. Re-run:

```bash
grapheme run examples/media-probe.gr
```

## Fast Recovery Checklist

Run these before deeper debugging:

```bash
cargo check --workspace
cargo test -p grapheme-compiler --lib
cargo test -p grapheme-runtime --lib
cargo test -p grapheme-sdk --lib
```

If issues persist, capture:

1. Command run
2. Full stderr/stdout
3. Example or minimal `.gr` source
4. Policy environment variables in effect
