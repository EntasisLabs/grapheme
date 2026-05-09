# Grapheme VS Code Extension

Grapheme language support for `.aql` files using a prebuilt `grapheme-lsp` binary.

## Runtime Model

The extension does **not** launch Cargo for LSP execution.

Binary resolution order:

1. `grapheme.lsp.binaryPath` (explicit absolute path)
2. bundled binary at `server/grapheme-lsp` (or `.exe` on Windows)
3. auto-download from GitHub Releases into VS Code global storage

## GitHub Release Assets

Publish one binary per target with names:

- `grapheme-lsp-linux-x64`
- `grapheme-lsp-linux-arm64`
- `grapheme-lsp-macos-x64`
- `grapheme-lsp-macos-arm64`
- `grapheme-lsp-windows-x64.exe`
- `grapheme-lsp-windows-arm64.exe`

## Settings

- `grapheme.lsp.binaryPath`
- `grapheme.lsp.releaseRepo`
- `grapheme.lsp.releaseTag`
- `grapheme.lsp.autoDownload`

## Build

```bash
cd extensions/grapheme-vscode
npm install
npm run build
```

## Package VSIX

```bash
npm install -g @vscode/vsce
cd extensions/grapheme-vscode
vsce package
```
