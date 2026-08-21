# Grapheme LSP and VSIX Release Guide

Release governance gates and docs versioning policy:

- `docs/internal/release/release-gates-and-doc-versioning.md`

This repository supports both CI-based and manual release flows for:

1. `grapheme-lsp` platform binaries
2. `grapheme-vscode` VSIX package

## CI Workflow

Workflow file:

- `.github/workflows/release-lsp.yml`

Expected binary asset names (used by extension auto-download logic):

- `grapheme-lsp-linux-x64`
- `grapheme-lsp-linux-arm64`
- `grapheme-lsp-macos-x64`
- `grapheme-lsp-macos-arm64`
- `grapheme-lsp-windows-x64.exe`
- `grapheme-lsp-windows-arm64.exe`

## Manual LSP Release Script

Script:

- `scripts/release-lsp.sh`

Build for host target:

```bash
./scripts/release-lsp.sh
```

Build for explicit targets:

```bash
./scripts/release-lsp.sh \
  --target x86_64-unknown-linux-gnu \
  --target aarch64-unknown-linux-gnu
```

Build and publish to release tag:

```bash
./scripts/release-lsp.sh \
  --target x86_64-unknown-linux-gnu \
  --tag v0.7.0 \
  --publish
```

Publish to a different repo:

```bash
./scripts/release-lsp.sh \
  --target x86_64-unknown-linux-gnu \
  --tag v0.7.0 \
  --publish \
  --repo owner/repo
```

Artifacts output to:

- `dist/lsp-release`

## Bundle Script (LSP + VSIX)

Script:

- `scripts/release-bundle.sh`

Build both artifacts:

```bash
./scripts/release-bundle.sh
```

Build only VSIX:

```bash
./scripts/release-bundle.sh --skip-lsp
```

Build only LSP binaries:

```bash
./scripts/release-bundle.sh --skip-vsix
```

Build and publish combined bundle:

```bash
./scripts/release-bundle.sh \
  --target x86_64-unknown-linux-gnu \
  --tag v0.7.0 \
  --publish
```

Bundle output directory:

- `dist/release-bundle`

## Release Prerequisites

- Rust toolchain + `rustup`
- Node.js + npm
- `gh` CLI authenticated (for `--publish`)

## Notes

- Extension packaging currently works with `package.json` `files` allowlist.
- `vsce` may still warn about file count/bundling; this is an optimization concern, not a release blocker.
- CLI now supports bundled example extraction (`grapheme examples init --out .`) for cargo-installed environments.
