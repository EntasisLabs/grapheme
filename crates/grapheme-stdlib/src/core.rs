use serde_json::{json, Map, Value as JsonValue};

pub fn echo(args: &JsonValue) -> JsonValue {
    let message = core_arg_message(args, "message");
    json!({ "message": message })
}

pub fn tap(args: &JsonValue) -> JsonValue {
    args.get("__input").cloned().unwrap_or_else(|| {
        let message = core_arg_message(args, "message");
        serde_json::json!({ "message": message })
    })
}

pub fn pack_state_data(args: &JsonValue) -> JsonValue {
    let state = args.get("state").cloned().unwrap_or(JsonValue::Null);
    let data = args
        .get("data")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or(JsonValue::Null);

    serde_json::json!({
        "state": state,
        "data": data,
    })
}

pub fn get_state(args: &JsonValue) -> JsonValue {
    args.get("input")
        .or_else(|| args.get("__input"))
        .and_then(|v| v.get("state"))
        .cloned()
        .unwrap_or(JsonValue::Null)
}

pub fn get_data(args: &JsonValue) -> JsonValue {
    args.get("input")
        .or_else(|| args.get("__input"))
        .and_then(|v| v.get("data"))
        .cloned()
        .unwrap_or(JsonValue::Null)
}

pub fn apply_lane(args: &JsonValue) -> JsonValue {
    let lane = args.get("lane").and_then(|v| v.as_str()).unwrap_or("");
    let fields = args
        .get("fields")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let input = args
        .get("input")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or(JsonValue::Null);

    let mut out = match input {
        JsonValue::Object(map) => map,
        other => {
            let mut map = Map::new();
            map.insert("data".to_string(), other);
            map
        }
    };

    let mut lane_obj = out
        .get(lane)
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    for (k, v) in fields {
        lane_obj.insert(k, v);
    }

    out.insert(lane.to_string(), JsonValue::Object(lane_obj));
    JsonValue::Object(out)
}

pub fn map(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let Some(field) = args.get("field").and_then(|v| v.as_str()) else {
        return JsonValue::Array(items);
    };

    JsonValue::Array(
        items
            .into_iter()
            .map(|item| item.get(field).cloned().unwrap_or(JsonValue::Null))
            .collect(),
    )
}

pub fn filter(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let Some(field) = args.get("field").and_then(|v| v.as_str()) else {
        return JsonValue::Array(items);
    };
    let equals = args.get("equals").cloned().unwrap_or(JsonValue::Null);

    JsonValue::Array(
        items
            .into_iter()
            .filter(|item| item.get(field).map(|v| v == &equals).unwrap_or(false))
            .collect(),
    )
}

pub fn find(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let Some(field) = args.get("field").and_then(|v| v.as_str()) else {
        return JsonValue::Null;
    };
    let equals = args.get("equals").cloned().unwrap_or(JsonValue::Null);

    items
        .into_iter()
        .find(|item| item.get(field).map(|v| v == &equals).unwrap_or(false))
        .unwrap_or(JsonValue::Null)
}

pub fn reduce(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("last");

    match mode {
        "sum" => {
            let sum = items
                .iter()
                .filter_map(|v| v.as_f64())
                .fold(0.0, |acc, n| acc + n);
            json!(sum)
        }
        "min" => items
            .iter()
            .filter_map(|v| v.as_f64())
            .reduce(f64::min)
            .map(|v| json!(v))
            .unwrap_or(JsonValue::Null),
        "max" => items
            .iter()
            .filter_map(|v| v.as_f64())
            .reduce(f64::max)
            .map(|v| json!(v))
            .unwrap_or(JsonValue::Null),
        "avg" => {
            let nums = items.iter().filter_map(|v| v.as_f64()).collect::<Vec<_>>();
            if nums.is_empty() {
                JsonValue::Null
            } else {
                json!(nums.iter().sum::<f64>() / nums.len() as f64)
            }
        }
        "concat" => {
            let initial = args
                .get("initial")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let joined = items
                .iter()
                .map(core_scalar_to_key)
                .collect::<Vec<_>>()
                .join("");
            json!(format!("{initial}{joined}"))
        }
        "count" => json!(items.len()),
        "first" => items.first().cloned().unwrap_or(JsonValue::Null),
        "last" => items.last().cloned().unwrap_or(JsonValue::Null),
        _ => json!({
            "error": format!("unsupported reduce mode '{mode}'"),
            "supported_modes": ["sum", "min", "max", "avg", "concat", "count", "first", "last"]
        }),
    }
}

pub fn group_by(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let Some(field) = args.get("field").and_then(|v| v.as_str()) else {
        return JsonValue::Object(Map::new());
    };

    let mut grouped = Map::new();
    for item in items {
        let key = item
            .get(field)
            .map(core_scalar_to_key)
            .unwrap_or_else(|| "null".to_string());

        let entry = grouped
            .entry(key)
            .or_insert_with(|| JsonValue::Array(Vec::new()));
        if let Some(values) = entry.as_array_mut() {
            values.push(item);
        }
    }

    JsonValue::Object(grouped)
}

