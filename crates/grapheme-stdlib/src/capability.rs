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

    pub fn invalid_args(message: impl AsRef<str>) -> JsonValue {
        envelope::failure(message.as_ref())
    }

    pub fn ok(payload: JsonValue) -> JsonValue {
        let body = match payload {
            JsonValue::Object(mut map) => {
                map.entry("ok".to_string()).or_insert(json!(true));
                JsonValue::Object(map)
            }
            other => json!({ "ok": true, "value": other }),
        };
        envelope::success(body)
    }
}
