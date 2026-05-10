# Language Control Flow v1

This document proposes a first version of loop and recursion semantics for AgentQL.

Status: design proposal, not implemented yet.

Implementation checklist:

- `docs/language-control-flow-v1-implementation.md`

## Goals

1. Add useful iterative behavior for agent workflows.
2. Keep execution deterministic and policy-governed.
3. Avoid unbounded compute by default.
4. Fit the current compile pipeline (AST -> HIR -> MIR -> artifact -> runtime).

## Non-Goals for v1

1. Turing-complete general purpose programming.
2. Arbitrary graph recursion without limits.
3. Implicit infinite streams.
4. Hidden control flow that bypasses policy checks.

## Proposed Surface Syntax

The preferred user-facing syntax should be fluent chaining (similar to modern .NET LINQ style), while still lowering to explicit bounded control-flow instructions internally.

### Fluent Chain (Preferred Ergonomic Form)

```aql
query PollUntilReady {
  Flow.range(start: 0, count: 5)
    .as("$i")
    .then(HTTP.get(url: "https://example.com/job/123"))
    .then(Core.pick(fields: ["status", "result"]))
    .until(field: "status", equals: "ready")
    .select("$current") {
      state { current pipeline }
    }
}
```

Design intent:

1. keep authoring linear and chain-like for LLMs and humans
2. avoid raw loop bookkeeping in source
3. preserve strict bounded execution after lowering

### Repeat Loop (Lowered/Internal Form)

Repeat executes a pipeline body up to a fixed max iteration count.

```aql
query PollUntilReady {
  repeat(max: 5, until: { field: "status", equals: "ready" }) {
    HTTP.get(url: "https://example.com/job/123") {
      state { current }
    }
    |> Core.pick(fields: ["status", "result"]) {
      state { current }
    }
  }
}
```

Notes:

- `max` is required.
- `until` is optional.
- `until` is evaluated against current state after each body execution.

### Loop Variables with $ (Binding Model)

v1 should standardize loop-scoped variables using `$` identifiers.

Proposed syntax extension:

```aql
query PollWithVars {
  repeat(max: 5, as: "$i", until: { field: "status", equals: "ready" }) {
    Core.echo(message: "$i") {
      state { current }
    }
  }
}
```

For collection iteration (phase 1.5 or phase 2):

```aql
query IterateItems {
  repeat(over: ["a", "b", "c"], as: "$item", index_as: "$i", max: 10) {
    Core.merge(left: { index: "$i" }, right: { value: "$item" }) {
      state { current }
    }
  }
}
```

Variable scope rules:

1. Operation variables (`$name`) are outer scope.
2. Loop variables (`as`, `index_as`) are inner scope and shadow same-name outer variables.
3. Loop variables exist only inside loop body.
4. After loop exits, loop variables are out of scope.

Variable reference model:

1. Variables are references, not string interpolation tokens.
2. Parser stores variable nodes distinctly (`Value::Variable`) and never coerces them to quoted strings.
3. Runtime resolves variable references from lexical scope frames.
4. Undefined variable reference is a compile-time or runtime error (implementation choice, prefer compile-time when static).

Reserved runtime loop variables (always available inside a repeat body):

1. `$loop.index` (0-based)
2. `$loop.iteration` (1-based)
3. `$loop.max`
4. `$loop.first` (bool)
5. `$loop.last` (bool when max known)

### Recursion via Call

Recursion is explicit through named executable invocation.

```aql
iterator Step on Any {
  Core.echo() {
    state { current }
  }
}

query RecurseDemo {
  call Step(max_depth: 3) {
    state { current pipeline }
  }
}
```

Notes:

- `iterator` is the preferred keyword in v1 source.
- `fragment` remains a compatible alias for GraphQL-like authoring and migration.
- `call` targets a named executable (fragment/query/mutation/subscription).
- Runtime enforces depth ceilings.
- Direct self-call is allowed only with explicit bounded depth.

## AST/HIR Additions

### AST

Add node variants:

1. `FluentChain`:
   - `source: ChainSource`
   - `operators: Vec<ChainOperator>`
   - `terminal: ChainTerminal`
2. `RepeatBlock`:
   - `max: Value`
   - `over: Option<Value>`
   - `as_var: Option<String>`
   - `index_as_var: Option<String>`
   - `until: Option<UntilClause>`
   - `pipelines: Vec<Pipeline>`
