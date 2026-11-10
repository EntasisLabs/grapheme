# Release 0.8.0 — GraphemeUI language surface (RFC-0007)

Status: prep
Target tag: **v0.8.0**
Created: 2026-11-10

## Scope

Ship the post-0.7.2 train that landed on `main` for RFC-0007 rollout steps 0–2 and Part H:

1. **Part J fixes** — `transition` writes the root field; partial typed patches; Int-preserving `inc_field` / `dec_field`
2. **Part B** — value expressions, expression type checking, `.count`, expression query bodies
3. **Part A** — state stores, lane-less `apply`, revisions / runtime + history diffs, `grapheme session`
4. **Parts C / D / I** — views, View IR (`--emit view-ir`), headless `render` / `invoke_action` with binding epochs
5. **Part H / I.2** — UI glyphs with `expose`, `StoreSession::mount` / `--mount`

## Pre-cut checklist

Use with `docs/internal/release/release-gates-and-doc-versioning.md`.

### Versions

- [x] Workspace language crates bumped to **0.8.0** (`grapheme-artifact` remains **0.3.0**)
- [x] VS Code extension `package.json` / lock → **0.8.0**
- [x] `CHANGELOG.md` `[0.8.0]` entry

### Docs / examples currency

- [x] Language contract covers stores, expressions, views, UI glyphs
- [x] CLI docs: `grapheme session`, `--emit view-ir`, `--render` / `--action` / `--mount`
- [x] SDK docs: `store_session`, `render`, `mount`, `invoke_action`
- [x] Canonical example: `examples/view-counter.gr`
- [x] Normative RFC: `docs/internal/rfc/rfc-0007-graphemeui-views-v1.md`
- [ ] Final human skim of product FAQ / tutorials before tag

### Gates (fill at cut time)

- [ ] `conformance` green on release PR / `main`
- [ ] `conformance-wasix` green (builds Stage B wasm first)
- [ ] `cargo doc --workspace --no-deps`
- [ ] Spot-check: `cargo run -p grapheme-cli -- session examples/view-counter.gr --mount --invoke Increment`
- [ ] Spot-check: `cargo run -p grapheme-cli -- compile examples/view-counter.gr --emit view-ir`
- [ ] Spot-check: `cargo test -p grapheme-sdk --lib`

### Cut sequence

1. Merge version-prep PR to `main`
2. Tag `v0.8.0` on the merge commit (`git tag -a v0.8.0 -m "v0.8.0"` + push tags)
3. Confirm `.github/workflows/release-lsp.yml` creates the GitHub Release and uploads all LSP + VSIX assets
4. Optional local fallback: `scripts/release-bundle.sh --tag v0.8.0`
5. Optional: `scripts/publish-crates.sh --publish` (dry-run first)
6. Point extension `grapheme.lsp.releaseTag` docs / users at `v0.8.0`
7. Review/edit the automatically generated GitHub Release notes against `CHANGELOG.md` `[0.8.0]`

### Explicit non-goals for 0.8.0

- Store lifetimes (`singleton` / `transient state`), registries, history / undo, hot reload
- `task` kind, subscriptions-on-stores, concurrency policies, effect receipts
- View styles, params, slots, `if` / `for`, `VirtualList`
- Shipping `grapheme-ui` / rndr inside this repo (external bridge, model A)
- Publishing plugin crates (`plugins/*-rs` remain excluded)

## Owners sign-off

| Gate | Owner | Sign-off |
| --- | --- | --- |
| Runtime / CLI / stores | | |
| Tooling (LSP / VSIX) | | |
| Docs | | |
