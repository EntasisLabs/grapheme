# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — 0.6.0 extensible platform

Foundation train for the coordinated 0.6.0 release. See `docs/internal/roadmaps/release-0.6.0-extensible-platform.md`.

### Added

- **Opt-in capability modules** (`data`, `pdf`, `image`, `plot`, `media`) behind Cargo feature flags on `grapheme-stdlib`, `grapheme-sdk`, `grapheme-signatures`, `grapheme-runtime`, and `grapheme-compiler`.
- **Host result envelope** (`{ data, meta, error }`) via `grapheme-stdlib::envelope` — capability scaffolds normalize to this shape.
- **Dynamic Wasm module discovery** — `grapheme-runtime::discover_wasm_modules`, manifest sidecar spec (`docs/internal/runtime/wasm-module-manifest-v1.md`, `schemas/wasm-module-manifest.schema.json`).
- **`grapheme modules scan`** — lists discovered Wasm modules from `grapheme.toml` `[modules].scan`, explicit paths, or defaults (`plugins/`, `modules/`).
- **`grapheme.toml` `[modules].scan`** — project config schema updated in `grapheme.schema.json`.

### Changed

- **`grapheme-cli`** — `default = ["full"]`; ships all capability features.
- **`grapheme-sdk`** — `default = []`; embedders opt in with `full` or per-module features (`data`, `pdf`, …).
- **`grapheme-lsp`** — ships with `full` signatures for editor discovery.

### Notes

- **`data` module (Polars, opt-in):** `data.read_csv` reads local CSV into `grapheme.data.frame/v1` handles; `data.filter`, `data.group_by`, `data.aggregate(op=count)`, `data.schema`, `data.to_json` operate on JSON frames in-pipeline.
- Examples: `examples/data-read-csv.gr`, `examples/data-filter.gr`, fixture `examples/fixtures/sample-users.csv`.
- **`pdf` Wasm plugin (reference):** `plugins/pdf-rs` implements `pdf.generate` + `pdf.extract_text` with host envelope output; sidecar at `modules/pdf.module.json`.
- **`grapheme modules activate|rollback`** — persist/remove Wasm bindings under `.grapheme/modules/bindings.json`.
- **Auto-bind on run** — discovered modules from `[modules].scan` (or `modules/` + `plugins/` defaults) bind automatically when referenced in a workflow.
- **`image` Wasm plugin:** `plugins/image-rs` — metadata, resize, convert (PNG/JPEG/GIF/WebP via `image` crate).
- **`plot` Wasm plugin:** `plugins/plot-rs` — line/bar/scatter charts as SVG envelope payloads.
- Examples: `examples/image-metadata.gr`, `examples/plot-line.gr`, fixture `examples/fixtures/sample.png`.

## [0.5.0] - 2026-06-03

User-facing release tag. Ships stdlib expansion and matching library crates.

### Added

- **`email` module** (host-backed, lettre): `email.smtp`, `email.gmail`, `email.providers`, `email.capabilities` for TLS/auth SMTP delivery with provider presets and env-based credentials (`SMTP_*`, `GMAIL_*`, `EMAIL_FROM`).
- **Web search providers** on `web.*` and `websearch.*`:
  - `google` — `GOOGLE_API_KEY` + `GOOGLE_CX`
  - `xaviv` — semantic search via Exa (`XAVIV_API_KEY` or `EXA_API_KEY`)
  - `tavily` — `TAVILY_API_KEY`
  - `brave` — native Brave Search API (`BRAVE_API_KEY`)
- Examples: `examples/email-smtp.gr`, `examples/web-tavily.gr`, `examples/web-brave.gr`.

### Changed

- `web.providers` / `web.capabilities` catalog reflects all wired providers (no breaking changes to existing ops).
- Policy guard applies recipient-domain rules to `email.smtp` and `email.gmail` (same as `smtp.send_mail`).
- SDK module search guidance includes `email`.

### Crate versions (this release)

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.4.0 |
| `grapheme-stdlib` | 0.4.0 |
| `grapheme-runtime` | 0.4.0 |
| `grapheme-compiler` | 0.4.0 |
| `grapheme-sdk` | 0.4.0 |
| `grapheme-cli` | 0.5.0 |
| `grapheme-lsp` | 0.5.0 |
| VS Code extension | 0.5.0 |
| `grapheme-artifact` | 0.2.0 (unchanged) |

### Notes

- **`smtp.send_mail`** remains available; the `email` module is additive.
- **Brave** uses a direct API integration because `websearch` 0.1.1 still stubs `BraveProvider`.

## [0.4.1] - 2026-05-24

### Changed

- CLI patch release on the 0.4.x tooling line.

## [0.4.0] - 2026-05-24

### Changed

- Major tooling and crate alignment: CLI/LSP/VS Code extension 0.4.x, compiler/runtime/signatures/stdlib 0.3.x → 0.4.x train (see git tag history for full scope).
- Parser and documentation updates for newline handling and release packaging.

## [0.3.0] - 2026-05

### Added

- Extended web provider routing examples, HTTP/websearch modules, and real-world scenario pack foundations.