pub fn merge(args: &JsonValue) -> JsonValue {
    let left = args
        .get("left")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| args.get("__input").and_then(|v| v.as_object()).cloned())
        .unwrap_or_default();

    let right = args
        .get("right")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut merged = left;
    for (k, v) in right {
        merged.insert(k, v);
    }

    JsonValue::Object(merged)
}

pub fn pick(args: &JsonValue) -> JsonValue {
    let input = args
        .get("input")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| args.get("__input").and_then(|v| v.as_object()).cloned())
        .unwrap_or_default();

    let fields = args
        .get("fields")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut picked = Map::new();
    for field in fields.iter().filter_map(|v| v.as_str()) {
        if let Some(value) = input.get(field) {
            picked.insert(field.to_string(), value.clone());
        }
    }
    JsonValue::Object(picked)
}

pub fn validate_schema(args: &JsonValue) -> JsonValue {
    let required = args
        .get("required")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let data = args
        .get("data")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let missing = required
        .iter()
        .filter_map(|v| v.as_str())
        .filter(|name| !data.contains_key(*name))
        .map(|s| JsonValue::String(s.to_string()))
        .collect::<Vec<_>>();

    json!({
        "ok": missing.is_empty(),
        "missing": missing,
    })
}

pub fn add(args: &JsonValue) -> JsonValue {
    let Some(a) = core_arg_f64(args, "a") else {
        return json!({ "error": "missing numeric arg: a" });
    };
    let Some(b) = core_arg_f64(args, "b") else {
        return json!({ "error": "missing numeric arg: b" });
    };
    json!(a + b)
}

pub fn sub(args: &JsonValue) -> JsonValue {
    let Some(a) = core_arg_f64(args, "a") else {
        return json!({ "error": "missing numeric arg: a" });
    };
    let Some(b) = core_arg_f64(args, "b") else {
        return json!({ "error": "missing numeric arg: b" });
    };
    json!(a - b)
}

pub fn inc(args: &JsonValue) -> JsonValue {
    let value = core_arg_f64(args, "value")
        .or_else(|| args.get("__input").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    json!(value + 1.0)
}

pub fn dec(args: &JsonValue) -> JsonValue {
    let value = core_arg_f64(args, "value")
        .or_else(|| args.get("__input").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    json!(value - 1.0)
}

pub fn eq(args: &JsonValue) -> JsonValue {
    let a = args.get("a").cloned().unwrap_or(JsonValue::Null);
    let b = args.get("b").cloned().unwrap_or(JsonValue::Null);
    json!({ "value": a == b })
}

pub fn lt(args: &JsonValue) -> JsonValue {
    compare_numeric(args, |a, b| a < b)
}

pub fn gt(args: &JsonValue) -> JsonValue {
    compare_numeric(args, |a, b| a > b)
}

pub fn gte(args: &JsonValue) -> JsonValue {
    compare_numeric(args, |a, b| a >= b)
}

pub fn lte(args: &JsonValue) -> JsonValue {
    compare_numeric(args, |a, b| a <= b)
}

pub fn inc_field(args: &JsonValue) -> JsonValue {
    bump_field(args, 1.0)
}

pub fn dec_field(args: &JsonValue) -> JsonValue {
    bump_field(args, -1.0)
}

pub fn set_fields(args: &JsonValue) -> JsonValue {
    let fields = args
        .get("fields")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut input = args
        .get("input")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| args.get("__input").and_then(|v| v.as_object()).cloned())
        .unwrap_or_default();

    for (key, value) in fields {
        input.insert(key, value);
    }

    JsonValue::Object(input)
}

pub fn split(args: &JsonValue) -> JsonValue {
    let text = core_arg_text(args, "text");
    let sep = args.get("sep").and_then(|v| v.as_str()).unwrap_or(",");

    JsonValue::Array(
        text.split(sep)
            .map(|s| JsonValue::String(s.to_string()))
            .collect(),
    )
}

pub fn join(args: &JsonValue) -> JsonValue {
    let sep = args.get("sep").and_then(|v| v.as_str()).unwrap_or(",");
    let items = core_items(args);
    let joined = items
        .iter()
        .map(core_scalar_to_key)
        .collect::<Vec<_>>()
        .join(sep);
    json!({ "text": joined })
}

pub fn replace(args: &JsonValue) -> JsonValue {
    let text = core_arg_text(args, "text");
    let from = args.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let to = args.get("to").and_then(|v| v.as_str()).unwrap_or("");
    json!({ "text": text.replace(from, to) })
}

pub fn trim(args: &JsonValue) -> JsonValue {
    let text = core_arg_text(args, "text");
    json!({ "text": text.trim() })
}

pub fn lower(args: &JsonValue) -> JsonValue {
    let text = core_arg_text(args, "text");
    json!({ "text": text.to_lowercase() })
}

pub fn upper(args: &JsonValue) -> JsonValue {
    let text = core_arg_text(args, "text");
    json!({ "text": text.to_uppercase() })
}

pub fn contains(args: &JsonValue) -> JsonValue {
    let haystack = args
        .get("haystack")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or(JsonValue::Null);
    let needle = args.get("needle").cloned().unwrap_or(JsonValue::Null);

    let contains = match &haystack {
        JsonValue::String(s) => needle.as_str().map(|n| s.contains(n)).unwrap_or(false),
        JsonValue::Array(items) => items.iter().any(|item| item == &needle),
        JsonValue::Object(map) => needle.as_str().map(|k| map.contains_key(k)).unwrap_or(false),
        _ => false,
    };

    json!({ "contains": contains })
}

pub fn get_path(args: &JsonValue) -> JsonValue {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let input = args
        .get("input")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or(JsonValue::Null);

    json_get_path_value(&input, path).unwrap_or(JsonValue::Null)
}

pub fn set_path(args: &JsonValue) -> JsonValue {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let value = args.get("value").cloned().unwrap_or(JsonValue::Null);
    let input = args
        .get("input")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or_else(|| JsonValue::Object(Map::new()));

    json_set_path_value(&input, path, value)
}

pub fn has_path(args: &JsonValue) -> JsonValue {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let input = args
        .get("input")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or(JsonValue::Null);

    json!({ "has_path": json_get_path_value(&input, path).is_some() })
}

fn core_items(args: &JsonValue) -> Vec<JsonValue> {
    args.get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .or_else(|| args.get("__input").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default()
}

fn core_scalar_to_key(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "<value>".to_string()),
    }
}

