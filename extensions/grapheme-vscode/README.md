# Grapheme VS Code Extension

VS Code language support for AgentQL (`.aql`) powered by `grapheme-lsp`.

## Runtime Model

The extension runs a prebuilt LSP binary and does not invoke Cargo at runtime.

Binary resolution order:

1. `grapheme.lsp.binaryPath` (explicit path)
2. bundled binary at `server/grapheme-lsp` (or `.exe` on Windows)
3. auto-download from GitHub Releases into VS Code global storage

## Supported Features

- diagnostics from parser errors
- document formatting via LSP
- syntax grammar and language configuration for `.aql`
- transform op hover hints with typed return-shape info
- transform op completion hints (triggered by `.`) with return-shape details
- go-to-definition for top-level executable references
- document symbols for queries/iterators/mutations/subscriptions
- snippet completions for common language constructs
- find references for symbols across open `.aql` files
- rename symbol support for top-level executable references
- signature help while typing transform operation arguments

Notes:

- references/rename/definition use workspace indexing for `.aql` files, including unopened files

## Settings

- `grapheme.lsp.binaryPath`
- `grapheme.lsp.releaseRepo`
- `grapheme.lsp.releaseTag`
- `grapheme.lsp.autoDownload`

## Expected Release Asset Names

Auto-download looks for one of these names by platform:

- `grapheme-lsp-linux-x64`
- `grapheme-lsp-linux-arm64`
- `grapheme-lsp-macos-x64`
- `grapheme-lsp-macos-arm64`
- `grapheme-lsp-windows-x64.exe`
- `grapheme-lsp-windows-arm64.exe`

## Local Development

```bash
cd extensions/grapheme-vscode
npm install
npm run build
```

Press `F5` in VS Code to launch an Extension Development Host.

## Package VSIX

```bash
cd extensions/grapheme-vscode
npx --yes @vscode/vsce package --allow-missing-repository
```

## Related Docs

- LSP quickstart: `docs/lsp/quickstart.md`
- Release guide: `docs/release/lsp-release.md`
