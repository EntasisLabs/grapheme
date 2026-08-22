# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-08-21

**Language + Stage B release** — executable parameters / tagged variables (RFC-0004) and a Wasm-compilable Stage B AOT path (RFC-0005). See `docs/internal/rfc/rfc-0004-params-and-tagged-vars-v1.md` and `docs/internal/rfc/rfc-0005-wasm-compilable-stdlib-v1.md`.

### Added

- **Executable parameters (RFC-0004 Phase 1)** — GraphQL-style `$param` lists on `query` / `mutation` / `iterator` / `subscription`, lowered through HIR → MIR → runtime locals with CLI/SDK entrypoint arg binding.
- **Tag schemas + scoped `using` (RFC-0004 Phase 2a)** — tagged variable schemas and block-scoped `using` for ambient auth/session-style handles.
- **Wasm-safe stdlib profile (RFC-0005)** — `grapheme-stdlib` feature split: default `host` vs `wasm` / `transforms` so Stage B containers compile for `wasm32-wasip1` without TLS/DB stacks.
- **`grapheme-aot-container`** — slim MIR walker WASI binary for Stage B workflows; local ops for `core` / `json` / `csv` / `yaml` / `html`; host-only ops surface as `grapheme.runtime.host.v1::call.capability` stubs.
- **Stage B host fulfillment** — multi-round in-process walker + `CapabilityHost` fulfillment (`call.capability`, `state.read` / `state.write`) via `host_fulfillments`.
- **Default Stage B emission** — `compile_to_aot_stage_b` / SDK `compile_source_to_aot_stage_b_default` + `scripts/build-aot-container.sh`.
- **Wasix multi-round Stage B** — optional sandbox path (`prefer_stage_b_wasix` / `GRAPHEME_PREFER_STAGE_B_WASIX`) using the same fulfillment contract.
- **Stage A ↔ Stage B parity fixtures** — outcome + `state.current` parity for core transforms and host-capability paths.

### Changed

- **Toolchain** — Rust edition **2024** / **rust-version 1.92+** required.
- **Strict Stage B** — in-process container walker is the primary path; Wasix is opt-in; parity fallback still available when not strict.
- **CI** — conformance gates for Wasm stdlib check, Stage B container, parity tests, and Wasix multi-round (builds container artifact first).
- **Docs / examples** — onboarding covers params, tags/`using`, and Stage B; canonical demos `examples/params-call-bind.gr` and `examples/tag-using-scope.gr`; author extract `docs/internal/language/params-and-tags-v1.md`.

### Crate versions (this release)

All workspace language crates ship at **0.7.0** except `grapheme-artifact` (unchanged at 0.2.0). The release tag is **v0.7.0**.

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.7.0 |
| `grapheme-stdlib` | 0.7.0 |
| `grapheme-aot-container` | 0.7.0 |
| `grapheme-runtime` | 0.7.0 |
| `grapheme-compiler` | 0.7.0 |
| `grapheme-sdk` | 0.7.0 |
| `grapheme-cli` | 0.7.0 |
| `grapheme-lsp` | 0.7.0 |
| VS Code extension | 0.7.0 |
| `grapheme-artifact` | 0.2.0 (unchanged) |

### Notes

- Last published Git tag before this cut was **v0.4.0**; Cargo already carried 0.5.x/0.6.x trains without matching tags. **v0.7.0** is the aligned language + Stage B cut.
- Build the Stage B wasm asset before Wasix sandbox runs: `./scripts/build-aot-container.sh`.
- RFC-0004 Phases 3+ (tag-typed parameters as the full call-edge model, `uses` sugar) remain follow-ups beyond this release.

## [0.6.1] - 2026-06-03

### Fixed

- **`grapheme-stdlib` publish tarball** — `src/data/` was excluded by a overly broad `.gitignore` rule (`data/`); fixed to `/data/` and included `data/mod.rs` + `data/frame.rs` in the crate.

## [0.6.0] - 2026-06-03

**Extensible platform release** — Wasm discovery, hotload, typed host envelopes, and opt-in capability modules. See `docs/internal/roadmaps/release-0.6.0-extensible-platform.md`.

### Added

- **Opt-in capability modules** (`data`, `pdf`, `image`, `plot`, `media`) behind Cargo feature flags on `grapheme-stdlib`, `grapheme-sdk`, `grapheme-signatures`, `grapheme-runtime`, and `grapheme-compiler`.
- **Host result envelope** (`{ data, meta, error }`) via `grapheme-stdlib::envelope` — capability results normalize to this shape with legacy flat dual-read.
- **Dynamic Wasm module discovery** — `grapheme-runtime::discover_wasm_modules`, manifest sidecar spec (`docs/internal/runtime/wasm-module-manifest-v1.md`, `schemas/wasm-module-manifest.schema.json`).
- **`grapheme modules scan|activate|rollback|status`** — discover Wasm modules, persist generations in `.grapheme/modules/hotload.json`, rollback with in-flight pinning metadata.
- **Hotload store (v1)** — `.grapheme/modules/hotload.json` hydrates CLI runs and SDK sessions via `with_default_hotload_store()`.
- **Reference Wasm plugins** — `plugins/pdf-rs`, `plugins/image-rs`, `plugins/plot-rs` with manifests under `modules/*.module.json`.
- **`data` module (Polars, native)** — `read_csv`, `filter`, `group_by`, `aggregate(op=count)`, `schema`, `to_json` on `grapheme.data.frame/v1` handles.
- **`media` module (ffmpeg native)** — `media.probe` (ffprobe JSON) and `media.transcode` (ffmpeg CLI bridge).
- **LSP envelope hints** — `$current` / `$current.data.*` completion and hover infer payload fields from the prior pipeline op; signature help lists envelope + data output fields.
- **Examples** — `data-read-csv.gr`, `data-filter.gr`, `pdf-generate.gr`, `image-metadata.gr`, `plot-line.gr`, `media-probe.gr`, `platform-release-060.gr`.

### Changed

- **`grapheme-cli`** — `default = ["full"]`; ships all capability features (v0.6.0).
- **`grapheme-sdk`** — `default = []`; embedders opt in with `full` or per-module features (`data`, `pdf`, …).
- **`grapheme-lsp`** — ships with `full` signatures for editor discovery (v0.6.0).
- **Auto-bind on run** — discovered modules from `[modules].scan` (or `modules/` + `plugins/` defaults) bind automatically when referenced in a workflow.

### Crate versions (this release)

All workspace crates ship at **0.6.0** except `grapheme-artifact` (unchanged at 0.2.0). The release tag is **v0.6.0**.

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.6.1 |
| `grapheme-stdlib` | 0.6.1 |
| `grapheme-runtime` | 0.6.1 |
| `grapheme-compiler` | 0.6.1 |
| `grapheme-sdk` | 0.6.1 |
| `grapheme-cli` | 0.6.1 |
| `grapheme-lsp` | 0.6.1 |
| VS Code extension | 0.6.0 |
| `grapheme-artifact` | 0.2.0 (unchanged) |

### Notes

- **`pdf` / `image` / `plot`** resolve to Wasm plugins when activated or auto-bound; stdlib scaffolds remain fallbacks for Mir-only paths.
- **`media`** requires `ffmpeg` and `ffprobe` on `PATH` for probe/transcode ops.
- Legacy `.grapheme/modules/bindings.json` is still read when no hotload store exists.

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
