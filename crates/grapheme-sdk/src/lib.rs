//! Embedded Grapheme SDK for in-process compile and execution workflows.
//!
//! The SDK wraps compiler and runtime crates with an ergonomic builder API,
//! structured output formatting, and AOT helper entrypoints.

use grapheme_artifact::{build_stage_b_container_from_aot, AotEnvelope, ArtifactEnvelope, ExecutionResult};
use grapheme_compiler::verifier::LintWarning;
use grapheme_compiler::{CompiledScript, Compiler, CompilerError, CompilerOptions};
use grapheme_runtime::{
    CapabilityCall, CapabilityHost, HostCallError, PolicyGuard, RuntimeEngine, RuntimeError,
    RuntimeOptions, TracePolicy,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

type HostFactory = Arc<dyn Fn() -> Box<dyn CapabilityHost + Send> + Send + Sync>;
type CapabilityObserver = Arc<dyn Fn(&CapabilityCall) + Send + Sync>;
type CapabilityInterceptor =
    Arc<dyn Fn(&CapabilityCall) -> Option<Result<JsonValue, HostCallError>> + Send + Sync>;

/// Structured output mode for SDK formatting helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredMode {
    /// YAML output.
    Yaml,
    /// JSON output.
    Json,
}

/// Top-level execute payload returned by SDK execution entrypoints.
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteResultPayload {
    /// Executed artifact id.
    pub artifact_id: String,
    /// Runtime execution result summary.
    pub execution: ExecutionResult,
    /// Final runtime state as JSON.
    pub final_state: JsonValue,
    /// Non-fatal compiler warnings collected during compile path.
    pub lint_warnings: Vec<LintWarning>,
}

/// Errors returned by SDK compile/execute/format operations.
#[derive(Debug, Error)]
pub enum GraphemeSdkError {
    #[error(transparent)]
    Compiler(#[from] CompilerError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("aot contract error: {0}")]
    Contract(String),
    #[error("serialize output: {0}")]
    Serialization(String),
}

/// Builder for configuring `GraphemeEngine` runtime behavior.
pub struct GraphemeEngineBuilder {
    runtime_options: RuntimeOptions,
    module_bindings: HashMap<String, PathBuf>,
    host_factory: Option<HostFactory>,
    capability_observer: Option<CapabilityObserver>,
    capability_interceptor: Option<CapabilityInterceptor>,
}

impl Default for GraphemeEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphemeEngineBuilder {
    /// Create a new builder with runtime defaults.
    pub fn new() -> Self {
        Self {
            runtime_options: RuntimeOptions::default(),
            module_bindings: HashMap::new(),
            host_factory: None,
            capability_observer: None,
            capability_interceptor: None,
        }
    }

    /// Set policy guard for runtime admission checks.
    pub fn with_policy_guard(mut self, guard: PolicyGuard) -> Self {
        self.runtime_options.policy_guard = guard;
        self
    }

    /// Set trace policy shaping runtime pipeline history.
    pub fn with_trace_policy(mut self, policy: TracePolicy) -> Self {
        self.runtime_options.trace_policy = policy;
        self
    }

    /// Enable or disable artifact integrity verification.
    pub fn with_verify_integrity(mut self, enabled: bool) -> Self {
        self.runtime_options.verify_integrity = enabled;
        self
    }

    /// Enable or disable streaming plain step output.
    pub fn with_stream_step_output(mut self, enabled: bool) -> Self {
        self.runtime_options.stream_step_output = enabled;
        self
    }

    /// Enable or disable strict Stage B container execution mode.
    pub fn with_strict_stage_b_container_execution(mut self, enabled: bool) -> Self {
        self.runtime_options.strict_stage_b_container_execution = enabled;
        self
    }

    /// Set optional maximum step count.
    pub fn with_max_steps(mut self, max_steps: Option<usize>) -> Self {
        self.runtime_options.max_steps = max_steps;
        self
    }

