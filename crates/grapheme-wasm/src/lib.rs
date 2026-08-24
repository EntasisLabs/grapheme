//! Grapheme runtime-in-Wasm engine (RFC-0006).
//!
//! Links the compiler + `RuntimeEngine` + wasm-safe stdlib so Grapheme can
//! **run inside** a Wasm host (WASI first). This is not a Stage B workflow
//! container and not a Wasix plugin runner.

pub mod host;

use grapheme_artifact::{ArtifactEnvelope, ExecutionOutcome, ExecutionResult};
use grapheme_compiler::verifier::LintWarning;
use grapheme_compiler::{Compiler, CompilerOptions};
use grapheme_runtime::{RuntimeEngine, RuntimeOptions};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use host::WasmStdlibHost;

/// Execute request accepted on WASI stdin (direct JSON or Wasix `{ args }` envelope).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    /// Grapheme source. Mutually exclusive with `artifact`.
    #[serde(default)]
    pub source: Option<String>,
    /// Precompiled artifact envelope. Mutually exclusive with `source`.
    #[serde(default)]
    pub artifact: Option<ArtifactEnvelope>,
    #[serde(default)]
    pub initial_current: Option<JsonValue>,
    #[serde(default)]
    pub args: Option<JsonValue>,
    #[serde(default)]
    pub entrypoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct WasixRequestEnvelope {
    #[serde(default)]
    #[allow(dead_code)]
    module: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    op: Option<String>,
    #[serde(default)]
    args: Option<JsonValue>,
}

/// Successful or failed execute payload written to stdout.
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_state: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lint_warnings: Vec<LintWarning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ExecuteError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecuteError {
    pub code: String,
    pub message: String,
}

impl ExecuteResponse {
    fn success(
        artifact_id: String,
        execution: ExecutionResult,
        final_state: JsonValue,
        lint_warnings: Vec<LintWarning>,
    ) -> Self {
        let ok = matches!(execution.outcome, ExecutionOutcome::Succeeded);
        let error = if ok {
            None
        } else {
            let code = match execution.outcome {
                ExecutionOutcome::Succeeded => "SUCCEEDED",
                ExecutionOutcome::RetryableFailure => "RETRYABLE_FAILURE",
                ExecutionOutcome::FatalFailure => "FATAL_FAILURE",
            };
            Some(ExecuteError {
                code: code.to_string(),
                message: execution
                    .message
                    .clone()
                    .unwrap_or_else(|| "execution did not succeed".to_string()),
            })
        };
        Self {
            ok,
            artifact_id: Some(artifact_id),
            execution: Some(execution),
            final_state: Some(final_state),
            lint_warnings,
            error,
        }
    }

    fn fail(code: &str, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            artifact_id: None,
            execution: None,
            final_state: None,
            lint_warnings: Vec::new(),
            error: Some(ExecuteError {
                code: code.to_string(),
                message: message.into(),
            }),
        }
    }
}

/// Compile (if needed) and execute using the in-Wasm `RuntimeEngine`.
pub fn execute(request: &ExecuteRequest) -> ExecuteResponse {
    let (artifact, lint_warnings) = match resolve_artifact(request) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let mut options = RuntimeOptions::default();
    options.strict_stage_b_container_execution = false;
    options.prefer_stage_b_wasix = false;
    if let Some(initial) = request.initial_current.clone() {
        options.initial_state_current = Some(initial);
    }
    if let Some(args) = request.args.clone() {
        options.entrypoint_args = Some(args);
    }

    let runtime = RuntimeEngine::new(options);
    let mut host = WasmStdlibHost;
    match runtime.execute_artifact(&artifact, &mut host) {
        Ok((state, execution)) => ExecuteResponse::success(
            artifact.artifact_id,
            execution,
            state.to_json(),
            lint_warnings,
        ),
        Err(err) => ExecuteResponse::fail("RUNTIME_ERROR", err.to_string()),
    }
}

pub fn execute_to_json(request: &ExecuteRequest) -> JsonValue {
    serde_json::to_value(execute(request)).unwrap_or_else(|_| json!({
        "ok": false,
        "error": { "code": "SERIALIZE_FAILED", "message": "failed to serialize execute response" }
    }))
}

/// Parse stdin JSON supporting both direct `ExecuteRequest` and Wasix envelopes.
pub fn parse_stdin_request(raw: &str) -> Result<ExecuteRequest, String> {
    let value: JsonValue =
        serde_json::from_str(raw).map_err(|e| format!("invalid request json: {e}"))?;

    if value.get("source").is_some() || value.get("artifact").is_some() {
        return serde_json::from_value(value).map_err(|e| format!("invalid execute request: {e}"));
    }

    let envelope: WasixRequestEnvelope = serde_json::from_value(value)
        .map_err(|e| format!("invalid wasix request envelope: {e}"))?;
    let args = envelope.args.unwrap_or(JsonValue::Null);
    if args.get("source").is_some() || args.get("artifact").is_some() {
        return serde_json::from_value(args)
            .map_err(|e| format!("invalid execute request in wasix args: {e}"));
    }

    Err("request must include source or artifact at the root or under wasix args".to_string())
}

fn resolve_artifact(
    request: &ExecuteRequest,
) -> Result<(ArtifactEnvelope, Vec<LintWarning>), ExecuteResponse> {
    match (&request.source, &request.artifact) {
        (Some(source), None) => {
            let mut options = CompilerOptions::default();
            options.entrypoint = request.entrypoint.clone();
            match Compiler::compile_source(source, options) {
                Ok(compiled) => Ok((compiled.artifact, compiled.compilation.lint_warnings)),
                Err(err) => Err(ExecuteResponse::fail("COMPILE_ERROR", err.to_string())),
            }
        }
        (None, Some(artifact)) => Ok((artifact.clone(), Vec::new())),
        (Some(_), Some(_)) => Err(ExecuteResponse::fail(
            "INVALID_REQUEST",
            "provide either source or artifact, not both",
        )),
        (None, None) => Err(ExecuteResponse::fail(
            "INVALID_REQUEST",
            "request must include source or artifact",
        )),
    }
}
