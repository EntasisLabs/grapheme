# Grapheme LSP Release Guide

## CI Workflow

Workflow file:

- .github/workflows/release-lsp.yml

Triggers:

- push tag `v*`
- manual dispatch with optional inputs:
  - `tag`
  - `publish` (true/false)

Build outputs (asset names expected by extension downloader):

- grapheme-lsp-linux-x64
- grapheme-lsp-linux-arm64
- grapheme-lsp-macos-x64
- grapheme-lsp-macos-arm64
- grapheme-lsp-windows-x64.exe
- grapheme-lsp-windows-arm64.exe

If publish is enabled, assets are uploaded to the GitHub release.

## Manual Script

Script file:

- scripts/release-lsp.sh

Bundle script (LSP + VSIX):

- scripts/release-bundle.sh

Build host target only:

- ./scripts/release-lsp.sh

Build explicit targets:

- ./scripts/release-lsp.sh --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu

Build and publish:

- ./scripts/release-lsp.sh --target x86_64-unknown-linux-gnu --tag v0.1.0 --publish

Override release repo:

- ./scripts/release-lsp.sh --target x86_64-unknown-linux-gnu --tag v0.1.0 --publish --repo owner/repo

Notes:

- Publishing requires `gh` CLI auth.
- Artifacts are written to `dist/lsp-release`.

## Bundle Script

Build host LSP + VSIX:

- ./scripts/release-bundle.sh

Build only VSIX:

- ./scripts/release-bundle.sh --skip-lsp

Build only LSP assets:

- ./scripts/release-bundle.sh --skip-vsix

Build and publish both:

- ./scripts/release-bundle.sh --target x86_64-unknown-linux-gnu --tag v0.1.0 --publish

Bundle artifacts are written to `dist/release-bundle`.
