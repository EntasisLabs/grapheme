# Next Feedback Wave

This roadmap captures the latest language/tooling feedback and proposed rollout order.

## Scope From Feedback

1. Rename/align reusable unit keyword from `iterator` to `node`.
2. Allow more primitive-oriented ergonomics similar to struct usage.
3. Expand standard transformation/utilities for mapping, string ops, and general-purpose workflows.
4. Add composition sugar for resilience policies.
5. Add intent annotation that is visible in trace output.
6. Add cargo-like module discovery commands.

## Current Status

Implemented in this wave:

1. `node` keyword support as a non-breaking alias for `iterator`.
2. CLI modules discovery subcommands:
   - `grapheme modules search <query>`
   - `grapheme modules info <module>`
   - `grapheme modules types <module>`
   - `grapheme modules examples <module>`
3. `@resilient` directive sugar lowering to `@loop`, `@retry`, `@timeout`.

## Proposed Phases

### Phase 1: Compatibility and Discoverability (Done)

- Add `node` syntax alias with no behavior change.
- Extend `modules` CLI UX for search/info/types/examples.

### Phase 2: Resilience Composition Sugar (Done)

Target syntax (draft):

```grapheme
@resilient(
  loop: { max: 32 },
  retry: { max: 2, backoff_ms: 50, on_fail: Fallback },
  timeout: { ms: 2500, on_timeout: TimeoutPath }
)
```

Lowering plan:

- Expand `@resilient(...)` into `@loop(...)`, `@retry(...)`, `@timeout(...)` at compile time.
- Preserve existing verifier/runtime semantics and errors.
- Reject mixing conflicting nested and top-level directive values.

### Phase 3: Intent Annotation + Trace Surfacing

Target syntax (draft):

```grapheme
@intent(message: "why this workflow exists")
```

Behavior plan:

- Directive accepted on executable definitions and optionally on pipeline steps.
- Propagate intent string into MIR metadata.
- Emit intent in runtime trace entries and stream output.

### Phase 4: Primitive-Centric Authoring Ergonomics

Candidate items:

- Clarify and extend scalar/list signatures for `query`/`mutation`/`node`.
- Add explicit literal initialization guidance where struct init is not required.
- Add typed examples for scalar/list-first workflows.

### Phase 5: Standard Library Expansion

Priority utility additions:

- Vector/list: `map`, `flat_map`, `reduce`, `find`, `sort_by`, `group_by`.
- String: `split`, `join`, `replace`, `trim`, `lower`, `upper`, `contains`.
- Object/data: `keys`, `values`, `has`, `get_path`, `set_path`.

Delivery pattern:

- Add op manifests + verifier arg checks.
- Add native module implementations and examples.
- Add module docs and `grapheme modules examples <module>` mappings.

## Acceptance Gates

For each phase:

1. Compiler tests for parse/lower/verify behavior.
2. Runtime tests for new execution semantics.
3. At least one curated showcase or cookbook example.
4. Docs update in language contract and CLI reference.

## DX Sprint Plan (May 2026 Feedback)

This section captures the latest in-the-wild developer experience feedback and becomes the active execution plan for CLI + SDK DX.

### Feedback Summary

1. CLI is strong when it works, but first-use discovery requires too many attempts.
2. `modules search` needs explainable output, not just matches.
3. `examples` flow is not intuitive enough for quick inspect/run loops.
4. Type system coverage is still incomplete and should be finished before broader feature expansion.
5. CLI and SDK should converge so CLI is a thin wrapper over shared SDK command capabilities.

### Prioritized Workstreams

1. Examples UX first (fastest DX win).
2. Search explain output second.
3. Type system completion before major new surface area.
4. SDK command-capability extraction so CLI can become a thin adapter.

### Sprint 1: Examples UX Redesign (Active)

Goal:

- Make discover -> inspect -> run examples a single intuitive flow.

Deliverables:

1. Add metadata-backed examples index in CLI output (summary, tags, complexity, run hint).
2. Improve `examples list` for quick decision-making.
3. Improve `examples show` with quick summary/how-to-use guidance.
4. Improve `examples init` completion output with explicit next steps.

Acceptance Criteria:

1. New user can identify a relevant example within one `examples list` run.
2. `examples show` explains when/how to use an example without requiring docs lookup.
3. Scaffold flow (`examples init`) prints actionable follow-up commands.
4. CLI reference documents all new flags and output modes.

Owner:

- CLI DX owner

Checklist:

