# CLI Reference

The CLI binary is `grapheme` (crate `crates/grapheme-cli`).

## Usage

```bash
grapheme <file.aql>
grapheme parse <file.aql>
grapheme compile <file.aql> --emit ast|hir|mir|artifact
grapheme plugins build [all|core|docs|io|http|memory|tcp|smtp|secrets ...]
grapheme run <file.aql> [--bind module=path.wasm ...] [--json] [--native-modules]
grapheme modules
```

## Commands

### `parse`

Parses source and prints AST JSON.

```bash
cargo run -- parse examples/hello-world.aql
```

### `compile`

Compiles source and prints selected intermediate or final representation.

```bash
cargo run -- compile examples/hello-world.aql --emit ast
cargo run -- compile examples/hello-world.aql --emit hir
cargo run -- compile examples/hello-world.aql --emit mir
cargo run -- compile examples/hello-world.aql --emit artifact
```

### `run`

Compiles and executes a program.

Plain text output (best-effort extraction from final state):

```bash
cargo run -- run examples/hello-world.aql
```

Structured JSON output:

```bash
cargo run -- run examples/hello-world.aql --json
```

Manual module binding:

```bash
cargo run -- run examples/http-get.aql --bind http=plugins/http-rs.wasm
```

Auto-build and auto-bind known modules:

```bash
cargo run -- run examples/core-merge.aql --native-modules
```

### `plugins build`

Builds plugin crates for `wasm32-wasip1` and copies canonical outputs to `plugins/*.wasm`.

```bash
cargo run -- plugins build all
cargo run -- plugins build core io http
```

### `modules`

Prints runtime module manifests as JSON.

```bash
cargo run -- modules
```

## Environment Variables

Runtime policy env vars consumed by CLI:

- `GRAPHEME_ALLOWED_HTTP_DOMAINS` (comma-separated hosts)
- `GRAPHEME_ALLOWED_TCP_TARGETS` (comma-separated host:port entries)
- `GRAPHEME_ALLOWED_SMTP_DOMAINS` (comma-separated domains)
- `GRAPHEME_ALLOWED_SECRETS` (comma-separated secret names)

Example:

```bash
GRAPHEME_ALLOWED_SECRETS=api_key \
  cargo run -- run examples/secrets-sign.aql --native-modules
```
