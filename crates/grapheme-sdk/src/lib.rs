use grapheme_artifact::{ArtifactEnvelope, ExecutionResult};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredMode {
    Yaml,
    Json,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecuteResultPayload {
    pub artifact_id: String,
    pub execution: ExecutionResult,
    pub final_state: JsonValue,
    pub lint_warnings: Vec<LintWarning>,
}

#[derive(Debug, Error)]
pub enum GraphemeSdkError {
    #[error(transparent)]
    Compiler(#[from] CompilerError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("serialize output: {0}")]
    Serialization(String),
}

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
    pub fn new() -> Self {
        Self {
            runtime_options: RuntimeOptions::default(),
            module_bindings: HashMap::new(),
            host_factory: None,
            capability_observer: None,
            capability_interceptor: None,
        }
    }

    pub fn with_policy_guard(mut self, guard: PolicyGuard) -> Self {
        self.runtime_options.policy_guard = guard;
        self
    }

    pub fn with_trace_policy(mut self, policy: TracePolicy) -> Self {
        self.runtime_options.trace_policy = policy;
        self
    }

    pub fn with_verify_integrity(mut self, enabled: bool) -> Self {
        self.runtime_options.verify_integrity = enabled;
        self
    }

    pub fn with_stream_step_output(mut self, enabled: bool) -> Self {
        self.runtime_options.stream_step_output = enabled;
        self
    }

    pub fn with_max_steps(mut self, max_steps: Option<usize>) -> Self {
        self.runtime_options.max_steps = max_steps;
        self
    }

    pub fn with_max_call_depth(mut self, max_call_depth: Option<usize>) -> Self {
        self.runtime_options.max_call_depth = max_call_depth;
        self
    }

    pub fn with_module_path(mut self, module: &str, path: impl Into<PathBuf>) -> Self {
        self.module_bindings
            .insert(module.to_lowercase(), path.into());
        self
    }

    pub fn with_capability_observer<F>(mut self, observer: F) -> Self
    where
        F: Fn(&CapabilityCall) + Send + Sync + 'static,
    {
        self.capability_observer = Some(Arc::new(observer));
        self
    }

    pub fn with_capability_interceptor<F>(mut self, interceptor: F) -> Self
    where
        F: Fn(&CapabilityCall) -> Option<Result<JsonValue, HostCallError>> + Send + Sync + 'static,
    {
        self.capability_interceptor = Some(Arc::new(interceptor));
        self
    }

    pub fn with_host_factory<F>(mut self, host_factory: F) -> Self
    where
        F: Fn() -> Box<dyn CapabilityHost + Send> + Send + Sync + 'static,
    {
        self.host_factory = Some(Arc::new(host_factory));
        self
    }

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

pub struct GraphemeEngine {
    runtime_options: RuntimeOptions,
    module_bindings: HashMap<String, PathBuf>,
    host_factory: Option<HostFactory>,
    capability_observer: Option<CapabilityObserver>,
    capability_interceptor: Option<CapabilityInterceptor>,
}

impl GraphemeEngine {
    pub fn builder() -> GraphemeEngineBuilder {
        GraphemeEngineBuilder::new()
    }

    pub fn execute_source(&self, source: &str) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        let compiled = Compiler::compile_source(source, CompilerOptions::default())?;
        self.execute_compiled(&compiled)
    }

    pub fn execute_artifact(
        &self,
        artifact: &ArtifactEnvelope,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        self.execute_artifact_with_lints(artifact, Vec::new())
    }

    pub fn execute_compiled(
        &self,
        compiled: &CompiledScript,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        self.execute_artifact_with_lints(
            &compiled.artifact,
            compiled.compilation.lint_warnings.clone(),
        )
    }

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
}
