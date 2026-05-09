# Grapheme x Stasis Integration Contract (First Draft)

## Intent

Grapheme must run standalone without Stasis.
Stasis is the preferred orchestration runtime for production multi-agent workloads.

This contract defines strict ownership boundaries to avoid architectural overlap.

## Ownership Boundaries

### Grapheme Owns

1. Language and compilation
- Parser and grammar
- AST/HIR/MIR and verifier passes
- Capability declarations extracted from programs
- Wasm-oriented execution artifact generation

2. Execution semantics
- Pipeline step semantics
- Agent state transition semantics within a single execution
- Deterministic step trace model

3. Standalone runtime path
- Local compile + execute loop
- Minimal host bindings for module calls

### Stasis Owns

1. Orchestration lifecycle
- Queueing, leasing, heartbeats
- Retry and dead-letter policy
- Recurring scheduling
- Outbox and event publication

2. Durable context continuity
- STTP node id input/output plumbing
- Job correlation and trace propagation
- Replay entry points

3. Multi-tenant and policy operations
- Queue-level policy enforcement
- Operational controls and run governance

## Non-Overlap Rules

1. Grapheme does not implement durable job scheduling.
2. Grapheme does not persist orchestration state machines.
3. Stasis does not parse/compile Grapheme source internals.
4. Stasis treats Grapheme artifacts as executable units, not source compilers.

## Integration Surface

## Artifact Envelope (Produced by Grapheme)

```json
{
  "artifact_id": "string",
  "artifact_version": "string",
  "entrypoint": "string",
  "required_capabilities": ["Database.query", "WebSocket.send"],
  "payload_ref": "sttp-or-artifact-ref",
  "integrity_hash": "sha256:..."
}
```

## Job Payload (Consumed by Stasis Worker)

```json
{
  "artifact_ref": "string",
  "input_sttp_node_id": "string",
  "capability_scope": ["Database.query"],
  "correlation_id": "string",
  "causation_id": "string",
  "trace_id": "string"
}
```

## Execution Result (Emitted by Worker)

```json
{
  "outcome": "succeeded|retryable_failure|fatal_failure",
  "output_sttp_node_id": "string|null",
  "trace_summary": {
    "steps": 3,
    "failed_step": null
  },
  "message": "string|null"
}
```

## Capability Security Model

Dual-gate enforcement:

1. Compile-time gate in Grapheme
- Verifier rejects disallowed capabilities.

2. Runtime gate in Stasis
- Worker admission policy validates requested capabilities for the tenant/job context.

A capability must pass both gates to execute.

## Standalone Mode Requirements

Grapheme standalone mode must support:

1. Compile source to artifact.
2. Execute artifact with local module bindings.
3. Produce deterministic state and trace output.
4. Optional capability policy input for local safety testing.

No Stasis dependency is required for the standalone path.

## Stasis-Preferred Mode Requirements

When running under Stasis:

1. Input context is resolved by STTP reference.
2. Execution occurs inside a leased job attempt.
3. Output is persisted as STTP reference.
4. Runtime events are emitted through outbox publisher.
5. Retry/dead-letter behavior is owned by Stasis.

## Versioning and Compatibility

1. Grapheme artifacts include semantic version metadata.
2. Stasis worker adapter declares supported artifact versions.
3. Incompatibilities fail fast at job start with explicit diagnostics.

## Near-Term Milestones

1. Define Rust structs for artifact envelope and execution result.
2. Add CLI emit target for artifact metadata (`grapheme compile --emit artifact`).
3. Build a Stasis runtime job handler that invokes Grapheme runtime entrypoint.
4. Add an end-to-end test: enqueue -> execute Grapheme artifact -> persist STTP output.
