# CLI Reference

The CLI binary is `grapheme` (crate `crates/grapheme-cli`).

## Usage

```bash
grapheme <file.gr>
grapheme parse <file.gr>
grapheme compile <file.gr> --emit ast|hir|mir|artifact
grapheme plugins build [all|core|docs|io|http|memory|tcp|smtp|secrets ...]
grapheme run <file.gr> [--bind module=path.wasm ...] [--json] [--native-modules] [--stream-steps]
grapheme modules
grapheme modules search <query>
grapheme modules info <module>
grapheme modules types <module>
grapheme modules examples <module>
```

## Commands

### `parse`

Parses source and prints AST JSON.

```bash
cargo run -- parse examples/hello-world.gr
```

### `compile`

Compiles source and prints selected intermediate or final representation.

```bash
cargo run -- compile examples/hello-world.gr --emit ast
cargo run -- compile examples/hello-world.gr --emit hir
cargo run -- compile examples/hello-world.gr --emit mir
cargo run -- compile examples/hello-world.gr --emit artifact
```

### `run`

Compiles and executes a program.

Plain text output (best-effort extraction from final state):

```bash
cargo run -- run examples/hello-world.gr
```

By default, plain mode prints actual emitted outputs (for example `core.echo` messages) without step trace prefixes.

Opt-in step trace streaming with prefixes like `[iter 1 | depth 1 | echo]`:

```bash
cargo run -- run examples/fibonacci-computed.gr --native-modules --stream-steps
```

Structured JSON output:

```bash
cargo run -- run examples/hello-world.gr --json
```

Manual module binding:

```bash
cargo run -- run examples/http-get.gr --bind http=plugins/http-rs.wasm
```

Auto-build and auto-bind known modules:

```bash
cargo run -- run examples/core-merge.gr --native-modules
```

`--native-modules` auto-builds/auto-binds known Wasm plugins except `http`, `tcp`, and `smtp`, which are host-backed by default for real socket access in the current runtime.

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

Search manifests by module id or op name:

```bash
cargo run -- modules search http
```

Show one manifest:

```bash
cargo run -- modules info http
```

Show op type metadata for one module:

```bash
cargo run -- modules types http
```

Show curated example files for one module:

```bash
cargo run -- modules examples http
```

Web search module example:

```bash
cargo run -- modules examples websearch
```

Run the high-level research pipeline (search -> fetch -> clean -> report):

```bash
cargo run -- run examples/websearch-report.gr --json
```

Run the composable research-materials pipeline (search -> fetch -> markdown + structured payload):

```bash
cargo run -- run examples/websearch-materials.gr --json
```

Use this when an LLM should decide its own output shape (summary, memo, citations, markdown artifact)
from source-grounded materials rather than relying on a fixed built-in report template.

For the expanded core std helpers, discover examples with:

```bash
cargo run -- modules examples core
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
  cargo run -- run examples/secrets-sign.gr --native-modules
```
