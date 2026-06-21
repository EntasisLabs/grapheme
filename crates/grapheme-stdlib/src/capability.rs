//! Shared helpers for opt-in 0.6.0 capability modules.

use crate::envelope;
use serde_json::{json, Value as JsonValue};

pub struct CapabilityResponse;

impl CapabilityResponse {
    pub fn scaffold(op: &str, note: &str) -> JsonValue {
        envelope::success(json!({
            "ok": false,
            "op": op,
            "status": "scaffold",
            "note": note,
            "release": "0.6.0",
        }))
    }

    pub fn invalid_args(message: &str) -> JsonValue {
        envelope::failure(message)
    }
}
