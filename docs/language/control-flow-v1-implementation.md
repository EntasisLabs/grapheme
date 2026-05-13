# Language Control Flow v1 Implementation Plan

This is the implementation checklist for the approved v1 control-flow scope.

Related proposal: `docs/language/control-flow-v1.md`

## V1.5 Progress Snapshot

Implemented:

1. Internal loop frame scaffold in runtime.
2. Bounded trace retention policy with minimal projection defaults.
3. Loop merge metadata propagated through compiler -> MIR -> runtime.
4. Runtime merge behavior implemented for `replace|append|reduce|none`.
5. Compiler verifier accepts and validates `@loop(merge: ...)` values.
6. Runtime conformance tests for merge semantics and trace budget behavior.

Examples:

1. `examples/fixtures/v1-loop-merge-append.gr`
2. `examples/fixtures/v1-loop-merge-reduce.gr`
3. `examples/fixtures/v1-loop-merge-none.gr`

## V1 Frozen Scope

Implement in v1:

1. Iterator loop directives (with `fragment` as compatibility alias):
   - `@loop(max: <int>)`
   - `@loop(max: <int>, until: { field: <string>, eq: <value> })`
   - optional collection mode: `@loop(max: <int>, each: <value>)`
2. Fragment invocation as pipeline step:
   - `|> PollJob`
3. Bounded recursion directive:
   - `@recursive(max_depth: <int>)`
4. Call step:
   - `call Step { ... }`
5. Runtime limits:
   - loop iteration cap
   - recursion depth cap

Defer to v1.5/2:

1. Fluent chain syntax (`.then`, `.until`, `.select`)
2. Full expression language in `until`/`each`
3. Native streaming subscription scheduler

## Canonical Semantics

### Fragment Loop

1. If an iterator (or fragment alias) has `@loop(max: N)`, its body is re-executed up to `N` times.
2. `until` is evaluated after each iteration against `state.current`.
3. If `until` becomes true, loop exits early.
4. If `until` is absent, exactly `N` iterations run.
5. If `each` is set, fragment executes once per item (up to `max`) with the item bound as current input for that iteration.

### Fragment Invocation in Pipeline

1. `|> IteratorName` passes current state as iterator input.
2. Fragment final current is returned to caller pipeline.
3. If fragment fails, caller pipeline fails.

### Recursion

1. `@recursive(max_depth: D)` required for self-recursive fragments.
2. Runtime tracks call depth per execution.
3. Exceeding `D` is fatal with explicit error code.

## File-by-File Checklist

### 1) Grammar

File: `crates/grapheme-compiler/src/grapheme.pest`

Tasks:

1. Add explicit `call_step` grammar for `call <ident> ...` in executable bodies.
2. Ensure directive arg values can represent `until` object form and `each` value form.
3. Keep backward compatibility for existing field calls.

Acceptance:

1. Existing examples still parse.
2. New `call Step { ... }` form parses.

### 2) AST

File: `crates/grapheme-compiler/src/ast.rs`

Tasks:

1. Add AST node for call step (`CallStep`).
2. Add AST shape for executable body entries that can contain both field calls and call steps.
3. Keep directives attached to fragments and executable defs.

Acceptance:

1. AST round-trip includes call steps and directives.

### 3) Parser

File: `crates/grapheme-compiler/src/parser.rs`

Tasks:

1. Parse `call_step` into AST.
2. Parse `@loop` and `@recursive` directive args into typed helper structs in lowering/verifier stage (AST can stay generic directive list).
3. Preserve current parse behavior for existing syntax.

Acceptance:

1. Parser handles all new v1 examples and existing examples.

### 4) HIR Lowering

File: `crates/grapheme-compiler/src/hir.rs`

Tasks:

1. Introduce HIR step kinds for:
   - capability call
   - call target invocation
2. Preserve capability collection behavior for call steps (if represented as synthetic capabilities, define naming convention).
3. Carry fragment directives metadata needed by runtime.

Acceptance:

1. HIR encodes loop/recursive metadata and call steps explicitly.

### 5) Verifier

File: `crates/grapheme-compiler/src/verifier.rs`

Tasks:

1. Validate `@loop(max)` exists and is positive integer.
2. Validate `until` object shape (`field`, `eq`) when present.
3. Validate `@recursive(max_depth)` positive integer.
4. Validate `call` targets exist.
5. Validate self-recursive fragments require `@recursive`.

Acceptance:

1. Invalid directives/call targets fail at compile-time with clear errors.

### 6) MIR Lowering

File: `crates/grapheme-compiler/src/mir_lower.rs`

Tasks:

1. Represent call steps in MIR (new instruction variant or synthetic call convention).
2. Ensure fragment directive metadata is available to runtime execution plan.

Acceptance:

1. MIR includes enough info for runtime to execute loops and recursion deterministically.

### 7) Artifact Types

File: `crates/grapheme-artifact/src/artifact.rs`

Tasks:

1. Extend MIR instruction schema if needed for call/control flow.
2. Preserve artifact integrity hashing compatibility.

Acceptance:

1. Artifact serialize/deserialize remains stable.

### 8) Runtime Execution

File: `crates/grapheme-runtime/src/runtime.rs`

Tasks:

1. Add call frame stack and depth tracking.
2. Execute fragment invocations from pipeline steps.
3. Apply `@loop` semantics when entering fragment with loop directive.
4. Enforce recursion depth cap and loop iteration cap.
5. Preserve existing policy checks for each inner call.

Acceptance:

1. Pipe into fragment loop and pipe out behavior matches contract.
2. Recursion stops at configured depth with deterministic failure code.

### 9) State/Trace

Files:

- `crates/grapheme-runtime/src/state.rs`
- `crates/grapheme-artifact/src/artifact.rs` (trace shape if needed)

Tasks:

1. Add loop/call metadata to pipeline trace entries:
   - `loop_id`
   - `iteration_index`
   - `call_depth`
   - `call_target`

Acceptance:

1. JSON execution output exposes enough control-flow diagnostics.

### 10) Examples

Files to add:

1. `examples/fixtures/v1-loop-fragment-poll.gr`
2. `examples/fixtures/v1-nested-loop-jobs.gr`
3. `examples/fixtures/v1-recursive-step.gr`
4. `examples/fixtures/v1-recursive-depth-error.gr`

Acceptance:

1. First three run successfully.
2. Depth-error example fails with expected code/message.

## Suggested Delivery Order

1. Grammar + AST + parser for call step
2. HIR/MIR representation for call step
3. Runtime call frame execution
4. Loop directive semantics
5. Recursion depth enforcement
6. Trace metadata
7. Examples and docs polish

## Done Criteria

v1 is complete when:

1. All v1 examples compile and run as specified.
2. Loop and recursion limits are enforced.
3. Existing examples remain compatible.
4. Compiler and runtime errors are deterministic and actionable.