//! Stage B AOT workflow container.
//!
//! Links `grapheme-stdlib` with the Wasm-safe profile and walks MIR inside a
//! WASI binary. Capability ops outside that profile surface as
//! `grapheme.runtime.host.v1::call.capability` stubs (RFC-0005 step 2).

pub mod host;
pub mod templates;
pub mod walk;

use grapheme_artifact::MirProgram;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use walk::{walk_program, walk_result_to_json, WalkError, WalkResult};

pub use host::{
    CALL_CAPABILITY_IMPORT, HOST_INTERFACE, STATE_READ_IMPORT, STATE_WRITE_IMPORT,
};
pub use walk::WalkResult as ContainerWalkResult;

/// Execute-request contract for the Stage B container.
///
/// Accepted either as the stdin root object or nested under Wasix
/// `{ "module", "op", "args" }` where `args` carries these fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    #[serde(default)]
    pub entrypoint: Option<String>,
    pub mir: MirProgram,
    #[serde(default)]
    pub initial_current: Option<JsonValue>,
    #[serde(default)]
    pub args: Option<JsonValue>,
    #[serde(default)]
    pub allowed_imports: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WasixRequestEnvelope {
    #[serde(default)]
    module: Option<String>,
    #[serde(default)]
    op: Option<String>,
    #[serde(default)]
    args: Option<JsonValue>,
}

pub fn execute(request: &ExecuteRequest) -> Result<WalkResult, WalkError> {
    let entrypoint = request
        .entrypoint
        .clone()
        .or_else(|| request.mir.functions.first().map(|f| f.name.clone()))
        .ok_or_else(|| WalkError {
            code: "MISSING_ENTRYPOINT".to_string(),
            message: "execute request has no entrypoint and MIR has no functions".to_string(),
            capability: None,
            step_index: 0,
        })?;

    let initial = request
        .initial_current
        .clone()
        .unwrap_or(JsonValue::Object(Default::default()));
    let call_args = request.args.clone().unwrap_or(JsonValue::Null);
    walk_program(&request.mir, &entrypoint, initial, &call_args)
}

pub fn execute_to_json(request: &ExecuteRequest) -> JsonValue {
    match execute(request) {
        Ok(result) => walk_result_to_json(&result),
        Err(err) => json!({
            "ok": false,
            "current": null,
            "steps": 0,
            "host_calls": [],
            "error": {
                "code": err.code,
                "message": err.message,
                "capability": err.capability,
                "step_index": err.step_index,
            }
        }),
    }
}

/// Parse stdin JSON supporting both direct `ExecuteRequest` and Wasix envelopes.
pub fn parse_stdin_request(raw: &str) -> Result<ExecuteRequest, String> {
    let value: JsonValue =
        serde_json::from_str(raw).map_err(|e| format!("invalid request json: {e}"))?;

    if value.get("mir").is_some() {
        return serde_json::from_value(value).map_err(|e| format!("invalid execute request: {e}"));
    }

    let envelope: WasixRequestEnvelope = serde_json::from_value(value)
        .map_err(|e| format!("invalid wasix request envelope: {e}"))?;

    let args = envelope.args.unwrap_or(JsonValue::Null);
    if args.get("mir").is_some() {
        return serde_json::from_value(args)
            .map_err(|e| format!("invalid execute request in wasix args: {e}"));
    }

    Err(
        "request must include MIR either at the root or under wasix args.mir".to_string(),
    )
}

pub fn default_allowed_imports() -> Vec<String> {
    vec![
        STATE_READ_IMPORT.to_string(),
        STATE_WRITE_IMPORT.to_string(),
        CALL_CAPABILITY_IMPORT.to_string(),
    ]
}