1. [x] Add bundled example metadata model in `crates/grapheme-cli/src/main.rs`.
2. [x] Add structured output (`--yaml|--json`) for examples discovery views.
3. [x] Add quick-look mode for `examples show`.
4. [x] Add tests for examples metadata rendering and flag handling.
5. [x] Update `docs/cli.md` examples section.
6. [x] Add examples list filtering (`--query`, `--tag`, `--complexity`, `--native-only`).
7. [x] Add richer `examples show` next-step run guidance.

### Sprint 2: Modules Search Explainability

Goal:

- Return matching modules/ops with practical guidance for when to use each.

Deliverables:

1. Extend `modules search` output to include: `why_matched`, `summary`, `use_when`, `avoid_when`, `related_examples`.
2. Keep machine-friendly YAML/JSON output for agent tooling.
3. Add an explicit explain mode flag to preserve backward compatibility for minimal output use cases.

Acceptance Criteria:

1. Search output includes both match and usage guidance.
2. Output remains deterministic and parseable in JSON and YAML.
3. Tests lock output contract shape.

Progress update (2026-05-16):

1. Implemented `grapheme modules search <query> --explain` with guidance fields:
  `why_matched`, `summary`, `use_when`, `avoid_when`, `related_examples`.
2. Added relevance `score` and explain detail tiers (`--detail concise|full`) for ranking-friendly output.
3. Added parser and payload tests for explain mode contract shape.
4. Kept default `modules search` output backward compatible.

### Sprint 3: Type System Completion

Goal:

- Move type verification from partial coverage to consistent contract-level confidence.

Deliverables:

1. Complete op signature coverage in verifier-facing type tables.
2. Improve branch/loop typing and merge-shape checks.
3. Improve compile-time type errors with actionable diagnostics.
4. Add strictness levels (`warn` and `strict`) for rollout safety.

Acceptance Criteria:

1. Known-module type errors are deterministic and actionable.
2. `modules types` reflects verifier-enforced behavior for covered modules.
3. New type checks are covered by compiler/runtime tests and docs updates.

Status assessment (2026-05-16):

1. Core verifier signature checks are live for known module ops (required args + basic arg-type checks).
2. Type/lint policy modes exist for executable-kind write boundaries (`Compatibility` warning vs `StrictMutationOnly` error), but this policy is not yet exposed as a first-class CLI strictness toggle.
3. Branch/match flow checks and typed field access checks exist, including state-machine transition validation.
4. `modules types` currently reflects runtime manifest/export metadata, not a strict verifier-contract coverage report.

Highest-value gaps to close first:

1. Reject unknown args for known ops in verifier (`core.foo(extra: 1)` should fail deterministically).
2. Add verifier strictness profile flag wiring in CLI/SDK compile path (`warn` vs `strict`) for rollout-safe enforcement.
3. Add contract snapshots for diagnostic codes/messages around arg-shape errors and strictness mode behavior.
4. Add coverage parity checks so `modules types` can report/verifiably align with verifier-enforced op signatures.

Recommended execution order (high value -> lower value):

1. Unknown-arg rejection + tests (fastest quality jump, low migration risk).
2. CLI/SDK strictness mode plumbing + tests (lets teams adopt strict mode intentionally).
3. Diagnostic contract snapshots for repair-loop stability.
4. `modules types` verifier-coverage alignment and docs updates.

Progress update (2026-05-16):

1. Added verifier rejection for unknown args on known ops with deterministic diagnostics that include allowed args.
2. Added CLI `--type-policy warn|strict` support for both `compile` and `run` command paths.
3. Wired SDK engine compile helpers to accept compiler options via builder so strictness policy is preserved in source->AOT execution flows.
4. Added parser/tests and smoke validation for strict vs warn behavior.

### Sprint 4: SDK Command Surface + CLI Thin Wrapper

Goal:

- Expose all CLI capabilities through SDK APIs and make CLI a formatting/arg-parsing adapter.

Deliverables:

1. Introduce SDK command capability interfaces for parse/compile/build/run/modules/examples.
2. Move CLI business logic into SDK command services.
3. Keep CLI output parity via adapter layer.
4. Add parity tests between CLI and SDK command outputs.

Acceptance Criteria:

1. Third-party tooling can call SDK directly for all current CLI command capabilities.
2. CLI behavior remains backward compatible for existing command contracts.
3. Golden contract tests validate SDK/CLI parity.

Progress update (2026-05-16):

1. Extracted module discovery/search logic into SDK APIs:
  `discover_module_manifests`, `curated_examples_for_module`, and `modules_search_payload`.
2. Expanded SDK extraction for modules payload builders:
  `modules_ops_payload`, `modules_types_payload`, and `modules_examples_payload`.
3. CLI `modules` discovery/search/ops/types/examples paths now call SDK APIs (thin-wrapper direction in progress).
