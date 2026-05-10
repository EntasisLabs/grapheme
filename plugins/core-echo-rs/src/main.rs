use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::io::{self, Read, Write};

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    args: Value,
}

fn arg_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
}

fn write_json(value: &Value) {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let mut stdout = io::stdout();
    let _ = stdout.write_all(&bytes);
    let _ = stdout.flush();
}

fn op_echo(args: &Value) -> Value {
    let message = if let Some(message) = arg_string(args, "message") {
        message
    } else if let Some(input) = args.get("__input") {
        serde_json::to_string_pretty(input).unwrap_or_default()
    } else {
        String::new()
    };
    json!({ "message": message })
}

fn op_pick(args: &Value) -> Value {
    let source = args
        .get("input")
        .or_else(|| args.get("__input"))
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_else(Map::new);

    let fields = args
        .get("fields")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut out = Map::new();
    for field in fields {
        if let Some(name) = field.as_str() {
            if let Some(value) = source.get(name) {
                out.insert(name.to_string(), value.clone());
            }
        }
    }

    Value::Object(out)
}

fn op_map(args: &Value) -> Value {
    let identifier = arg_string(args, "identifier").unwrap_or_else(|| "items".to_string());

    let items = if let Some(input) = args.get("__input") {
        input
            .get(&identifier)

            .unwrap_or_default()
    } else {
        args.get(&identifier)
            // .and_then(|v| v.as_array())
            // .cloned()
            .unwrap_or_default()
    };
    let field = arg_string(args, "field");

    let mapped = if let Some(field) = field {
        write_json(items);
        items
        .get(&field)
        .unwrap_or_default()
    } else {
        items
    };

    json!({ "items": mapped })
}

fn op_filter(args: &Value) -> Value {
    let items = args
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let field = arg_string(args, "field").unwrap_or_default();
    let equals_value = args.get("equals").cloned().unwrap_or(Value::Null);

    let filtered = items
        .into_iter()
        .filter(|item| item.get(&field).cloned().unwrap_or(Value::Null) == equals_value)
        .collect::<Vec<_>>();

    json!({ "items": filtered })
}

fn op_merge(args: &Value) -> Value {
    let left = args
        .get("left")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_else(Map::new);
    let right = args
        .get("right")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_else(Map::new);

    let mut merged = left;
    for (k, v) in right {
        merged.insert(k, v);
    }

    Value::Object(merged)
}

fn op_validate_schema(args: &Value) -> Value {
    let required = args
        .get("required")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let data = args
        .get("data")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_else(Map::new);

    let mut missing = Vec::new();
    for key in required {
        if let Some(name) = key.as_str() {
            if !data.contains_key(name) {
                missing.push(name.to_string());
            }
        }
    }

    json!({
        "valid": missing.is_empty(),
        "missing": missing,
    })
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        write_json(&json!({ "error": "failed to read request" }));
        return;
    }

    let request: Request = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => {
            write_json(&json!({ "error": "invalid request json" }));
            return;
        }
    };

    let output = match request.op.as_str() {
        "echo" => op_echo(&request.args),
        "pick" => op_pick(&request.args),
        "map" => op_map(&request.args),
        "filter" => op_filter(&request.args),
        "merge" => op_merge(&request.args),
        "validate_schema" => op_validate_schema(&request.args),
        other => json!({ "error": format!("unsupported core op: {other}") }),
    };

    write_json(&output);
}
