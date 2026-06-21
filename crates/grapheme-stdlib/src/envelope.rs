//! Normalized host capability result envelope for Grapheme 0.6.0+.
//!
//! Target shape: `{ data, meta, error }` with dual-read compatibility for legacy
//! flat JSON objects during migration.

use serde_json::{json, Value as JsonValue};

pub const ENVELOPE_SCHEMA: &str = "grapheme.host.result.envelope/v1";

/// Normalize a raw module response into the standard host envelope.
pub fn normalize(raw: JsonValue) -> JsonValue {
    if is_envelope(&raw) {
        return raw;
    }

    let error = raw
        .get("error")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);

    if error.is_some() {
        return json!({
            "data": raw.get("data").cloned().unwrap_or(JsonValue::Null),
            "meta": merge_meta(raw.get("meta"), json!({ "legacy_flat": true })),
            "error": error,
        });
    }

    json!({
        "data": raw,
        "meta": json!({ "legacy_flat": true }),
        "error": null,
    })
}

/// Build a success envelope.
pub fn success(data: JsonValue) -> JsonValue {
    json!({
        "data": data,
        "meta": json!({ "schema": ENVELOPE_SCHEMA }),
        "error": null,
    })
}

/// Build a failure envelope.
pub fn failure(error: impl Into<String>) -> JsonValue {
    json!({
        "data": null,
        "meta": json!({ "schema": ENVELOPE_SCHEMA }),
        "error": error.into(),
    })
}

/// Read the primary payload from either envelope or legacy flat JSON.
pub fn data<'a>(value: &'a JsonValue) -> &'a JsonValue {
    if is_envelope(value) {
        return value.get("data").unwrap_or(value);
    }
    value
}

pub fn is_envelope(value: &JsonValue) -> bool {
    value.as_object().is_some_and(|obj| {
        obj.contains_key("data") && obj.contains_key("meta") && obj.contains_key("error")
    })
}

fn merge_meta(existing: Option<&JsonValue>, extra: JsonValue) -> JsonValue {
    let Some(JsonValue::Object(map)) = existing else {
        return extra;
    };

    let Some(extra_map) = extra.as_object() else {
        return JsonValue::Object(map.clone());
    };

    let mut merged = map.clone();
    for (key, value) in extra_map.clone() {
        merged.entry(key).or_insert(value);
    }
    JsonValue::Object(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn wraps_legacy_flat_object() {
        let out = normalize(json!({ "accepted": true, "count": 3 }));
        assert_eq!(out.get("data").and_then(|v| v.get("count")).and_then(|v| v.as_u64()), Some(3));
        assert!(out.get("error").and_then(|v| v.as_null()).is_some());
    }

    #[test]
    fn preserves_existing_envelope() {
        let input = json!({ "data": { "ok": true }, "meta": {}, "error": null });
        assert_eq!(normalize(input.clone()), input);
    }
}
