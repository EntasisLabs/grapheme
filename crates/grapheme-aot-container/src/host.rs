//! Host-import stubs for ops outside the Wasm stdlib profile.

use serde_json::{json, Value as JsonValue};

pub const HOST_INTERFACE: &str = "grapheme.runtime.host.v1";
pub const CALL_CAPABILITY_IMPORT: &str = "grapheme.runtime.host.v1::call.capability";
pub const STATE_READ_IMPORT: &str = "grapheme.runtime.host.v1::state.read";
pub const STATE_WRITE_IMPORT: &str = "grapheme.runtime.host.v1::state.write";

#[derive(Debug, Clone)]
pub struct HostCallRequest {
    pub import: String,
    pub module: String,
    pub op: String,
    pub capability: String,
    pub args: JsonValue,
}

/// v1 stub: Stage B containers do not yet link real Wasm imports.
/// Non-local capability ops fail with a structured host-call marker so the
/// host/runtime can detect the boundary and (later) fulfill the import.
pub fn stub_call_capability(req: &HostCallRequest) -> JsonValue {
    json!({
        "error": {
            "code": "host_call_required",
            "import": req.import,
            "module": req.module,
            "op": req.op,
            "capability": req.capability,
            "args": req.args,
            "message": format!(
                "capability '{}.{}' is outside the Wasm stdlib profile; host must provide {}",
                req.module, req.op, req.import
            )
        }
    })
}

pub fn is_host_call_error(value: &JsonValue) -> bool {
    value
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_str())
        == Some("host_call_required")
}
