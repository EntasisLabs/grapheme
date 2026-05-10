# Grapheme LSP Quickstart

This guide covers local development and validation for `crates/grapheme-lsp`.

## What The LSP Currently Provides

- parse diagnostics for `.aql` documents via `grapheme_compiler::parse`
- full-document formatting (trim trailing spaces, normalize ending newline)

## Run Locally

```bash
cargo run -p grapheme-lsp
```

The server communicates over stdio and is intended to be launched by an editor client.

## Build Release Binary

```bash
cargo build -p grapheme-lsp --release
```

Binary output:

- `target/release/grapheme-lsp`

## Validate In VS Code Extension

The extension in `extensions/grapheme-vscode` runs a prebuilt binary and resolves it in this order:

1. `grapheme.lsp.binaryPath`
2. bundled binary in `server/`
3. GitHub release auto-download

For extension details, see:

- `extensions/grapheme-vscode/README.md`

For release flows, see:

- `docs/release/lsp-release.md`
