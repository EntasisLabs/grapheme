use serde_json::{json, Value as JsonValue};

pub fn parse(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    match serde_json::from_str::<JsonValue>(&text) {
        Ok(value) => value,
        Err(err) => json!({ "error": format!("json parse failed: {err}") }),
    }
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| args.get("__input").and_then(|v| v.as_str()).map(ToOwned::to_owned))
        .unwrap_or_default()
}
