# Grapheme LSP Quickstart

This guide covers local development and validation for `crates/grapheme-lsp`.

## What The LSP Currently Provides

- parse diagnostics for `.gr` documents via `grapheme_compiler::parse`
- full-document formatting (trim trailing spaces, normalize ending newline)
- hover hints for transform ops (`html.to_md`, `json.parse`, `csv.to_list`, `yaml.to_json`)
- completion for transform ops with typed return-shape details
- go-to-definition for `glyph`/`query`/`iterator`/`mutation`/`subscription` references
- document symbols for top-level executable definitions
- snippet completions for key language constructs (`query`, `iterator`, `flow.branch`)
- find references for symbols across open `.gr` documents
- rename symbol support for top-level executable names and usages
- signature help for transform calls while typing args

## Transform Hover and Completion Hints

Hovering a transform op shows:

- operation summary
- primary arg name and expected input type
- typed return-shape hint

Completion supports:

- trigger on `.` after module names
- op suggestions with return-shape detail
- context filtering by typed module prefix
- snippet insertion with named args for transform calls

## References, Rename, and Signature Help

- References: resolves symbol usages across workspace `.gr` files (including unopened files under workspace roots).
- Rename: applies coordinated text edits for symbol occurrences across workspace `.gr` files.
- Signature help: on `(` and `,` for transform calls, shows arg contract and return shape.

Workspace indexing notes:

- Indexed extensions: `.gr`
- Root sources: LSP workspace folders (fallback: root URI)
- Skipped directories: `.git`, `target`, `node_modules`, `.vscode`

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
