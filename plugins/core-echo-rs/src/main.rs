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

    if let Some(input_obj) = args.get("__input").and_then(|v| v.as_object()) {
        let mut out = input_obj.clone();
        out.insert("message".to_string(), json!(message));
        return Value::Object(out);
    }

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

fn arg_number(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn op_add(args: &Value) -> Value {
    let a = arg_number(args, "a").unwrap_or(0.0);
    let b = arg_number(args, "b").unwrap_or(0.0);
    with_value_from_input(args, a + b)
}

fn op_sub(args: &Value) -> Value {
    let a = arg_number(args, "a").unwrap_or(0.0);
    let b = arg_number(args, "b").unwrap_or(0.0);
    with_value_from_input(args, a - b)
}

fn op_inc(args: &Value) -> Value {
    let value = arg_number(args, "value")
        .or_else(|| args.get("__input").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    with_value_from_input(args, value + 1.0)
}

fn op_dec(args: &Value) -> Value {
    let value = arg_number(args, "value")
        .or_else(|| args.get("__input").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    with_value_from_input(args, value - 1.0)
}

fn with_value_from_input(args: &Value, value: f64) -> Value {
    if let Some(input_obj) = args.get("__input").and_then(|v| v.as_object()) {
        let mut out = input_obj.clone();
        out.insert("value".to_string(), json!(value));
        return Value::Object(out);
    }

    json!({ "value": value })
}

fn op_eq(args: &Value) -> Value {
    let a = args.get("a").cloned().unwrap_or(Value::Null);
    let b = args.get("b").cloned().unwrap_or(Value::Null);
    with_result_from_input(args, a == b)
}

fn op_lt(args: &Value) -> Value {
    let a = arg_number(args, "a").unwrap_or(0.0);
    let b = arg_number(args, "b").unwrap_or(0.0);
    with_result_from_input(args, a < b)
}

fn op_gt(args: &Value) -> Value {
    let a = arg_number(args, "a").unwrap_or(0.0);
    let b = arg_number(args, "b").unwrap_or(0.0);
    with_result_from_input(args, a > b)
}

fn op_gte(args: &Value) -> Value {
    let a = arg_number(args, "a").unwrap_or(0.0);
    let b = arg_number(args, "b").unwrap_or(0.0);
    with_result_from_input(args, a >= b)
}

fn op_lte(args: &Value) -> Value {
    let a = arg_number(args, "a").unwrap_or(0.0);
    let b = arg_number(args, "b").unwrap_or(0.0);
    with_result_from_input(args, a <= b)
}

fn object_input(args: &Value) -> Map<String, Value> {
    args
        .get("input")
        .or_else(|| args.get("__input"))
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_else(Map::new)
}

fn op_inc_field(args: &Value) -> Value {
    let field = arg_string(args, "field").unwrap_or_default();
    let mut out = object_input(args);
    if field.is_empty() {
        return Value::Object(out);
    }

    let current = out.get(&field).and_then(|v| v.as_f64()).unwrap_or(0.0);
    out.insert(field, json!(current + 1.0));
    Value::Object(out)
}

fn op_dec_field(args: &Value) -> Value {
    let field = arg_string(args, "field").unwrap_or_default();
    let mut out = object_input(args);
    if field.is_empty() {
        return Value::Object(out);
    }

    let current = out.get(&field).and_then(|v| v.as_f64()).unwrap_or(0.0);
    out.insert(field, json!(current - 1.0));
    Value::Object(out)
}

fn op_set_fields(args: &Value) -> Value {
    let mut out = object_input(args);
    let Some(fields) = args.get("fields").and_then(|v| v.as_object()) else {
        return Value::Object(out);
    };

    for (key, value) in fields {
        out.insert(key.clone(), value.clone());
    }

    Value::Object(out)
}

fn with_result_from_input(args: &Value, result: bool) -> Value {
    if let Some(input_obj) = args.get("__input").and_then(|v| v.as_object()) {
        let mut out = input_obj.clone();
        out.insert("result".to_string(), json!(result));
        return Value::Object(out);
    }

    json!({ "result": result })
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
        "add" => op_add(&request.args),
        "sub" => op_sub(&request.args),
        "inc" => op_inc(&request.args),
        "dec" => op_dec(&request.args),
        "eq" => op_eq(&request.args),
        "lt" => op_lt(&request.args),
        "gt" => op_gt(&request.args),
        "gte" => op_gte(&request.args),
        "lte" => op_lte(&request.args),
        "inc_field" => op_inc_field(&request.args),
        "dec_field" => op_dec_field(&request.args),
        "set_fields" => op_set_fields(&request.args),
        other => json!({ "error": format!("unsupported core op: {other}") }),
    };

    write_json(&output);
}
