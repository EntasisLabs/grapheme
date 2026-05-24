# S+ Readiness Matrix (Syntax, Naming, Capability, Health)

Status: internal working draft  
Last updated: 2026-05-24

## Purpose

This matrix translates the S+ goal ("official-language quality + maximal accessibility for non-coders and zero-shot LLMs") into checkable criteria with direct repository evidence.

## Scoring Scale

- 5 = S+ ready (production-trust, beginner-trust, and machine-trust)
- 4 = strong (minor polish gaps)
- 3 = viable but not yet S+
- 2 = emerging
- 1 = missing

## Current Snapshot

- Overall S+ readiness score: **4.0 / 5** (about **80 / 100**)
- Capability breadth: strong
- Naming consistency: strong
- Runtime/compiler reliability: strong
- Beginner first-run and contract-single-source trust: main gaps

## Evidence Baseline

- Canonical op signatures: 76 ops across 16 modules in [crates/grapheme-signatures/src/lib.rs](../../crates/grapheme-signatures/src/lib.rs)
- Runnable examples: 90 `.gr` examples under [examples](../../examples)
- Test surface markers: 235 `#[test]` markers across `crates` and `src`
- Recent health checks:
  - `cargo test -p grapheme-sdk --lib` => 38 passed
  - `cargo test -p grapheme-compiler --lib` => 61 passed
  - `cargo test -p grapheme-runtime --lib` => 35 passed
- CI conformance workflows present: [conformance.yml](../../.github/workflows/conformance.yml)
- Release governance policy present: [docs/release/release-gates-and-doc-versioning.md](../release/release-gates-and-doc-versioning.md)

## Matrix A: Core S+ Dimensions

| Area | S+ Bar | Current Evidence | Score (1-5) | Status | Checklist |
|---|---|---|---:|---|---|
| Syntax ergonomics | Minimal mental model, predictable semantics, low surprise | Rich DSL supports `glyph`, `query`, `mutation`, `iterator`, `node`, `fragment`, control-flow sugar, typed signatures in [crates/grapheme-compiler/src/grapheme.pest](../../crates/grapheme-compiler/src/grapheme.pest) | 4.0 | Powerful but dense | [ ] Reduce beginner surface into staged profile |
| Syntax single source of truth | One authoritative grammar/spec path | Canonical grammar now owned in [crates/grapheme-compiler/src/grapheme.pest](../../crates/grapheme-compiler/src/grapheme.pest), with contract enforcement in [scripts/check-grammar-sync.sh](../../scripts/check-grammar-sync.sh) | 4.5 | Strong | [x] |
| Naming conventions (ops/modules) | Uniform, memorable, composable names | Module names are uniform lowercase domains; ops in catalog match lowercase/snake_case style in [crates/grapheme-signatures/src/lib.rs](../../crates/grapheme-signatures/src/lib.rs) | 4.5 | Strong | [ ] Add formal naming ADR + lint |
| Capability breadth | Standard library covers common automation jobs | 76 ops / 16 modules covering core/io/http/websearch/sql/surreal/secrets/memory/etc. in [crates/grapheme-signatures/src/lib.rs](../../crates/grapheme-signatures/src/lib.rs) | 4.5 | Strong | [ ] Add high-level "goal ops" layer |
| Type and contract clarity | Explicit IO shapes + stable machine-readable contracts | Typed metadata and output-field contracts in signatures crate + module payload APIs in SDK | 4.0 | Strong base | [ ] Expand schema refs coverage and enforce completeness |
| Runtime safety defaults | Safe-by-default policies for side effects | Policy allow-list model documented in [docs/runtime-policy.md](../runtime-policy.md) and enforced by runtime tests | 4.0 | Strong | [ ] Add novice-safe presets and policy profiles |
| Tooling (CLI/LSP/SDK) | Excellent authoring + embedding + diagnostics | CLI, LSP, SDK surfaces present; LSP library entrypoints in [crates/grapheme-lsp/src/lib.rs](../../crates/grapheme-lsp/src/lib.rs) | 4.0 | Strong | [ ] Add LSP test harness and scenario tests |
| Docs quality (official feel) | Task-first guides, expected outputs, troubleshooting, consistency | Task-first onboarding and expected outputs now in [docs/getting-started.md](../getting-started.md), top-failure playbook in [docs/troubleshooting.md](../troubleshooting.md), and scenario pack in [docs/quality/scenario-playbooks-v1.md](../quality/scenario-playbooks-v1.md) | 4.1 | Strengthening | [x] Add v1 scenario playbook pack |
| Example pedagogy | Progressive examples with intent and output expectations | 90 examples and canonical index in [examples/README.md](../../examples/README.md) | 3.5 | Good breadth, light narrative | [ ] Add "intent/input/output/when-to-use" cards |
| Reliability and test depth | Broad automated confidence across compiler/runtime/sdk | Compiler/runtime/sdk suites are green; conformance workflow exists | 4.5 | Strong | [ ] Add workspace-level release test gate report artifact |
| Governance and release discipline | Versioned docs + release gate enforcement | Policy and checklists in [docs/release/release-gates-and-doc-versioning.md](../release/release-gates-and-doc-versioning.md) | 4.0 | Strong | [ ] Automate docs-drift checks in CI |

