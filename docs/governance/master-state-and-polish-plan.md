# Grapheme Master State and Polish Plan

Status: phases 1-4 completed (2026-05-12)
Owner: core maintainers (compiler, runtime, sdk, cli, lsp, docs)
Objective: move Grapheme from feature-delivery momentum to a documented, release-ready platform posture.

## 0. Executive View (Read This First)

This document is complete, but most day-to-day readers should only use this section.

Current posture:

1. Documentation correctness and link integrity: complete.
2. Rustdoc-first API documentation baseline: complete.
3. Product narrative and persona onboarding paths: complete.
4. Release governance and docs versioning policy: complete.

Canonical operational docs (primary references):

1. `docs/README.md` (entry index + persona routes)
2. `docs/cli.md` (user-facing command contract)
3. `docs/governance/rustdoc-readiness.md` (Rust API documentation policy)
4. `docs/release/release-gates-and-doc-versioning.md` (release governance)

How to use this file:

1. Use sections 0 and 5 for status and phase history.
2. Use sections 7 and 8 for governance and done criteria.
3. Treat the rest as audit/reference context.

Execution snapshot:

1. Phase 1 complete: docs correctness and link integrity updates landed.
2. Phase 2 complete: rustdoc-first baseline and CI rustdoc verification landed.
3. Phase 3 complete: product narrative alignment and persona-first navigation landed.
4. Phase 4 complete: release gates, docs drift PR checklist process, and versioned docs policy landed.

## 1. Scope and Review Method

This document is a full-project governance baseline covering:

1. Repository structure and component readiness.
2. Documentation quality and navigability.
3. Example suite quality and discoverability.
4. CLI/SDK/LSP release and conformance posture.
5. Rust documentation quality (rustdoc-first standard).

Review method used:

1. Repository inventory of crates, docs, examples, release scripts, and CI workflows.
2. Cross-check of stated docs guidance against current implementation and file layout.
3. Gap classification into correctness, discoverability, maintainability, and release-risk categories.

## 2. Current State Inventory

### 2.1 Core Platform

Current workspace components are cleanly separated and aligned to platform layers:

1. Language and lowering: `crates/grapheme-compiler`.
2. Artifact and AOT contracts: `crates/grapheme-artifact`.
3. Runtime execution and policy: `crates/grapheme-runtime`.
4. Embedded API surface: `crates/grapheme-sdk`.
5. User-facing command surface: `crates/grapheme-cli`.
6. Tooling/editor integration: `crates/grapheme-lsp` and `extensions/grapheme-vscode`.
7. Capability metadata and std helpers: `crates/grapheme-signatures`, `crates/grapheme-stdlib`.

Assessment:

1. Architecture split is strong and production-oriented.
2. AOT Stage A and Stage B scaffold work is integrated across compiler/runtime/sdk/cli.
3. Conformance workflow exists and includes AOT coverage lanes.

### 2.2 Documentation Set

Documentation breadth is high:

1. Foundational docs: getting started, architecture, language contract, runtime policy, modules.
2. Domain docs: runtime contracts, roadmaps, RFCs, LSP and release flows.
3. Product docs: root README, docs index, examples index, extension README.

Assessment:

1. Coverage exists for most major surfaces.
2. Quality is not yet uniform across documents.
3. Several navigation and drift issues reduce trust in docs as a source of truth.

### 2.3 Examples and Demonstrations

The canonical examples set is healthy and broad for current capability scope.

Assessment:

1. Root `examples/` has a practical set of runnable `.gr` scenarios.
2. Legacy material is preserved under `examples/legacy/`.
3. Some docs still reference pre-move paths (for example showcase/cookbook roots), creating dead-link risk.

### 2.4 Tooling and Release

Current release and conformance scaffolding is in place:

1. `.github/workflows/conformance.yml` includes major contract checks.
2. `.github/workflows/release-lsp.yml` supports LSP release flow.
3. `scripts/release-lsp.sh` and `scripts/release-bundle.sh` exist for release operations.

Assessment:

1. Good baseline for engineering confidence.
2. Documentation and CLI contract sync needs hardening before claiming full release polish.

## 3. Key Gaps and Risks

This section captures non-code functional risk (documentation/process/tooling quality).

### 3.1 Documentation Drift (High)

