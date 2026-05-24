# Runtime Data and State Flow (Current)

This document describes the current control-flow and state behavior implemented in the codebase today.

Scope:

- Compiler lowering of `@loop(...)` metadata
- Runtime function execution, call dispatch, and loop behavior
- AgentState mutation behavior on success/failure
- Why loop handling feels fragile with the current state model

## 1) Compile-Time Flow for Loop Metadata

The compiler preserves loop directives from AST -> HIR -> MIR.

- HIR captures `loop_directive_count` and raw `loop_args` per executable.
- Verifier enforces one `@loop`, fragment-only placement, `max >= 1`, and `until` shape.
- MIR lowering converts loop args into `MirLoopConfig { max, until }`.

```mermaid
flowchart TD
  A[Source .gr with fragment loop directive] --> B[Parser and AST]
  B --> C[HIR lowering with loop args]
  C --> D[Verifier checks max and until]
  D --> E[MIR lowering to loop config]
  E --> F[Artifact envelope with integrity hash]
```

## 2) Runtime Execution Path

`RuntimeEngine::execute_artifact` resolves entrypoint and delegates to `execute_function`.

Inside `execute_function`:

- Determine `iteration_max` from `function.loop_config.max` (default `1`).
- For each iteration, execute blocks/instructions in order.
- For `call` module steps: recurse into another MIR function.
- For capability steps: resolve module/op, policy-check args, invoke host/wasm, advance state.
- After each full iteration, evaluate `loop_until_satisfied` against `state.current`.

```mermaid
flowchart TD
    A[execute_artifact] --> B[verify format + integrity]
    B --> C[resolve entrypoint]
    C --> D[execute_function]

    D --> E[iteration_max = loop.max or 1]
    E --> F{for each iteration}
    F --> G{for each instruction}

    G -->|module == call| H[execute_function recursively]
    H --> I[state.advance_in_place\nwith current clone]
    I --> G

    G -->|capability call| J[resolve module/op + policy guard]
    J --> K[host.call or wasix]
    K --> L[state.advance_in_place]
    L --> G

    G --> M[iteration complete]
    M --> N{until matched on state.current field equals eq}
    N -->|yes| O[break loop]
    N -->|no| F
    O --> P[return to caller]
```

## 3) Call Sequence and State Mutation

This sequence diagram reflects one successful capability step and one failure path.

```mermaid
sequenceDiagram
    participant R as RuntimeEngine
    participant S as AgentState
    participant G as PolicyGuard
    participant H as CapabilityHost

    R->>G: check(resolved_call, args+__input)
    G-->>R: allow
    R->>H: call(CapabilityCall)
    H-->>R: output JSON
    R->>S: advance_in_place(step, op, output)
    Note over S: diff = compute_diff(current, output)\npipeline push ok=true\ncurrent = output

    R->>H: call(CapabilityCall)
    H-->>R: fatal/retryable error
    R->>S: fail_in_place(step, op, code, message)
    Note over S: errors push\npipeline push ok=false\ndiff = null\ncurrent unchanged
```

## 4) AgentState Model (Current Behavior)

`AgentState` is a single mutable accumulator threaded through all calls and iterations:

- `current`: last successful output value
- `diff`: structural diff from previous `current` to new output
- `errors`: accumulated failures
- `pipeline`: append-only step history
- `proposed`: proposal tracking

### State Transition Shape

```mermaid
stateDiagram-v2
    [*] --> Fresh
    Fresh --> SuccessStep: advance_in_place
    Fresh --> FailedStep: fail_in_place

    SuccessStep --> SuccessStep: next success step
    SuccessStep --> FailedStep: step failure

    FailedStep --> SuccessStep: later success step
    FailedStep --> FailedStep: another failure

    state SuccessStep {
      [*] --> Mutate
      Mutate: current <- output
      Mutate: diff <- compute_diff(prev, output)
      Mutate: pipeline += ok entry
    }

    state FailedStep {
      [*] --> Record
      Record: errors += {step, code, message}
      Record: pipeline += failed entry
      Record: diff <- null
      Record: current unchanged
    }
```

## 5) Why Looping Feels Bad Today

Based on current runtime/state code, the main pressure points are:

1. Single global `current` for all scopes
- Nested calls and loop iterations all write to one `state.current`.
- There is no per-call-frame local state snapshot.

2. Loop termination is tied to final top-level object shape
- `until` checks only `state.current[field] == eq` at end of each iteration.
- No path expressions, no nested lookup, and no richer predicate evaluation.

3. Loop/call provenance is missing in trace entries
- `pipeline` currently stores step index/op/output/ok/error only.
- Iteration index, call depth, and call target are not captured.

4. Error and diff semantics are coarse for control flow
- Any failed step forces immediate failure result and `diff = null` for that step.
- There is no first-class loop-local failure handling policy.

5. Input propagation is implicit and broad
- Each capability call gets `__input = state.current` merged into args.
- This makes data dependencies convenient, but also hides scope boundaries.

## 6) Practical Next Architecture Step

A good next step before redesigning language surface syntax is adding runtime trace context:

- Add `iteration_index`, `call_depth`, `function_name`, `call_target` to pipeline entries.
- Keep behavior unchanged initially; improve observability first.
- Then introduce scoped state (frame-local current/diff) with explicit merge rules.

That sequence keeps risk controlled while making loop bugs easier to reason about.