3. `CallStep`:
   - `target: String`
   - `args: Vec<(String, Value)>`
   - `max_depth: Option<Value>`

### HIR

Lower to explicit control-flow nodes:

1. `HirStep::FluentChain` as an intermediate representation for diagnostics and tooling.
2. Lower `HirStep::FluentChain` to `HirStep::Repeat` + ordinary call steps.
3. `HirStep::Call` with target resolution metadata.

Verification additions:

1. `max` must be integer and `>= 1` when static.
2. `max_depth` must be integer and `>= 1` when static.
3. call target must exist.
4. recursion cycle detection must enforce explicit bounded depth.
5. `as`/`index_as` must be valid variable names beginning with `$`.
6. loop variable names cannot use reserved runtime names (`$loop.*`).
7. loop-scoped variable references must not escape body scope.

## MIR Shape (Proposed)

Current MIR is linear call blocks. v1 adds minimal control instructions:

1. `LoopStart { max, until }`
2. `LoopEnd`
3. `CallDef { target, max_depth }`

Alternative fallback (if MIR changes are deferred):

- unroll bounded repeat at compile time when `max` is static and small
- represent call as normal capability call to a runtime pseudo-module

Preferred path is explicit MIR ops for readability and traceability. Fluent chains are compile-time sugar over these primitives.

## Runtime Semantics

### Repeat

1. Evaluate `max`.
2. For each iteration:
  - bind reserved loop vars (`$loop.*`)
  - bind `as` / `index_as` vars when configured
   - execute body steps in order
   - evaluate `until` against current state (if provided)
   - break when `until` matches
3. If no `until`, run exactly `max` iterations.

### Call/Recursion

1. Resolve target executable in artifact MIR function table.
2. Push call frame with depth counter.
3. Execute target function body.
4. Pop frame and return resulting state.

## Safety and Budgets

Runtime must enforce all of the following regardless of source values:

1. `max_steps_total` per execution (global instruction budget).
2. `max_loop_iterations` per repeat block.
3. `max_call_depth` across nested calls.
4. `max_runtime_ms` wall-clock budget.

Any budget breach yields fatal runtime outcome with machine-readable error code.

Suggested default ceilings:

1. `max_steps_total = 1000`
2. `max_loop_iterations = 100`
3. `max_call_depth = 16`
4. `max_runtime_ms = 30000`

## Policy Interaction

Policy checks stay at call boundaries as they are today.

Implications:

1. repeated calls still pass through capability and policy guards each iteration
2. recursive calls do not bypass module policy
3. policy-denied step fails execution immediately

## Trace and Debugging Contract

Execution trace should expose control-flow metadata so users and tools can reason about behavior.

Proposed additions to pipeline trace entries:

1. `iteration_index` for loop body steps
2. `loop_id` to group loop executions
3. `call_depth` for call/recursion frames
4. `call_target` for call steps

## Phased Rollout Plan

### Phase 0: Fluent Surface + Lowering

1. Grammar + AST for fluent chain operators (`as`, `then`, `until`, `select`)
2. HIR lowering from fluent chain to repeat/call internals
3. Keep runtime unchanged except for existing repeat/call support
4. Add diagnostics that show lowered form for debugging

### Phase 1: Bounded Repeat

1. Grammar + AST for `repeat(max: ..., until: ...) { ... }`
2. HIR verify static bound types
3. Runtime execution with hard limits
4. Trace fields for iteration

### Phase 2: Named Call

1. Grammar + AST for `call Target(...)`
2. Target resolution and cycle checks
3. Runtime call frames + depth limits
4. Trace fields for call stack

### Phase 3: Subscription Alignment

1. Add runtime stream scheduling for subscription definitions
2. Integrate repeat/call with event ticks
3. Add cancellation semantics

## Open Decisions

1. Should fluent operators be method-like (`.then(...)`) only, or also keyword aliases?
2. Should `until` support boolean expressions or stay field-equals in v1?
3. Should recursion be limited to fragments in v1 for simpler mental model?
4. Should static unrolling be enabled for very small loops to simplify runtime?
5. Should budget defaults live in runtime config or policy guard config?
6. Should `repeat(over: ...)` land in phase 1 or phase 2?

## Example Set to Add with Implementation

1. `examples/repeat-http-poll.aql`
2. `examples/repeat-fixed-count.aql`
3. `examples/call-fragment-depth-bounded.aql`
4. `examples/recursion-depth-exceeded.aql` (expected failure)