Observed issues:

1. CLI docs are stale relative to implemented AOT/build/strict execution flags.
2. Docs index and root README contain example links that no longer match actual example locations.
3. Some docs still imply old showcase/cookbook root paths while files now live under legacy paths.

Impact:

1. Onboarding friction.
2. Broken quickstart commands.
3. Erosion of operator confidence in docs.

### 3.2 Naming and File-Type Messaging Inconsistency (Medium)

Observed issues:

1. Extension README repeats/duplicates language extension notes in a confusing way.
2. Cross-doc language around supported file suffixes is inconsistent.

Impact:

1. User confusion in editor setup.
2. Increased support burden and false bug reports.

### 3.3 Rustdoc Posture Is Not Yet First-Class (High)

Observed issues:

1. Several public API surfaces are exported without consistent rustdoc comments.
2. Crate-level documentation is uneven across crates.
3. No explicit docs.rs readiness checklist is captured as a project standard.

Impact:

1. Weak discoverability for embed users.
2. Harder API adoption for external Rust developers.
3. Risk of accidental API behavior drift without documented contracts.

### 3.4 Governance and Acceptance Criteria Fragmentation (Medium)

Observed issues:

1. Roadmap acceptance criteria exist, but doc-quality acceptance criteria are not centralized.
2. No single execution board for doc cleanup and release-quality gates.

Impact:

1. Work can ship functionally complete but operationally inconsistent.
2. Quality debt can silently grow between delivery waves.

## 4. Target Standard (Microsoft-Style Procedural Posture)

Grapheme should enforce one authoritative rule set:

1. Every user-facing command appears in exactly one canonical reference and is validated by smoke commands.
2. Every major component has a single source-of-truth document and clear cross-links.
3. Every public Rust API surface is documented at crate level and symbol level.
4. Every release lane has explicit gate criteria and owner sign-off.

Operational quality bar:

1. Command correctness: all docs commands execute or are marked as illustrative.
2. Link correctness: no dead internal links in docs and README surfaces.
3. Contract correctness: docs reflect current CLI/runtime/sdk behavior in the same cycle as changes.
4. Tooling consistency: extension/LSP/readme language and capability statements match implementation.

## 5. Phased Execution Plan

### Phase 0: Baseline Freeze and Ownership (1-2 days)

Deliverables:

1. Publish this document as the active polish charter.
2. Assign one owner per workstream: docs, examples, cli references, rustdoc, release pipeline.
3. Establish weekly quality review checkpoint until closure.

Acceptance criteria:

1. Owners and due dates listed in the project board.
2. Every listed gap mapped to a tracked item.

### Phase 1: Correctness and Link Integrity (2-4 days) - Complete (2026-05-12)

Deliverables:

1. Align `docs/cli.md` with implemented CLI surface:
   parse, compile, run, build, modules, plugin flows, AOT flags, Stage B strict/fallback behavior.
2. Fix docs index and root README links to actual examples layout.
3. Normalize examples references to canonical vs legacy paths.

Acceptance criteria:

1. No broken internal links in top-level docs (`README.md`, `docs/README.md`, `examples/README.md`).
2. CLI reference commands map directly to current parser options.
3. A new contributor can run the documented happy-path without path errors.

Completion notes:

1. CLI docs were aligned to shipped command/flag surface.
2. Stale showcase/cookbook references were normalized to current paths.
3. Root/docs/example navigation references were corrected.

### Phase 2: Rustdoc-First Upgrade (3-5 days) - Complete (2026-05-12)

Deliverables:

1. Add crate-level `//!` documentation for every crate in `crates/`.
2. Add `///` docs to all public types and functions in SDK, artifact, compiler API, and runtime API surfaces.
3. Add usage examples in rustdoc for primary entrypoints.
4. Add docs build verification in CI (`cargo doc --workspace --no-deps`).

Acceptance criteria:

1. Core public APIs have human-readable docs and examples.
2. `cargo doc` succeeds in CI.
3. docs.rs readiness checklist is complete and documented.

Completion notes:

1. Crate-level rustdoc (`//!`) was added across crates.
2. Public API docs were added on core artifact/compiler/runtime/sdk entry surfaces.
3. CI now verifies `cargo doc --workspace --no-deps`.
4. Rustdoc/docs.rs readiness checklist was published.

