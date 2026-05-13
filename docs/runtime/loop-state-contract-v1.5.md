# Loop State Contract v1.5

Status: design decision draft for implementation.

Goal: make loop execution feel like a true programming language runtime by default:

1. fast execution
2. bounded memory
3. deterministic semantics
4. per-iteration visibility on demand

## Problem Statement

Current loop behavior mixes three concerns into one state flow:

1. execution state for computing next step
2. language-visible state semantics
3. debug and agent trace retention

When these concerns are fused, every iteration pays both execution and observability costs.

## Core Model

Runtime state is split into three layers.

### 1) ExecFrame

- minimal mutable frame used by instruction execution
- scoped per call and per loop frame
- updated in place
- not retained as history

### 2) ProgramState

- language-visible state semantics (`state { ... }`)
- updated at semantic boundaries
- source of truth for correctness

### 3) TraceState

- diagnostics and agent introspection only
- policy-bounded retention and projection
- never required for correctness

## Loop As First-Class Frame

Each loop creates a `LoopFrame`:

1. `input`: entry snapshot
2. `current`: mutable iteration value
3. `accumulator`: optional aggregate sink
4. `meta`: iteration index, max, exit flags

Execution order:

1. initialize frame
2. run loop body against frame current
3. evaluate until predicate against frame current
4. stop on until true or max reached
5. merge once back to caller state

## Merge Semantics

Loop exit must use explicit merge behavior.

Supported modes:

1. `replace`: caller current becomes loop current
2. `append`: caller receives collected iteration outputs
3. `reduce`: caller receives accumulator result
4. `none`: no state merge (side-effect loop)

Default in v1.5: `replace`

## Trace Retention Policy

Visibility is decoupled from execution state.

Retention:

1. `none`
2. `last(N)`
3. `sampled(K)`
4. `full` (debug only)

Projection:

1. `minimal`
2. `fields([...])`
3. `full`

Default in v1.5:

- retain: `last(20)`
- project: `minimal`

## Runtime Invariants

Implementation must preserve:

1. deterministic outcomes for same input and policy
2. bounded memory by retention policy
3. constant-overhead hot-path mutation
4. merge-once semantics at loop exit
5. trace off does not change result

## Error Semantics

Canonical loop-related outcome codes:

1. `LOOP_MAX_REACHED`
2. `LOOP_UNTIL_EVAL_ERROR`
3. `LOOP_MERGE_ERROR`
4. `LOOP_BODY_FATAL`

## What We Explicitly Reject

1. unbounded per-iteration history by default
2. trace retention as part of correctness semantics
3. implicit merge behavior that varies by module output shape
4. global mutable state writes from plugin modules

## Migration Plan

1. introduce internal loop frame and retention policy without syntax break
2. add merge metadata to compiler and MIR
3. expose user-facing merge and trace controls
4. enforce budgets and conformance tests

## Related Docs

- `docs/runtime/runtime-state-flow.md`
- `docs/runtime/wit-contract-v1.5.md`
- `docs/language/control-flow-v1-implementation.md`