    /// Set optional maximum nested call depth.
    pub fn with_max_call_depth(mut self, max_call_depth: Option<usize>) -> Self {
        self.runtime_options.max_call_depth = max_call_depth;
        self
    }

    /// Bind a module id to a Wasm path for runtime resolution.
    pub fn with_module_path(mut self, module: &str, path: impl Into<PathBuf>) -> Self {
        self.module_bindings
            .insert(module.to_lowercase(), path.into());
        self
    }

    /// Register an observer called for each capability invocation.
    pub fn with_capability_observer<F>(mut self, observer: F) -> Self
    where
        F: Fn(&CapabilityCall) + Send + Sync + 'static,
    {
        self.capability_observer = Some(Arc::new(observer));
        self
    }

    /// Register an interceptor that can override capability call results.
    pub fn with_capability_interceptor<F>(mut self, interceptor: F) -> Self
    where
        F: Fn(&CapabilityCall) -> Option<Result<JsonValue, HostCallError>> + Send + Sync + 'static,
    {
        self.capability_interceptor = Some(Arc::new(interceptor));
        self
    }

    /// Provide a custom host factory for full capability dispatch control.
    pub fn with_host_factory<F>(mut self, host_factory: F) -> Self
    where
        F: Fn() -> Box<dyn CapabilityHost + Send> + Send + Sync + 'static,
    {
        self.host_factory = Some(Arc::new(host_factory));
        self
    }

    /// Build the configured `GraphemeEngine`.
    pub fn build(self) -> GraphemeEngine {
        GraphemeEngine {
            runtime_options: self.runtime_options,
            module_bindings: self.module_bindings,
            host_factory: self.host_factory,
            capability_observer: self.capability_observer,
            capability_interceptor: self.capability_interceptor,
        }
    }
}

/// High-level embedded engine for compile/execute and AOT helper flows.
pub struct GraphemeEngine {
    runtime_options: RuntimeOptions,
    module_bindings: HashMap<String, PathBuf>,
    host_factory: Option<HostFactory>,
    capability_observer: Option<CapabilityObserver>,
    capability_interceptor: Option<CapabilityInterceptor>,
}

impl GraphemeEngine {
    /// Create a builder for `GraphemeEngine`.
    pub fn builder() -> GraphemeEngineBuilder {
        GraphemeEngineBuilder::new()
    }

    /// Compile and execute source in one call.
    pub fn execute_source(&self, source: &str) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        let compiled = Compiler::compile_source(source, CompilerOptions::default())?;
        self.execute_compiled(&compiled)
    }

    /// Compile source into a Stage A AOT envelope.
    pub fn compile_source_to_aot(&self, source: &str) -> Result<AotEnvelope, GraphemeSdkError> {
        let compiled = Compiler::compile_source_to_aot(source, CompilerOptions::default())?;
        Ok(compiled.aot)
    }

    /// Compile source into Stage B AOT using provided workflow bytes/imports.
    pub fn compile_source_to_aot_stage_b(
        &self,
        source: &str,
        workflow_wasm: &[u8],
        allowed_imports: &[String],
    ) -> Result<AotEnvelope, GraphemeSdkError> {
        let stage_a = self.compile_source_to_aot(source)?;
        build_stage_b_container_from_aot(&stage_a, workflow_wasm, allowed_imports)
            .map_err(|e| GraphemeSdkError::Contract(e.to_string()))
    }

    /// Execute a prebuilt artifact envelope.
    pub fn execute_artifact(
        &self,
        artifact: &ArtifactEnvelope,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        self.execute_artifact_with_lints(artifact, Vec::new())
    }

