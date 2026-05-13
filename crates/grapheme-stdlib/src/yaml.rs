use serde_json::{json, Value as JsonValue};

pub fn to_json(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    match serde_yaml::from_str::<serde_yaml::Value>(&text) {
        Ok(value) => match serde_json::to_value(value) {
            Ok(json) => json,
            Err(err) => json!({ "error": format!("yaml conversion failed: {err}") }),
        },
        Err(err) => json!({ "error": format!("yaml parse failed: {err}") }),
    }
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| args.get("__input").and_then(|v| v.as_str()).map(ToOwned::to_owned))
        .unwrap_or_default()
}
