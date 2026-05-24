# Grapheme Compiler and Runtime Split

> Historical note: this document describes an early architecture framing. For current crate layout and runtime behavior, see `docs/architecture.md`.

## Intent

Model Grapheme after a Roslyn-style architecture:

1. Compiler is responsible for source analysis and artifact emission.
2. Runtime is responsible for executing compiled artifacts in a governed sandbox.
3. Orchestrators such as Stasis integrate through a plugin adapter layer.

## Module Layout (Current Monorepo)

- src/compiler_api/mod.rs
  - Compiler facade for source -> compilation -> artifact.
- src/artifact.rs
  - Shared artifact ABI and execution result contracts.
- src/runtime/mod.rs
  - Runtime engine that validates artifact integrity and executes MIR calls.
- src/host/mod.rs
  - Host capability invocation interface for plugin adapters.

## Ownership Boundaries

### Compiler API

Responsibilities:
- Parse source.
- Run compile pipeline passes and verifiers.
- Emit immutable artifact envelope.

### Runtime Engine

Responsibilities:
- Validate payload format compatibility.
- Verify artifact integrity hash.
- Select entrypoint and execute MIR calls.
- Enforce runtime capability policy.
- Produce deterministic execution outcome and trace summary.

### Host Adapter Interface

Responsibilities:
- Resolve capability calls to concrete plugin/tool behavior.
- Classify host errors as retryable or fatal.

## Stasis Integration Model

Stasis plugin flow:

1. Receive or generate Grapheme source.
2. Call compiler API to emit artifact envelope.
3. Persist artifact and reference in job payload.
4. Run Grapheme runtime in worker process with scoped capability host.
5. Persist execution outputs by STTP references.

## Next Steps

1. Move artifact structs into dedicated grapheme-artifact crate.
2. Move runtime module into dedicated grapheme-runtime crate.
3. Add wasmer-backed executor behind a runtime feature flag.
4. Add stasis-grapheme-plugin crate with host adapter implementation.