    /// Execute a prebuilt AOT envelope.
    pub fn execute_aot(&self, aot: &AotEnvelope) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        self.execute_aot_with_lints(aot, Vec::new())
    }

    /// Execute a compiled script produced by compiler APIs.
    pub fn execute_compiled(
        &self,
        compiled: &CompiledScript,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        self.execute_artifact_with_lints(
            &compiled.artifact,
            compiled.compilation.lint_warnings.clone(),
        )
    }

    /// Format execute results as YAML or JSON.
    pub fn format_result(
        &self,
        result: &ExecuteResultPayload,
        mode: StructuredMode,
    ) -> Result<String, GraphemeSdkError> {
        match mode {
            StructuredMode::Json => serde_json::to_string_pretty(result)
                .map_err(|e| GraphemeSdkError::Serialization(e.to_string())),
            StructuredMode::Yaml => serde_yaml::to_string(result)
                .map_err(|e| GraphemeSdkError::Serialization(e.to_string())),
        }
    }

    /// Format an AOT envelope as YAML or JSON.
    pub fn format_aot(&self, aot: &AotEnvelope, mode: StructuredMode) -> Result<String, GraphemeSdkError> {
        match mode {
            StructuredMode::Json => serde_json::to_string_pretty(aot)
                .map_err(|e| GraphemeSdkError::Serialization(e.to_string())),
            StructuredMode::Yaml => serde_yaml::to_string(aot)
                .map_err(|e| GraphemeSdkError::Serialization(e.to_string())),
        }
    }

    fn execute_artifact_with_lints(
        &self,
        artifact: &ArtifactEnvelope,
        lint_warnings: Vec<LintWarning>,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        let mut options = self.runtime_options.clone();
        for (module, path) in &self.module_bindings {
            options
                .module_registry
                .set_wasm_path(module.as_str(), path.clone());
        }

        let runtime = RuntimeEngine::new(options);
        let (state, execution) = if let Some(factory) = &self.host_factory {
            let mut host = factory();
            runtime.execute_artifact(artifact, host.as_mut())?
        } else {
            let mut host = StdlibHost {
                capability_observer: self.capability_observer.clone(),
                capability_interceptor: self.capability_interceptor.clone(),
            };
            runtime.execute_artifact(artifact, &mut host)?
        };

        Ok(ExecuteResultPayload {
            artifact_id: artifact.artifact_id.clone(),
            execution,
            final_state: state.to_json(),
            lint_warnings,
        })
    }

    fn execute_aot_with_lints(
        &self,
        aot: &AotEnvelope,
        lint_warnings: Vec<LintWarning>,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        let mut options = self.runtime_options.clone();
        for (module, path) in &self.module_bindings {
            options
                .module_registry
                .set_wasm_path(module.as_str(), path.clone());
        }

        let runtime = RuntimeEngine::new(options);
        let (state, execution) = if let Some(factory) = &self.host_factory {
            let mut host = factory();
            runtime
                .execute_aot(aot, host.as_mut())
                .map_err(map_runtime_aot_error)?
        } else {
            let mut host = StdlibHost {
                capability_observer: self.capability_observer.clone(),
                capability_interceptor: self.capability_interceptor.clone(),
            };
            runtime
                .execute_aot(aot, &mut host)
                .map_err(map_runtime_aot_error)?
        };

        Ok(ExecuteResultPayload {
            artifact_id: aot.base_artifact.artifact_id.clone(),
            execution,
            final_state: state.to_json(),
            lint_warnings,
        })
    }
}

fn map_runtime_aot_error(err: RuntimeError) -> GraphemeSdkError {
    match err {
        RuntimeError::ArtifactCompatibilityError(message) => GraphemeSdkError::Contract(message),
        other => GraphemeSdkError::Runtime(other),
    }
}

struct StdlibHost {
    capability_observer: Option<CapabilityObserver>,
    capability_interceptor: Option<CapabilityInterceptor>,
}

impl StdlibHost {
    fn resolve_module(call: &CapabilityCall) -> String {
        call.module
            .as_deref()
            .map(|m| m.to_lowercase())
            .or_else(|| call.capability.split('.').next().map(|m| m.to_lowercase()))
            .unwrap_or_default()
    }
}

