use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Read, Write};

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    args: Value,
}

fn print_json(value: &Value) {
    let serialized = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let mut stdout = io::stdout();
    let _ = stdout.write_all(serialized.as_bytes());
    let _ = stdout.flush();
}

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(ToOwned::to_owned)
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        print_json(&json!({ "error": "failed to read stdin" }));
        return;
    }

    let request: Request = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => {
            print_json(&json!({ "error": "invalid request json" }));
            return;
        }
    };

    let result = match request.op.as_str() {
        "read_text" => {
            match arg_str(&request.args, "path") {
                Some(path) => match fs::read_to_string(&path) {
                    Ok(text) => json!({ "text": text }),
                    Err(e) => json!({ "error": format!("read_text failed for '{}': {e}", path) }),
                },
                None => json!({ "error": "missing required arg: path" }),
            }
        }
        "write_text" => {
            let Some(path) = arg_str(&request.args, "path") else {
                print_json(&json!({ "error": "missing required arg: path" }));
                return;
            };
            let Some(text) = arg_str(&request.args, "text") else {
                print_json(&json!({ "error": "missing required arg: text" }));
                return;
            };

            match fs::write(&path, text.as_bytes()) {
                Ok(_) => json!({ "ok": true, "path": path }),
                Err(e) => json!({ "error": format!("write_text failed for '{}': {e}", path) }),
            }
        }
        "list_dir" => {
            let path = arg_str(&request.args, "path").unwrap_or_else(|| ".".to_string());
            match fs::read_dir(&path) {
                Ok(iter) => {
                    let mut entries = Vec::new();
                    for item in iter {
                        if let Ok(entry) = item {
                            entries.push(entry.file_name().to_string_lossy().to_string());
                        }
                    }
                    entries.sort();
                    json!({ "path": path, "entries": entries })
                }
                Err(e) => json!({ "error": format!("list_dir failed for '{}': {e}", path) }),
            }
        }
        other => json!({ "error": format!("unsupported io op: {other}") }),
    };

    print_json(&result);
}
