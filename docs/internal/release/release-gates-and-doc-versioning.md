# Release Gates and Documentation Versioning Policy

Status: active
Scope: all Grapheme releases (CLI, runtime, SDK, LSP, VSIX)

This document is the authoritative release-governance checklist for shipping Grapheme artifacts.

## 1. Mandatory Release Gates

A release is eligible only when every gate below is marked complete.

### Gate A: CI and Conformance

- [ ] `conformance` workflow is green.
- [ ] `conformance-wasix` workflow is green.
- [ ] Rustdoc build verification is green (`cargo doc --workspace --no-deps`).

### Gate B: CLI and Runtime Contract Confidence

- [ ] CLI snapshot contracts are green (including AOT manifest snapshot tests).
- [ ] Runtime Stage B strict-mode contract tests are green.
- [ ] SDK strict-mode AOT contract tests are green.

### Gate C: Documentation Currency

- [ ] `README.md` reflects current command and path examples.
- [ ] `docs/internal/cli.md` reflects all user-visible CLI flags/commands for this release.
- [ ] `docs/internal/getting-started.md` and `docs/internal/sdk.md` still match current onboarding flow.
- [ ] `docs/internal/sdk-feature-flags.md` matches SDK/CLI feature matrix for capability modules.
- [ ] All changed user-visible behavior has corresponding docs updates in the same release scope.

### Gate D: Tooling and Distribution Notes

- [ ] `extensions/grapheme-vscode/README.md` is aligned with shipped extension behavior.
- [ ] `docs/internal/lsp/quickstart.md` and `docs/internal/release/lsp-release.md` are aligned with release flow.
- [ ] Expected LSP asset names and packaging assumptions are still correct.

### Gate E: Release Artifact Verification

- [ ] Release scripts used for the cut complete successfully (`scripts/release-lsp.sh`, `scripts/release-bundle.sh` as applicable).
- [ ] Final artifact names/paths match documented release expectations.
- [ ] Tag/version metadata is consistent across release notes and shipped assets.

## 2. Sign-Off Requirement

No release proceeds without explicit sign-off from:

1. Runtime/CLI owner (execution and contract gates)
2. Tooling owner (LSP/VSIX gates)
3. Documentation owner (docs currency and versioning gates)

Sign-off should be captured in the release PR description or release tracking issue.

## 3. PR-Level Docs Drift Process

Every PR with user-visible changes must satisfy the following before merge:

1. Author checks docs-impact in PR checklist.
2. Author updates affected docs in the same PR.
3. Reviewer validates command/path correctness in changed docs.
4. Reviewer blocks merge when docs updates are missing for behavior changes.

This process is enforced operationally via `.github/pull_request_template.md`.

## 4. Versioned Documentation Policy

Documentation is versioned alongside code and tied to release branches/tags.

### 4.1 Branch Policy

1. `main` contains next-release documentation.
2. `release/*` branches contain release-candidate documentation for that train.
3. Docs fixes required for a release must be merged into the corresponding `release/*` branch before cut.

### 4.2 Tag Policy

1. Every release tag (`vX.Y.Z`) is treated as a frozen docs snapshot for that version.
2. Any post-tag docs correction that changes release-critical behavior guidance requires a patch release (`vX.Y.Z+1`) or an explicit addendum in release notes.
3. Release notes must reference the docs state associated with the tag.

### 4.3 Change Coupling Rules

1. Any user-visible CLI/runtime/SDK behavior change must include docs changes in the same PR.
2. Any command/path rename must update all canonical docs references before merge.
3. Any LSP/extension behavior change must update both extension README and release guidance when relevant.

## 5. Minimum Release Evidence

For each release, keep these links in the release PR or issue:

1. Conformance workflow run URL
2. Wasix conformance run URL
3. Rustdoc build run URL
4. Release checklist completion notes
5. Final tag and artifact list
