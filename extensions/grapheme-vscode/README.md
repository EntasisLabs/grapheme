# Grapheme VS Code Extension

VS Code language support for Grapheme Lang (`.gr`) powered by `grapheme-lsp` **0.6.0**.

## Runtime Model

The extension runs a prebuilt LSP binary and does not invoke Cargo at runtime.

Binary resolution order:

1. `grapheme.lsp.binaryPath` (explicit path)
2. bundled binary at `server/grapheme-lsp` (or `.exe` on Windows)
3. auto-download from GitHub Releases into VS Code global storage

Set `grapheme.lsp.releaseTag` to `v0.6.0` (or `latest`) when using auto-download.

## Supported Features

- diagnostics from parser errors
- document formatting via LSP
- syntax grammar and language configuration for `.gr`
- transform op hover hints with typed return-shape info
- transform op completion hints (triggered by `.`) with return-shape details
- **`$current` field completion (0.6.0+)** — infers envelope fields (`data`, `meta`, `error`) and `$current.data.*` payload fields from the prior pipeline op
- go-to-definition for top-level executable references
- document symbols for glyph/query/iterator/mutation/subscription definitions
- snippet completions for common language constructs
- find references for symbols across open `.gr` files
- rename symbol support for top-level executable references
- signature help while typing transform operation arguments (includes envelope + `data.*` output fields for capability ops)

Notes:

- references/rename/definition use workspace indexing for `.gr` files, including unopened files
- envelope hints work best in untyped `query { ... |> module.op ... }` pipelines after a capability op

## Settings

- `grapheme.lsp.binaryPath`
- `grapheme.lsp.releaseRepo`
- `grapheme.lsp.releaseTag` — use `v0.6.0` for this release train
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

- LSP quickstart: `docs/internal/lsp/quickstart.md`
- Release guide: `docs/internal/release/lsp-release.md`
- 0.6.0 platform overview: `docs/internal/roadmaps/release-0.6.0-extensible-platform.md`
