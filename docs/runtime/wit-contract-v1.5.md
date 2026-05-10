# Grapheme WIT Runtime and Module Contract (V1.5)

Status: proposed target architecture for runtime-module interop in the v1.5 era.

This document defines how Grapheme should treat Wasm modules as managed runtime units (CLR-style assembly analogy) while keeping language/control-flow semantics in Grapheme runtime.

## Why WIT in V1.5

WIT should replace ad-hoc JSON serialization for runtime <-> module boundaries because it provides:

1. Stable typed contracts instead of implicit shape agreements.
2. Better performance characteristics than repeated stringify/parse for internal calls.
3. Safer evolution with explicit versioned interfaces.
4. Better generated bindings for Rust and other component-model targets.

Important boundary rule:

- Internal runtime-module calls: WIT typed contracts.
- External system edges (HTTP payloads, file content, CLI output): serialized bytes/JSON as domain data.

## Architectural Positioning

Grapheme should remain IR-first:

1. Grapheme source compiles to managed Grapheme IR/MIR.
2. Runtime executes IR and owns control-flow semantics (loop, call, retry, depth, cancellation).
3. Runtime dispatches selected operations to Wasm modules through WIT interfaces.

This keeps semantic authority in runtime while using Wasm for extension and isolation.

## Contract Layers

1. Compile Contract
- Compiler emits MIR + metadata (capabilities, loop config, call constraints).

2. Runtime Contract
- Runtime executes MIR deterministically and orchestrates call frames/state scopes.

3. Module ABI Contract
- WIT defines typed boundary for module operation invocation and host services.

## WIT Package Layout (Recommended)

Use one package namespace per major contract line:

- `grapheme:runtime@1.5.0`

Recommended files:

1. `types.wit`
- Shared records/variants used by all interfaces.

2. `module.wit`
- Exports that Grapheme runtime expects from a module component.

3. `host.wit`
- Imports runtime exposes to module components (logging, clocks, optional handles).

4. `policy.wit` (optional split)
- Policy inquiry interfaces if policy services are exposed to modules.

Current repository scaffold:

- `wit/grapheme-runtime-v1.5/types.wit`
- `wit/grapheme-runtime-v1.5/module.wit`
- `wit/grapheme-runtime-v1.5/host.wit`

## Core Type Model

The ABI should include these concepts as first-class types:

1. Execution context
- `trace_id`, `step_index`, `call_depth`, `iteration_index`, `function_name`.

2. Capability context
- requested capability and enforcement mode.

3. Resource budget snapshot
- cpu/memory/io/network limits visible to callee for graceful behavior.

4. Invocation input
- typed argument map plus runtime-provided pipeline input.

5. Invocation output
- output payload, diagnostics, metrics, and optional effect declarations.

6. Error taxonomy
- deterministic variants mapped to runtime outcome codes.

## Example WIT Sketch

```wit
package grapheme:runtime@1.5.0;

interface types {
  record trace-context {
    trace-id: string,
    step-index: u32,
    call-depth: u16,
    iteration-index: option<u32>,
    function-name: string,
    call-target: string,
  }

  record capability-context {
    capability: string,
  }

  record resource-budget {
    max-cpu-ms: u64,
    max-memory-mb: u64,
    max-io-bytes: u64,
    max-network-calls: u32,
  }

  record invoke-input {
    args-json: string,
    pipeline-input-json: string,
    trace: trace-context,
    capability: capability-context,
    budget: resource-budget,
  }

  record diagnostic {
    level: string,
    code: string,
    message: string,
  }

  record invoke-output {
    output-json: string,
    diagnostics: list<diagnostic>,
    cpu-ms: u64,
    io-bytes: u64,
    network-calls: u32,
  }

  variant invoke-error {
    invalid-args(string),
    denied(string),
    retryable(string),
    fatal(string),
  }
}

interface module {
  use types.{invoke-input, invoke-output, invoke-error};

  invoke: func(op: string, input: invoke-input) -> result<invoke-output, invoke-error>;
}
```

Note: the sketch keeps JSON payload fields for rapid adoption. In later iterations, high-volume ops can move selected payloads to stronger WIT-native records.

## Runtime Responsibilities (Non-Negotiable)

Runtime remains authoritative for:

1. Call graph orchestration
- function dispatch, recursion depth checks, loop scheduling, cancellation.

2. State ownership
- frame-local state, merge semantics, and final state projection.

3. Policy enforcement
- capability allow/deny, argument policy checks, egress constraints.

4. Deterministic outcomes
- outcome code mapping and trace generation.

Modules should not mutate global runtime state directly.

## Error and Outcome Mapping

Define a strict mapping table from module error variants to runtime outcomes:

1. `invalid-args` -> `FATAL` with compile/runtime contract violation message.
2. `denied` -> `POLICY_DENIED`.
3. `retryable` -> `RETRYABLE`.
4. `fatal` -> `FATAL`.

Runtime still wraps all failures into canonical execution result envelopes.

## Versioning Policy

Use explicit versioning in WIT package namespace and module manifests.

Rules:

1. Additive field/type additions are non-breaking within minor versions.
2. Removed/renamed fields or semantic reinterpretation requires major bump.
3. Runtime advertises supported WIT package ranges.
4. Module manifest must declare implemented WIT package version.

## Migration Plan from Current V1

1. Keep existing `mir_v1` and `wasix_v1` routing behavior.
2. Introduce `wasix_wit_v1_5` ABI tag in module manifest.
3. Add runtime adapter that maps current call envelope -> WIT invoke input.
4. Preserve current JSON envelopes for legacy modules.
5. Gate new modules on WIT conformance tests.

## Conformance Tests to Add

1. ABI compatibility tests
- runtime rejects modules with unsupported WIT package versions.

2. Deterministic error mapping tests
- each module error variant maps to expected runtime outcome code.

3. Budget telemetry tests
- reported resource metrics do not exceed runtime observations.

4. Trace propagation tests
- trace context is preserved across nested call and loop boundaries.

## Open Design Decisions

1. Whether `args` should remain JSON string in v1.5 or use typed value variants immediately.
2. Whether policy APIs are fully host-internal or partially visible to trusted modules.
3. How to represent large binary payloads (inline bytes vs runtime handle references).
4. Whether module invocation stays single-entry `invoke(op, input)` or moves to per-op exports.

## Recommendation

For v1.5, adopt WIT now with a pragmatic hybrid:

1. WIT for contract shape and strong context typing.
2. JSON payload fields for fast migration and backwards compatibility.
3. Progressive typing of high-value operations over v1.6+.

This keeps momentum while moving the architecture toward Roslyn/CLR-like contract rigor.
