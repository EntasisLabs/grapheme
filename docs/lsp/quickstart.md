# Grapheme LSP Quickstart

## Run The Server

Use stdio mode for editor integration:

```bash
cargo run -p grapheme-lsp
```

## Current Capabilities

- `.aql` parse diagnostics via `grapheme-compiler::parse`
- full-document formatting (trailing-space cleanup + newline normalization)

## VSIX Next Step

For a VS Code extension package, wire a client that:

1. registers language id `aql`
2. starts `grapheme-lsp` over stdio
3. contributes syntax grammar for highlighting
4. contributes formatting command bound to LSP formatting

This keeps the language intelligence in Rust while VSIX handles editor UX.
