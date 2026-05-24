use serde_json::{json, Value as JsonValue};

pub fn to_list(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(text.as_bytes());

    let headers = match reader.headers() {
        Ok(h) => h.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        Err(err) => return json!({ "error": format!("csv header parse failed: {err}") }),
    };

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = match record {
            Ok(r) => r,
            Err(err) => return json!({ "error": format!("csv row parse failed: {err}") }),
        };

        let mut obj = serde_json::Map::new();
        for (idx, value) in record.iter().enumerate() {
            let key = headers
                .get(idx)
                .cloned()
                .unwrap_or_else(|| format!("col_{idx}"));
            obj.insert(key, JsonValue::String(value.to_string()));
        }
        rows.push(JsonValue::Object(obj));
    }

    JsonValue::Array(rows)
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default()
}