## Matrix B: "Next Python" Questions Coverage

Legend: ✅ answered now, 🟡 partially answered, ⛔ not answered yet

| # | Strategic Question | Coverage | Evidence | Confidence | Checklist |
|---:|---|---|---|---|---|
| 1 | Do we have a one-line mission? | ✅ | [README.md](../../README.md) | High | [x] |
| 2 | Can a new user succeed in under 10 minutes? | ✅ | Explicit 10-minute success flow with expected outputs in [docs/getting-started.md](../getting-started.md) | High | [x] |
| 3 | Top 5 weekly user jobs known and prioritized? | 🟡 | Hinted by examples/modules, not formalized | Medium | [ ] Publish top jobs list |
| 4 | Do we know the 20 ops that drive 80% usage? | 🟡 | Local opt-in telemetry now supports summarize + redacted export + weekly rollup cadence; ranking still needs broader sample volume | Medium | [x] Instrument usage telemetry |
| 5 | Which concepts can be removed? | ⛔ | No explicit simplification backlog | Low | [ ] Create language simplification RFC |
| 6 | Where beginners fail in first 15 minutes? | 🟡 | TTFS funnel events + failure stage counts are captured, exportable, and included in weekly rollups; live dashboarding is still pending | Medium | [x] Add v1 weekly telemetry rollup reporting |
| 7 | Top runtime errors and guided remediations defined? | ✅ | Top-10 remediation guide in [docs/troubleshooting.md](../troubleshooting.md) | High | [x] |
| 8 | Is there one canonical style? | ✅ | [examples/README.md](../../examples/README.md) conventions | High | [x] |
| 9 | Can docs drift from behavior? | ✅ | CI docs smoke checks + single-source grammar contract in [conformance.yml](../../.github/workflows/conformance.yml) | High | [x] |
| 10 | Are LLM defaults safe and predictable? | 🟡 | Structured contracts + policy model exist | Medium | [ ] Add explicit LLM-safe profile and prompt contracts |
| 11 | Can novices stay safe without expert burden? | 🟡 | Allow-list policies exist; novice UX presets missing | Medium | [ ] Add beginner policy presets |
| 12 | Is compatibility promise explicit? | 🟡 | Release/versioning policy exists | Medium | [ ] Publish semver + stability matrix by surface |
| 13 | Is production-ready defined by SLOs? | ⛔ | No SLO doc found | Low | [ ] Add runtime SLO/SLA targets |
| 14 | Is third-party module trust model explicit? | 🟡 | Governance/release gates exist | Medium | [ ] Add signed module trust tiers |
| 15 | Is migration from Python/JS documented? | ⛔ | No migration playbook found | Low | [ ] Add migration guides |
| 16 | Are defaults "pit of success" for network/secrets/data writes? | 🟡 | Policy controls exist | Medium | [ ] Add secure-by-default starter profile |
| 17 | Can users test deterministically with fixtures? | ✅ | Fixtures/examples present under [examples/fixtures](../../examples/fixtures) and test suites | High | [x] |
| 18 | Is debugging explainable to non-coders? | 🟡 | Trace and lint outputs exist in runtime/CLI | Medium | [ ] Add human-centric debug guide |
| 19 | Is governance model for language evolution explicit? | 🟡 | RFC/roadmap/governance folders exist under [docs](../README.md) | Medium | [ ] Add concise governance one-pager |
| 20 | Is adoption wedge clearly defined? | 🟡 | Positioning exists; wedge metric not explicit | Medium | [ ] Define ICP + wedge KPI |

