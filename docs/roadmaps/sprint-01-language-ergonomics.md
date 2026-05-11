# Sprint 01: Language Ergonomics Kickoff

Status: in-progress draft
Duration: 1 week
Goal: Start the v2 ergonomics line with maximum leverage and minimal runtime churn.

## Sprint Outcome

Deliver three foundations:

1. Compact conditional syntax (if/else) lowered to flow.branch.
2. Enum declarations in grammar + ast + parser + minimal verifier wiring.
3. LSP quick-action scaffold for missing branch handler generation.

## Scope

### A) Compact Conditional Syntax

Target syntax:

```grapheme
if $current.attempt >= $current.max_attempts then MarkTimeout else EvaluateSignal
```

Compiler behavior:

1. Parse and lower to the same internal branch representation used by flow.branch.
2. Preserve existing flow.branch support unchanged.

Acceptance:

1. Existing branch tests still pass.
2. New parse/lower tests validate if/else sugar.
3. Runtime behavior is equivalent to flow.branch form.

### B) Enums (Foundation)

Target syntax:

```grapheme
enum JobStatus { queued, running, blocked, done, timeout }
```

Compiler behavior:

1. Parse enum definitions and register named enum symbols.
2. Allow enum members in equality branch comparisons.
3. Emit clear diagnostics for unknown enum members.

Acceptance:

1. Parse tests for enum declarations pass.
2. Verifier catches unknown members.
3. No regressions in existing struct/type features.

### C) LSP Quick Action Scaffold

Target behavior:

1. Detect branch target names that are missing iterators.
2. Offer code action: Generate missing iterator skeleton.
3. Insert iterator stubs at end of file with inferred typed signature when possible.

Acceptance:

1. Code action appears on missing target diagnostics.
2. Inserted skeleton compiles after fill-in.
3. No hover/completion regression.

## Engineering Plan

1. Day 1-2: grammar + ast + parser for if/else and enum.
2. Day 3: hir + verifier updates and tests.
3. Day 4: lsp diagnostics + code action plumbing.
4. Day 5: docs + showcase example + stabilization.

## Files Expected To Change

- crates/grapheme-compiler/src/grapheme.pest
- crates/grapheme-compiler/src/ast.rs
- crates/grapheme-compiler/src/parser.rs
- crates/grapheme-compiler/src/hir.rs
- crates/grapheme-compiler/src/verifier.rs
- crates/grapheme-compiler/src/lib.rs
- crates/grapheme-lsp/src/main.rs
- examples/showcase/job-coordinator-lifecycle.gr

## Test Plan

1. cargo test -p grapheme-compiler
2. cargo check -p grapheme-lsp -p grapheme
3. runtime smoke for updated showcase

## Out of Scope (Sprint 01)

1. Full state-machine syntax.
2. Retry/timeout directives.
3. Exhaustiveness and reachability graph checks.
4. Capability schema registry.

## Exit Criteria

1. All new compiler and lsp tests pass.
2. Existing showcase files still run.
3. One updated advanced showcase uses if/else sugar and enum status values.
