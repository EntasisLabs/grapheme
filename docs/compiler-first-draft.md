# Grapheme Compiler First Draft

> Historical note: this document captures early design intent and is not the primary source of truth for the current implementation. Use `docs/README.md` for up-to-date guidance.

## Goal

Establish an LLVM-like compiler pipeline for Grapheme that lowers declarative agent workflows into a verifiable execution form suitable for WebAssembly runtimes (Wasmer).

## Compiler Shape

1. Frontend
- Parse source text into typed AST.
- Surface syntax and structural parse errors.

2. Mid-end
- Lower AST -> HIR (capability-aware logical plan).
- Verify HIR invariants.
- Lower HIR -> MIR (Wasm-oriented instruction plan).
- Verify MIR invariants and capability policy constraints.

3. Backend (next phase)
- Lower MIR -> executable Wasm plan/module.
- Bind host imports to approved capabilities only.
- Emit runtime metadata for replay/audit.

## Implemented in This Draft

- `src/compiler/hir.rs`
  - HIR nodes for executable defs and pipeline steps.
  - Capability extraction from module/op usage.

- `src/compiler/mir.rs`
  - MIR function/block/instruction structures.
  - Call-oriented instruction set with state-threading semantics.

- `src/compiler/capability.rs`
  - Canonical capability token model.
  - Draft allow/deny policy object.

- `src/compiler/verifier.rs`
  - HIR verification:
    - executable definitions exist
    - names are non-empty
    - pipelines and steps are non-empty
  - MIR verification:
    - functions and blocks are present
    - all capabilities satisfy policy

- `src/compiler/pipeline.rs`
  - `compile_program` orchestration API.
  - `CompileOptions` with capability policy.
  - `CompilationArtifact` carrying AST/HIR/MIR.

- `src/lib.rs`
  - `compile(source: &str)` helper to parse and run the compiler pipeline.

## Security Direction

This draft starts capability-based execution policy at compile time.

Current model:
- Every operation resolves to a capability token.
- Policy checks run during MIR verification.
- Denied capability stops compilation.

Planned extensions:
- Capability scopes and delegation.
- Signed capability manifests for plugins/modules.
- Runtime-enforced capability handles in Wasmer imports.

## Replayability Direction

Current artifacts already preserve deterministic structure:
- Ordered pipelines
- Explicit capability calls
- Step-level instruction ordering

Planned extensions:
- Stable instruction IDs.
- Snapshot/checkpoint format keyed by function + step.
- Deterministic input/event logs for replay and audit.

## Near-Term Next Steps

1. Semantic analysis phase before HIR lowering
- variable resolution
- type constraints and coercions
- module/op signature checks

2. Expanded MIR
- explicit state read/write ops
- branching and error edges
- effect annotations (pure, IO, network, storage)

3. Wasm backend MVP
- map MIR call instructions to host imports
- run with Wasmer and fuel limits
- emit trace stream from runtime

4. Tooling
- `grapheme compile file.aql --emit hir|mir|json`
- verifier diagnostics with source spans
- golden tests for lowering + verification