### Coverage Count

- ✅ answered now: 6 / 20
- 🟡 partially answered: 10 / 20
- ⛔ not answered: 4 / 20

Interpretation: **16 / 20 are now answered or partially answered, with 6 fully closed**, which supports the hypothesis that we are closer than it appears.

## Matrix C: Naming Convention Audit

| Check | Current State | Status | Checklist |
|---|---|---|---|
| Module names lowercase domain nouns | `core`, `io`, `http`, `websearch`, `sql`, `surreal`, etc. | Strong | [x] |
| Operation names lowercase snake_case | Catalog scan shows consistent style in signatures list | Strong | [x] |
| Argument names mostly descriptive | `max_results`, `timeout_ms`, `thing_or_table`, etc. | Strong | [x] |
| Potential ambiguity/verbosity hotspots | Examples: `thing_or_table`, `get_secret_handle`, short math ops (`eq`, `lt`, `gt`) | Moderate | [ ] Publish preferred naming map and alias/deprecation rules |
| Stability metadata on names | Stability tags are now attached to operation payload rows and discovery outputs (`stable`, `experimental`, `deprecated`) | Strong | [x] |

## What Is Closest to S+ Already

1. Runtime/compiler/sdk reliability and contract discipline.
2. Capability breadth and composable DSL power.
3. Naming consistency across modules and operations.
4. CI conformance posture and release governance intent.

## What Blocks S+ Most Directly

1. Novice-safe defaults and policy presets (pit-of-success behavior for side effects).
2. Explicit compatibility and migration contracts (surface-level semver/stability matrix + migration guides).
3. Telemetry sample growth and dashboarding (increase shared report volume for stronger trend confidence).
4. Adoption wedge definition and top-jobs prioritization.

## 30-Day Checklist (High-Leverage)

- [x] Create canonical grammar source and remove/auto-generate duplicate grammar.
- [x] Publish "10-minute Grapheme" guide with expected outputs and failure remediations.
- [x] Add troubleshooting doc: top 10 failures with exact fix commands.
- [x] Add stability tags to module ops and surface in CLI + SDK payloads.
- [x] Add CI gate that verifies docs command snippets and grammar-contract alignment.
- [x] Define and publish S+ metrics: time-to-first-success, docs drift incidents, novice failure rate, policy-safe default adoption.

## Execution Update (2026-05-24)

Delivered:

1. 10-minute onboarding flow now includes explicit expected outcomes in `docs/getting-started.md`.
2. New top-10 failure remediation guide added at `docs/troubleshooting.md`.
3. New CI and local contract checks added:

- `scripts/check-grammar-sync.sh`
- `scripts/docs-smoke-checks.sh`
- wired into `.github/workflows/conformance.yml`

1. Grammar drift was detected and corrected during rollout.
2. Grammar architecture moved to single-source ownership:

- canonical grammar: `crates/grapheme-compiler/src/grapheme.pest`
- root parser now references canonical grammar directly
- duplicate `src/grapheme.pest` removed
3. S+ KPI contract published at `docs/quality/splus-metrics.md` with explicit targets for TTFS, docs drift, novice failure rate, and policy-safe default adoption.
4. Module operation stability tags are now surfaced in SDK/CLI module operation payloads (`stable`, `experimental`, `deprecated`).
5. Module discovery search now defaults to stable-preferred matching with explicit `--include-experimental` opt-in.

Remaining from this tranche:

1. Completed.
2. New local opt-in telemetry support added in CLI (`GRAPHEME_TELEMETRY`, JSONL sink, `grapheme telemetry summarize`).
3. Redacted shareable export added (`grapheme telemetry export`) with schema contract in `docs/quality/telemetry-export-schema.md`.
4. Weekly telemetry rollup workflow operationalized via `scripts/telemetry-weekly-rollup.sh` and runbook at `docs/quality/telemetry-reporting-cadence.md`.
5. Scenario-first learning depth expanded with v1 playbook pack at `docs/quality/scenario-playbooks-v1.md` and telemetry-linked weekly improvement loop.

## Suggested Owners

- Language/Syntax owner: grammar single-source + simplification backlog
- DX/Docs owner: onboarding + troubleshooting + narrative examples
- Runtime/Policy owner: safe presets + policy profile docs
- Tooling owner: CI contract checks + LSP scenario tests
- Product owner: adoption wedge definition + KPI telemetry
