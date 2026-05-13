# Rustdoc and Docs.rs Readiness

Status: active
Scope: all crates in `crates/`

This checklist defines the minimum documentation quality bar for publishing and sustaining Rust API quality.

## 1. Crate-Level Documentation

- [x] Every crate has crate-level rustdoc (`//!`) describing purpose and scope.
- [x] Crate docs describe primary entrypoints or usage intent.

## 2. Public API Surface Documentation

- [x] Public API entry surfaces are documented for:
  - `grapheme-artifact`
  - `grapheme-compiler`
  - `grapheme-runtime`
  - `grapheme-sdk`
- [x] Core public types include field-level docs where contract semantics matter.
- [x] Public methods in runtime/SDK/compiler top-level APIs have behavior-oriented docs.

## 3. Build Verification

- [x] Workspace docs build succeeds locally via:

```bash
cargo doc --workspace --no-deps
```

- [x] CI includes rustdoc build verification in conformance workflow.

## 4. Ongoing Governance Rules

1. Any new public type/function must include rustdoc in the same PR.
2. Any behavior change to a documented API must update rustdoc in the same PR.
3. Any new crate added under `crates/` must include crate-level `//!` docs before merge.
4. `cargo doc --workspace --no-deps` must remain green in CI.

## 5. Recommended Next Tightening (Optional)

1. Enable `#![warn(missing_docs)]` incrementally per crate once documentation debt reaches zero.
2. Add `/// # Examples` sections for top 5 SDK and runtime APIs used by embedders.
3. Add a lightweight docs link check for markdown references in CI.
