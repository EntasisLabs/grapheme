use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::{self, Read, Write};

const STORE_PATH: &str = ".grapheme-memory-store.json";

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    args: Value,
}

fn read_store() -> Map<String, Value> {
    match fs::read_to_string(STORE_PATH) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(Value::Object(obj)) => obj,
            _ => Map::new(),
        },
        Err(_) => Map::new(),
    }
}

fn write_store(store: &Map<String, Value>) -> Result<(), String> {
    let value = Value::Object(store.clone());
    let bytes = serde_json::to_vec_pretty(&value).map_err(|e| format!("serialize store: {e}"))?;
    fs::write(STORE_PATH, bytes).map_err(|e| format!("write store file: {e}"))
}

fn arg_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(ToOwned::to_owned)
}

fn write_json(value: &Value) {
    let mut stdout = io::stdout();
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let _ = stdout.write_all(&bytes);
    let _ = stdout.flush();
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        write_json(&json!({ "error": "failed to read request" }));
        return;
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => {
            write_json(&json!({ "error": "invalid request json" }));
            return;
        }
    };

    let result = match req.op.as_str() {
        "store_context" => {
            let key = arg_string(&req.args, "key").unwrap_or_else(|| "default".to_string());
            let value = req.args.get("value").cloned().unwrap_or(Value::Null);

            let mut store = read_store();
            store.insert(key.clone(), value);
            match write_store(&store) {
                Ok(()) => json!({ "ok": true, "key": key, "entries": store.len() }),
                Err(err) => json!({ "error": err }),
            }
        }
        "load_context" => {
            let key = arg_string(&req.args, "key").unwrap_or_else(|| "default".to_string());
            let store = read_store();
            match store.get(&key) {
                Some(value) => json!({ "key": key, "value": value }),
                None => json!({ "key": key, "value": Value::Null, "found": false }),
            }
        }
        "summarize_context" => {
            let store = read_store();
            let keys = store.keys().cloned().collect::<Vec<_>>();
            json!({ "entries": store.len(), "keys": keys })
        }
        other => json!({ "error": format!("unsupported memory op: {other}") }),
    };

    write_json(&result);
}