impl CapabilityHost for StdlibHost {
    fn call(&mut self, call: &CapabilityCall) -> Result<JsonValue, HostCallError> {
        if let Some(observer) = &self.capability_observer {
            observer(call);
        }

        if let Some(interceptor) = &self.capability_interceptor {
            if let Some(outcome) = interceptor(call) {
                return outcome;
            }
        }

        let module = Self::resolve_module(call);
        if let Some(out) = grapheme_stdlib::registry::dispatch(&module, &call.op, &call.args) {
            return Ok(out);
        }

        Err(HostCallError::Fatal(format!(
            "unsupported capability dispatch: module='{}' op='{}'",
            module, call.op
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grapheme_artifact::ExecutionOutcome;
    use grapheme_runtime::PolicyGuard;
    use std::sync::{Arc, Mutex};

    #[test]
    fn execute_source_runs_core_echo() {
                let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "hello from sdk") {
    state { current }
  }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let result = engine.execute_source(source).expect("execution should succeed");

        assert_eq!(
            result
                .final_state
                .get("current")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str()),
            Some("hello from sdk")
        );
    }

    #[test]
    fn format_result_supports_yaml_and_json() {
                let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "hello") {
    state { current }
  }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let result = engine.execute_source(source).expect("execution should succeed");

        let yaml = engine
            .format_result(&result, StructuredMode::Yaml)
            .expect("yaml format should succeed");
        let json = engine
            .format_result(&result, StructuredMode::Json)
            .expect("json format should succeed");

        assert!(yaml.contains("artifact_id:"));
        assert!(json.contains("\"artifact_id\""));
    }

    #[test]
    fn execute_artifact_preserves_lint_warnings_from_compilation_path() {
                let source = r#"import core from "grapheme/core"

query Q {
    core.echo(message: "hi") {
    state { current }
  }
}
"#;

        let compiled = Compiler::compile_source(source, CompilerOptions::default())
            .expect("compile should succeed");
        let engine = GraphemeEngine::builder().build();
        let result = engine
            .execute_compiled(&compiled)
            .expect("compiled execution should succeed");

        assert_eq!(result.lint_warnings.len(), compiled.compilation.lint_warnings.len());
    }

    #[test]
    fn capability_observer_receives_host_calls() {
        let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "observe") {
    state { current }
  }
}
"#;

        let observed = Arc::new(Mutex::new(Vec::<String>::new()));
        let observed_ref = Arc::clone(&observed);

        let engine = GraphemeEngine::builder()
            .with_capability_observer(move |call| {
                observed_ref
                    .lock()
                    .expect("lock observer")
                    .push(format!("{}.{}", call.module.clone().unwrap_or_default(), call.op));
            })
            .build();

