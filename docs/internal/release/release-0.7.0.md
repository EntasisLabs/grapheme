# Release 0.7.0 — Language params/tags + Stage B AOT

Status: prep
Target tag: **v0.7.0**
Created: 2026-08-21

## Scope

Ship the post-0.6.1 train that landed on `main`:

1. **RFC-0004** — executable parameters + tagged variables / scoped `using` (Phases 1–2a)
2. **RFC-0005** — Wasm-compilable stdlib, Stage B container, host fulfillment, default emission, parity, Wasix multi-round
3. **Toolchain** — edition 2024 / Rust 1.92+

## Pre-cut checklist

Use with `docs/internal/release/release-gates-and-doc-versioning.md`.

### Versions

- [x] Workspace crates bumped to **0.7.0** (`grapheme-artifact` stays **0.2.0**)
- [x] VS Code extension `package.json` / lock → **0.7.0**
- [x] `CHANGELOG.md` `[0.7.0]` entry
- [x] `scripts/publish-crates.sh` includes `grapheme-aot-container` (after stdlib)

### Docs / examples currency

- [x] Language contract updated for params/tags (no longer says binding is unfinished)
- [x] Quickstart / getting-started / language-tour cover params, tags, Stage B
- [x] CLI docs: `--args-json`, Stage B env vars, container build script
- [x] SDK docs: `with_entrypoint_args`, Stage B compile/execute helpers
- [x] Canonical examples: `examples/params-call-bind.gr`, `examples/tag-using-scope.gr`
- [x] Normative extract: `docs/internal/language/params-and-tags-v1.md`
- [ ] Final human skim of product FAQ / tutorials before tag

### Gates (fill at cut time)

- [ ] `conformance` green on release PR / `main`
- [ ] `conformance-wasix` green (builds Stage B wasm first)
- [ ] `cargo doc --workspace --no-deps`
- [ ] Spot-check: `cargo test -p grapheme-sdk stage_a_vs_stage_b`
- [ ] Spot-check: `./scripts/build-aot-container.sh` then Wasix multi-round test

### Cut sequence

1. Merge version-prep PR to `main`
2. Tag `v0.7.0` on the merge commit (`git tag -a v0.7.0 -m "v0.7.0"` + push tags)
3. Confirm `.github/workflows/release-lsp.yml` uploads LSP assets for the tag
4. Optional: `scripts/release-bundle.sh --tag v0.7.0`
5. Optional: `scripts/publish-crates.sh --publish` (dry-run first)
6. Point extension `grapheme.lsp.releaseTag` docs / users at `v0.7.0`
7. Publish GitHub Release notes from `CHANGELOG.md` `[0.7.0]`

### Explicit non-goals for 0.7.0

- Full MIR→Wasm lowering (interpreter-in-Wasm container is enough)
- RFC-0004 Phase 3+ (tag-typed params as fundamental call-edge; `uses` sugar)
- Publishing plugin crates (`plugins/*-rs` remain 0.1.0 / excluded)

## Owners sign-off

| Gate | Owner | Sign-off |
| --- | --- | --- |
| Runtime / Stage B | | |
| Tooling (LSP / VSIX) | | |
| Docs | | |
