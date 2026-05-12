# RFC-0001: Embedded Runtime SDK v1

Status: draft
Authors: runtime + cli
Created: 2026-05-12
Target release window: next 1-2 sprints

## Summary

Introduce an embeddable Rust SDK for Grapheme runtime execution so application developers can compile and execute workflows in-process without invoking the CLI.

The SDK becomes the primary integration surface for agentic systems. The CLI remains a thin adapter over SDK APIs.

## Motivation

Current strengths:

1. Reliable compile/runtime pipeline.
2. Capability-governed execution model.
3. Growing CLI discovery UX with structured output.

Current gap:

- Rust applications cannot yet consume Grapheme as a first-class library API with stable embedding semantics.

Desired outcome:

- Any ad-hoc model workflow behavior can be represented as Grapheme programs and executed via embedded runtime controls.

## Goals

1. Provide stable in-process compile + execute APIs.
2. Preserve runtime policy/trace behavior parity with CLI.
3. Support LLM-native structured output (`yaml` default, `json` optional) for SDK-facing diagnostics/introspection.
4. Keep public API small, explicit, and versioned.

## Non-Goals

1. Replace CLI UX.
2. Implement database features in this RFC.
3. Implement hot module loading in this RFC.
4. Introduce cross-language SDKs.

## Proposed Architecture

### Crate layout

Preferred:

1. Add new crate `crates/grapheme-sdk`.
2. Move reusable orchestration from CLI into SDK crate.
3. Keep `crates/grapheme-cli` as argument parsing + presentation layer.

Alternative:

- Expose SDK API directly from existing runtime crate.

Decision:

- Use dedicated `grapheme-sdk` crate for cleaner API ownership and versioning.

### Core types (draft)

```rust
pub struct GraphemeEngineBuilder { /* policy, trace, modules, limits */ }

pub struct GraphemeEngine { /* compiled runtime context */ }

pub struct ExecuteRequest {
    pub source: Option<String>,
    pub artifact: Option<grapheme_artifact::ArtifactEnvelope>,
}

pub struct ExecuteResult {
    pub artifact_id: String,
    pub final_state: serde_json::Value,
    pub execution: grapheme_artifact::ExecutionResult,
    pub lint_warnings: Vec<grapheme_compiler::verifier::LintWarning>,
}

pub enum StructuredMode { Yaml, Json }
```

### API sketch (draft)

```rust
impl GraphemeEngineBuilder {
    pub fn new() -> Self;
    pub fn with_policy_guard(self, guard: grapheme_runtime::PolicyGuard) -> Self;
    pub fn with_trace_policy(self, policy: grapheme_runtime::TracePolicy) -> Self;
    pub fn with_module_path(self, module: &str, path: std::path::PathBuf) -> Self;
    pub fn with_native_module_autobuild(self, enabled: bool) -> Self;
    pub fn build(self) -> Result<GraphemeEngine, GraphemeSdkError>;
}

impl GraphemeEngine {
    pub fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResult, GraphemeSdkError>;
    pub fn format_result(
        &self,
        result: &ExecuteResult,
        mode: StructuredMode,
    ) -> Result<String, GraphemeSdkError>;
}
```

## Behavior and Compatibility

1. Execution semantics must match current CLI behavior.
2. Policy guard environment behavior is preserved, with explicit builder overrides.
3. `--native-modules` behavior maps to SDK builder flag.
4. Module bindings map to `with_module_path`.

## Error Model

Introduce SDK error taxonomy:

1. `CompileError`
2. `RuntimeError`
3. `PolicyError`
4. `ModuleLoadError`
5. `SerializationError`

Guideline:

- Keep original source errors available for diagnostics.

## Security Considerations

1. SDK must default to same policy-safe execution assumptions as CLI.
2. Native module auto-build should be opt-in when embedded.
3. All capability invocations remain policy-gated.
4. Structured outputs must avoid leaking sensitive policy internals by default.

## Observability

1. Preserve runtime trace projection settings.
2. Add hooks for host applications to receive step-level events.
3. Maintain deterministic event ordering in single-run execution.

## Testing Strategy

1. Unit parity tests between CLI and SDK for representative workflows.
2. Golden structured output tests for YAML and JSON formatting.
3. Policy profile tests covering allowed and denied capability invocations.
4. Module binding tests for host and Wasm module paths.

## Rollout Plan

### Phase 1: Extraction

1. Move reusable run orchestration from CLI to SDK internals.
2. Keep CLI behavior unchanged.

### Phase 2: Public API

1. Stabilize builder and execute API.
2. Add examples for embedding in Rust apps.

### Phase 3: Hardening

1. Contract tests for output, errors, and policy parity.
2. Publish migration guide and versioning guarantees.

## Migration Plan

1. Existing CLI commands continue to work unchanged.
2. Document SDK as preferred integration path for Rust apps.
3. Provide equivalent mappings for current CLI flags.

## Open Questions

1. Should SDK expose compile-only and run-artifact-only entrypoints separately?
2. Should YAML remain default in SDK formatting, or only in CLI-facing helpers?
3. How much of module autobuild should be retained in embedded contexts?

## Acceptance Criteria

1. A minimal Rust host app can execute a `.gr` program fully in-process.
2. SDK output parity with CLI for `run` reference examples.
3. Policy and trace controls are configurable through builder APIs.
4. CLI run path is demonstrably powered by SDK internals.
