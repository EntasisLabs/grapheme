# LLM-Native Language Contract v1

Status: draft candidate for v1.0 baseline
Owner: language + compiler + runtime + lsp
Scope: language semantics and tooling guarantees that improve LLM reasoning reliability

## 1. Purpose

This contract defines what Grapheme must guarantee to be considered LLM-native in production, not only feature-rich.

LLM-native here means:

1. Programs have predictable structure and effects.
2. Failures are machine-readable and easy to repair.
3. The same source behaves deterministically under bounded policy.
4. Tooling surfaces enough type/shape intent for one-shot generation to succeed.

## 2. Non-Goals

This contract does not require:

1. General-purpose Turing-complete semantics.
2. Unbounded execution without policy limits.
3. Immediate migration of all legacy scripts to strict semantics.

## 3. Normative Invariants

These are the required invariants for v1 strict LLM profile.

### 3.1 Canonical State Envelope

Runtime shape should normalize to:

1. `current.state`: lifecycle/control lane
2. `current.data`: payload lane
3. `current.meta`: trace and execution metadata lane
4. `current.error`: structured error lane

Rules:

1. Root-level write constructs are disallowed in strict profile.
2. Lane writes are explicit and auditable.
3. Helper ops may project lanes, but must preserve canonical lane meaning.

### 3.2 Host Return Envelope Normalization

Capability results should normalize to:

1. `data`
2. `meta`
3. `error`

Rules:

1. Runtime supports dual-read compatibility during migration.
2. Strict profile emits lint/error when a step reads legacy ad-hoc host fields directly without normalization.

### 3.3 Executable Kind Contracts

Executable kinds are behavioral contracts:

1. `query`: read/derive only, no lane writes.
2. `mutation`: explicit lane writes via `apply state` or `apply data`.
3. `iterator` or `node`: orchestration/control flow; write by delegating to mutation.
4. `subscription`: contract defined as stream/event oriented; current equivalence to query is transitional.

### 3.4 Explicit Mutation Boundary

Mutation intent is explicit in syntax and verifier behavior:

1. `apply state { ... }` and `apply data { ... }` are mutation-only constructs.
2. Write-like compatibility ops outside mutation emit lint in compatibility mode and fail in strict mode.

### 3.5 Variable and Scope Determinism

Variable references are not string interpolation tokens.

Rules:

1. Variable references are typed AST values.
2. Lexical scope and shadowing rules are deterministic.
3. Undefined variables fail with stable diagnostic codes.

### 3.6 Deterministic Bounded Execution

The runtime must preserve deterministic outcomes under equivalent inputs/policy.

Rules:

1. Step budget and call-depth budget are first-class and enforced.
2. Execution traces are stable enough for replay-based debugging.
3. Capability and policy checks happen before dispatch.

### 3.7 Typed Contract Progression

Typed records and executable signatures are part of LLM-native ergonomics.

Rules:

1. Typed mode is gradual, then promoted for new examples/templates.
2. Typed field access violations are compile-time diagnostics in typed scopes.
3. LSP completion/hover/signature help prioritize typed contracts.

## 4. Profile Model

### 4.1 Compatibility Profile

Purpose:

1. Preserve legacy behavior.
2. Surface migration guidance with structured lints.

### 4.2 Strict LLM Profile

Purpose:

1. Enforce invariants from section 3.
2. Optimize for one-shot generation correctness and low-retry repair loops.

## 5. Acceptance Matrix

Each invariant needs concrete acceptance tests by surface.

### 5.1 Compiler

Must verify:

1. Kind contract enforcement by profile.
2. Lane write restrictions (`apply`) by executable kind.
3. Typed field access validity in typed scopes.
4. Stable diagnostic codes for common LLM repair loops (unknown field, illegal write, unresolved variable).
5. Structured lint output for compatibility profile.

Minimum test evidence:

1. Parse/lower tests for lane syntax and kind declarations.
2. Verifier tests for strict rejection and compatibility warnings.
3. Golden diagnostics snapshots for code/message stability.

### 5.2 Runtime

Must verify:

1. Canonical state lane behavior under step chaining.
2. Host return envelope normalization and migration compatibility.
3. Deterministic replay parity for representative workflows.
4. Policy-denied outcomes are structured and consistent.

Minimum test evidence:

1. Contract tests for normalized lane/envelope shape.
2. Replay parity tests for interpreted and AOT paths.
3. Policy denial tests with stable error payload shape.

### 5.3 LSP

Must verify:

1. Kind contract visibility in hover/signature help.
2. Typed field completion from declared/inferred record shape.
3. Diagnostics and quick-fix guidance for write violations and shape-clobber patterns.

Minimum test evidence:

1. Hover/completion fixture tests for typed files.
2. Rename/reference stability across executable kinds.
3. Diagnostic fixture tests for strict-kind violations.

### 5.4 CLI and SDK

Must verify:

1. Structured output includes lint warnings and machine-readable diagnostics.
2. Profile selection is explicit and reproducible.
3. Run/compile outputs remain stable for agent integration.

Minimum test evidence:

1. Snapshot tests for `run --json` and compile outputs.
2. Profile smoke tests in CI for compatibility vs strict behavior.

## 6. LLM-Native Quality Scoreboard

Release candidates should include metrics for:

1. One-shot compile success rate on curated prompts.
2. Automated repair success rate within one retry.
3. Deterministic replay parity rate.
4. Policy-correct execution rate on adversarial policy prompts.
5. Typed completion acceptance rate in editor-guided generation.

These metrics are release evidence, not aspirational notes.

## 7. Current Gap Snapshot (2026-05)

1. Strict kind enforcement exists, but compatibility remains default.
2. Shape-clobber lint is present, but dedicated LLM lint profile is not finalized.
3. Canonical lane and host envelope normalization are partially documented, not fully enforced.
4. Variable binding model is not yet final.
5. Typed records are in active proposal/partial implementation stage.

## 8. Rollout Criteria

v1 contract can be marked active when all are true:

1. Strict LLM profile is available and documented as preferred for new projects.
2. Canonical state and host envelopes are enforced or strongly linted with migration guidance.
3. Compiler/runtime/LSP acceptance suites are green in CI.
4. At least three canonical examples are authored in strict profile style.
5. Release notes include scoreboard metrics from section 6.