fn json_get_path_value(input: &JsonValue, path: &str) -> Option<JsonValue> {
    let segments = path.split('.').filter(|s| !s.is_empty()).collect::<Vec<_>>();
    if segments.is_empty() {
        return Some(input.clone());
    }

    let mut current = input;
    for segment in segments {
        current = match current {
            JsonValue::Object(map) => map.get(segment)?,
            JsonValue::Array(items) => {
                let idx = segment.parse::<usize>().ok()?;
                items.get(idx)?
            }
            _ => return None,
        };
    }

    Some(current.clone())
}

fn json_set_path_value(input: &JsonValue, path: &str, value: JsonValue) -> JsonValue {
    let segments = path.split('.').filter(|s| !s.is_empty()).collect::<Vec<_>>();
    if segments.is_empty() {
        return input.clone();
    }

    let mut output = if input.is_object() {
        input.clone()
    } else {
        JsonValue::Object(Map::new())
    };

    set_path_recursive(&mut output, &segments, value);
    output
}

fn set_path_recursive(node: &mut JsonValue, segments: &[&str], value: JsonValue) {
    if segments.is_empty() {
        return;
    }

    if segments.len() == 1 {
        if !node.is_object() {
            *node = JsonValue::Object(Map::new());
        }
        if let Some(map) = node.as_object_mut() {
            map.insert(segments[0].to_string(), value);
        }
        return;
    }

    if !node.is_object() {
        *node = JsonValue::Object(Map::new());
    }

    if let Some(map) = node.as_object_mut() {
        let child = map
            .entry(segments[0].to_string())
            .or_insert_with(|| JsonValue::Object(Map::new()));
        set_path_recursive(child, &segments[1..], value);
    }
}

fn core_arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| args.get("__input").and_then(|v| v.as_str()).map(ToOwned::to_owned))
        .unwrap_or_default()
}

    fn core_arg_message(args: &JsonValue, key: &str) -> String {
        args.get(key)
        .map(core_scalar_to_key)
        .unwrap_or_default()
    }

fn core_arg_f64(args: &JsonValue, key: &str) -> Option<f64> {
    args.get(key)
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
}

fn compare_numeric(args: &JsonValue, pred: fn(f64, f64) -> bool) -> JsonValue {
    let Some(a) = core_arg_f64(args, "a") else {
        return json!({ "error": "missing numeric arg: a" });
    };
    let Some(b) = core_arg_f64(args, "b") else {
        return json!({ "error": "missing numeric arg: b" });
    };
    json!({ "value": pred(a, b) })
}

fn bump_field(args: &JsonValue, delta: f64) -> JsonValue {
    let field = args.get("field").and_then(|v| v.as_str()).unwrap_or("");
    if field.is_empty() {
        return json!({ "error": "missing arg: field" });
    }

    let mut input = args
        .get("input")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| args.get("__input").and_then(|v| v.as_object()).cloned())
        .unwrap_or_default();

    let current = input.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0);
    input.insert(field.to_string(), json!(current + delta));
    JsonValue::Object(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_accepts_numeric_message_values() {
        let out = echo(&json!({ "message": 42 }));
        assert_eq!(out, json!({ "message": "42" }));
    }

    #[test]
    fn echo_accepts_object_message_values() {
        let out = echo(&json!({ "message": { "a": 1, "b": true } }));
        assert_eq!(out, json!({ "message": "{\"a\":1,\"b\":true}" }));
    }

    #[test]
    fn tap_preserves_input_when_present() {
        let out = tap(&json!({ "__input": { "value": 1 }, "message": "ignored" }));
        assert_eq!(out, json!({ "value": 1 }));
    }
}
