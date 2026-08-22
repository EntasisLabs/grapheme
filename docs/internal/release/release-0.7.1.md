# Release 0.7.1 — Slim embedded SDK

Status: prep
Target tag: **v0.7.1**
Created: 2026-08-22

## Scope

Ship the embedded-runtime work needed by iOS and Wasm consumers:

1. `grapheme-sdk` `slim` feature profile with no host/AOT/Wasix dependency edge
2. Public host-module registration and registry configuration APIs
3. Per-call `state.current` seeding for embedded execution
4. Released-style 0.7.1 crate and VS Code extension versions

## Pre-cut checklist

### Versions

- [x] Workspace language crates bumped to **0.7.1** (`grapheme-artifact` remains **0.3.0**)
- [x] VS Code extension `package.json` / lock → **0.7.1**
- [x] `CHANGELOG.md` `[0.7.1]` entry

### Slim target gates

- [x] `cargo test -p grapheme-sdk --no-default-features --features slim`
- [x] `cargo check -p grapheme-sdk --no-default-features --features slim --target aarch64-apple-ios`
- [x] `cargo check -p grapheme-sdk --no-default-features --features slim --target aarch64-apple-ios-sim`
- [x] `cargo check -p grapheme-sdk --no-default-features --features slim --target x86_64-apple-ios`
- [x] `cargo check -p grapheme-sdk --no-default-features --features slim --target wasm32-unknown-unknown`

### Remaining cut gates

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo doc --workspace --no-deps`
- [ ] Publish crates in dependency order with `scripts/publish-crates.sh --publish`
- [ ] Tag the merge commit `v0.7.1` and verify the LSP/VSIX release assets

## Cut sequence

1. Merge the 0.7.1 release branch to `main`.
2. Publish the language crates in dependency order; wait for crates.io index visibility.
3. Tag `v0.7.1` on the merge commit.
4. Confirm the release workflow uploads all LSP and VSIX assets.
5. Verify a fresh consumer resolves `grapheme-sdk = { version = "0.7.1", default-features = false, features = ["slim"] }` without local patches.