### Phase 3: Product Narrative and Operator Guidance (2-3 days) - Complete (2026-05-12)

Deliverables:

1. Unify product narrative across root README, getting started, architecture, and sdk docs.
2. Add clear personas: language author, runtime operator, SDK embedder, extension user.
3. Add "choose your path" quick links in root docs.

Acceptance criteria:

1. Each persona can find a complete start-to-success path in <= 3 clicks.
2. Duplicate/conflicting product statements removed.

Completion notes:

1. Core docs were aligned to a single platform narrative.
2. Persona-based "choose your path" navigation was added for key audiences.
3. Docs index now includes persona entry points for faster onboarding.

### Phase 4: Release Gate Formalization (2-3 days) - Complete (2026-05-12)

Deliverables:

1. Publish release gate checklist:
   docs up to date, conformance green, extension notes synced, CLI snapshots verified.
2. Add a docs drift check process to PR template/review checklist.
3. Define versioned documentation update policy tied to release branches/tags.

Acceptance criteria:

1. No release proceeds without explicit gate sign-off.
2. Docs updates are required for user-visible behavior changes.

Completion notes:

1. Canonical release gates and docs versioning policy were published.
2. PR template now enforces docs-drift checks for user-visible changes.
3. Release guidance/docs index were linked to governance policy.

## 6. Workstreams and Owners

### Workstream A: Docs Contract Accuracy

Scope:

1. `README.md`
2. `docs/README.md`
3. `docs/cli.md`
4. `docs/getting-started.md`
5. `docs/sdk.md`

Primary outcomes:

1. Correct command surfaces.
2. Correct path/link references.
3. Unified terminology for AOT stages and runtime policy behavior.

### Workstream B: Examples Information Architecture

Scope:

1. `examples/README.md`
2. Legacy index notes under `examples/legacy/`
3. All docs referencing showcase/cookbook paths.

Primary outcomes:

1. Canonical vs legacy split is explicit and consistent.
2. No reference points to moved directories.

### Workstream C: Rustdoc and API Publishing

Scope:

1. All crates in `crates/`.
2. Public surfaces in `grapheme-sdk`, `grapheme-artifact`, `grapheme-compiler`, `grapheme-runtime`.

Primary outcomes:

1. High-quality crate landing pages.
2. Symbol-level docs for all exposed API contracts.
3. Docs examples that compile or are clearly marked.

### Workstream D: Tooling and Release Consistency

Scope:

1. Extension readme and LSP quickstart alignment.
2. Release scripts and release docs consistency checks.
3. Conformance workflow documentation mapping.

Primary outcomes:

1. Editor experience docs accurately reflect LSP capabilities.
2. Release instructions are deterministic for maintainers.

## 7. Documentation Governance Rules

These rules are mandatory from this point forward:

1. Any user-visible CLI flag or behavior change must update `docs/cli.md` and relevant command examples in the same PR.
2. Any example path move must update `examples/README.md` plus all referenced docs in the same PR.
3. Any public SDK/runtime API addition must include rustdoc comments and at least one usage snippet.
4. Any release-process change must update `docs/release/*.md` and script comments if behavior changed.
5. Any new major doc must be linked from `docs/README.md`.

## 8. Definition of Done for Polish Wave

Polish wave is complete only when all conditions are true:

1. All Phase 1-4 acceptance criteria are satisfied.
2. Root docs navigation has no dead links.
3. CLI reference exactly mirrors shipping commands and flags.
4. Rustdoc generation is green in CI and core API docs are complete.
5. Release checklist is published and used for the next release cut.

## 9. Immediate Next Actions (Execution Queue)

1. Approve this master plan as canonical governance doc.
2. Open a "Polish Wave" project board with workstreams A-D and owners.
3. Execute Phase 1 first (docs correctness/link integrity) before further feature expansion.
4. Schedule Phase 2 rustdoc-first sprint immediately after Phase 1 closure.

## 10. Success Metrics

Track these metrics each week until closure:

1. Dead internal links: target 0.
2. Docs-command failure rate in smoke validation: target 0.
3. Public API rustdoc coverage (core crates): target 100% of exported symbols documented.
4. Release gate pass rate without follow-up hotfix docs PR: target 100%.