        let result = engine.execute_source(source).expect("execution should succeed");
        assert_eq!(
            result
                .final_state
                .get("current")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str()),
            Some("observe")
        );

        let calls = observed.lock().expect("lock observer snapshot");
        assert!(calls.iter().any(|v| v.ends_with(".echo")));
    }

    #[test]
    fn capability_interceptor_can_override_stdlib_dispatch() {
        let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "original") {
    state { current }
  }
}
"#;

        let engine = GraphemeEngine::builder()
            .with_capability_interceptor(|call| {
                if call.op == "echo" {
                    return Some(Ok(serde_json::json!({"message": "intercepted"})));
                }
                None
            })
            .build();

        let result = engine.execute_source(source).expect("execution should succeed");
        assert_eq!(
            result
                .final_state
                .get("current")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str()),
            Some("intercepted")
        );
    }

        #[test]
        fn sql_runtime_flow_denied_by_policy_when_not_allowlisted() {
                let source = r#"import sql from "grapheme/sql"

query SqlDenied {
    sql.query(connection: "sqlite::memory:", sql: "select 1 as ok") {
        state { current }
    }
}
"#;

                let engine = GraphemeEngine::builder().build();
                let result = engine.execute_source(source).expect("execution should complete");

                assert!(matches!(result.execution.outcome, ExecutionOutcome::FatalFailure));
                assert!(result
                        .execution
                        .message
                        .as_deref()
                        .unwrap_or_default()
                        .contains("sql module is disabled"));
        }

        #[test]
        fn sql_runtime_flow_succeeds_when_connection_allowlisted() {
                let source = r#"import sql from "grapheme/sql"

query SqlAllowed {
    sql.query(connection: "sqlite::memory:", sql: "select 1 as ok") {
        state { current }
    }
}
"#;

                let engine = GraphemeEngine::builder()
                        .with_policy_guard(PolicyGuard {
                                allowed_sql_connections: vec!["sqlite::memory:".to_string()],
                                ..PolicyGuard::default()
                        })
                        .build();
                let result = engine.execute_source(source).expect("execution should complete");

                assert!(matches!(result.execution.outcome, ExecutionOutcome::Succeeded));
                assert_eq!(
                        result
                                .final_state
                                .get("current")
                                .and_then(|v| v.get("ok"))
                                .and_then(|v| v.as_bool()),
                        Some(true)
                );
        }

        #[test]
        fn surreal_runtime_flow_denied_by_policy_when_not_allowlisted() {
                let source = r#"import surreal from "grapheme/surreal"

query SurrealDenied {
    surreal.query(connection: "local", query: "return true;") {
        state { current }
    }
}
"#;

                let engine = GraphemeEngine::builder().build();
                let result = engine.execute_source(source).expect("execution should complete");

                assert!(matches!(result.execution.outcome, ExecutionOutcome::FatalFailure));
                assert!(result
                        .execution
                        .message
                        .as_deref()
                        .unwrap_or_default()
                        .contains("surreal module is disabled"));
        }

        #[test]
        fn surreal_runtime_flow_reaches_module_when_connection_allowlisted() {
                let source = r#"import surreal from "grapheme/surreal"

query SurrealAllowed {
    surreal.query(connection: "local", query: "return true;") {
        state { current }
    }
}
"#;

                let engine = GraphemeEngine::builder()
                        .with_policy_guard(PolicyGuard {
                                allowed_surreal_connections: vec!["local".to_string()],
                                ..PolicyGuard::default()
                        })
                        .build();
                let result = engine.execute_source(source).expect("execution should complete");

                assert!(matches!(result.execution.outcome, ExecutionOutcome::Succeeded));
                assert_eq!(
                        result
                                .final_state
                                .get("current")
                                .and_then(|v| v.get("error"))
                                .and_then(|v| v.get("code"))
                                .and_then(|v| v.as_str()),
                        Some("surreal_connection_unresolved")
                );
        }

    #[test]
    fn execute_aot_matches_base_artifact_execution_parity() {
        let source = r#"import core from "grapheme/core"

query HelloAotParity {
    core.echo(message: "hello-aot-parity") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let compiled = Compiler::compile_source_to_aot(source, CompilerOptions::default())
            .expect("compile to aot should succeed");

        let interpreted = engine
            .execute_artifact(&compiled.artifact)
            .expect("interpreted execution should succeed");
        let staged = engine
            .execute_aot(&compiled.aot)
            .expect("aot-backed execution should succeed");

        assert!(matches!(
            (&interpreted.execution.outcome, &staged.execution.outcome),
            (ExecutionOutcome::Succeeded, ExecutionOutcome::Succeeded)
                | (ExecutionOutcome::RetryableFailure, ExecutionOutcome::RetryableFailure)
                | (ExecutionOutcome::FatalFailure, ExecutionOutcome::FatalFailure)
        ));
        assert_eq!(interpreted.final_state, staged.final_state);
    }

    #[test]
    fn format_aot_supports_yaml_and_json() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let aot = engine
            .compile_source_to_aot(source)
            .expect("compile source to aot should succeed");

        let yaml = engine
            .format_aot(&aot, StructuredMode::Yaml)
            .expect("yaml aot formatting should succeed");
        let json = engine
            .format_aot(&aot, StructuredMode::Json)
            .expect("json aot formatting should succeed");

        assert!(yaml.contains("stage: stage_a"));
        assert!(json.contains("\"stage\": \"stage_a\""));
    }

    #[test]
    fn compile_source_to_aot_stage_b_emits_container_metadata() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let imports = vec![
            "grapheme.runtime.host.v1::state.read".to_string(),
            "grapheme.runtime.host.v1::state.write".to_string(),
        ];
        let aot = engine
            .compile_source_to_aot_stage_b(source, b"\0asmstageb", &imports)
            .expect("compile source to stage_b aot should succeed");

        assert!(matches!(aot.stage, grapheme_artifact::AotStage::StageB));
        assert_eq!(aot.payload.format, "grapheme.aot.stage_b.v1");
        assert!(aot.payload.workflow_wasm.is_some());
    }

    #[test]
    fn execute_aot_rejects_stage_b_outside_host_boundary() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let mut stage_a = engine
            .compile_source_to_aot(source)
            .expect("compile source to aot should succeed");

        stage_a.stage = grapheme_artifact::AotStage::StageB;
        stage_a.payload.format = "grapheme.aot.stage_b.v1".to_string();
        stage_a.payload.workflow_wasm = Some(grapheme_artifact::AotWorkflowWasmContainer {
            byte_len: 8,
            sha256: "sha256:deadbeef".to_string(),
            entry_export: "_start".to_string(),
            allowed_imports: vec!["wasi_snapshot_preview1::fd_write".to_string()],
            inline_wasm_hex: None,
        });

        let err = engine
            .execute_aot(&stage_a)
            .expect_err("stage_b boundary escape should be rejected");

        assert!(matches!(err, GraphemeSdkError::Contract(_)));
        assert!(err.to_string().contains("outside host interface boundary"));
    }

    #[test]
    fn execute_aot_stage_b_routes_through_runtime_stage_b_path() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let imports = vec![
            "grapheme.runtime.host.v1::state.read".to_string(),
            "grapheme.runtime.host.v1::state.write".to_string(),
        ];
        let stage_b = engine
            .compile_source_to_aot_stage_b(source, b"\0asmstageb", &imports)
            .expect("compile source to stage_b should succeed");

        let result = engine
            .execute_aot(&stage_b)
            .expect("stage_b execution should succeed");

        assert!(result
            .execution
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("stage_b scaffold executed via parity path")
            || result
                .execution
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("stage_b container executed directly via wasix backend"));

        let stage_b_event_found = result
            .final_state
            .get("runtime_events")
            .and_then(|events| events.as_array())
            .map(|events| {
                events.iter().any(|event| {
                    event
                        .get("kind")
                        .and_then(|v| v.as_str())
                        == Some("aot.stage_b.container_routed")
                })
            })
            .unwrap_or(false);

        assert!(stage_b_event_found);
    }

    #[cfg(not(feature = "wasix-runtime"))]
    #[test]
    fn execute_aot_stage_b_strict_mode_rejects_when_container_runtime_unavailable() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder()
            .with_strict_stage_b_container_execution(true)
            .build();
        let imports = vec![
            "grapheme.runtime.host.v1::state.read".to_string(),
            "grapheme.runtime.host.v1::state.write".to_string(),
        ];
        let stage_b = engine
            .compile_source_to_aot_stage_b(source, b"\0asmstageb", &imports)
            .expect("compile source to stage_b should succeed");

        let err = engine
            .execute_aot(&stage_b)
            .expect_err("strict mode should reject fallback when container runtime is unavailable");

        assert!(matches!(err, GraphemeSdkError::Contract(_)));
        assert!(err
            .to_string()
            .contains("strict stage_b container execution required"));
    }
}
