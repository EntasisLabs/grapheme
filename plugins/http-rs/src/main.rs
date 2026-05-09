use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Read, Write};

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    args: Value,
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
        "get" => {
            let url = arg_string(&req.args, "url").unwrap_or_else(|| "about:blank".to_string());
            json!({
                "status": 200,
                "url": url,
                "method": "GET",
                "body": format!("mock GET response for {url}")
            })
        }
        "post" => {
            let url = arg_string(&req.args, "url").unwrap_or_else(|| "about:blank".to_string());
            let body = req.args.get("body").cloned().unwrap_or(Value::Null);
            json!({
                "status": 200,
                "url": url,
                "method": "POST",
                "accepted": true,
                "echo": body
            })
        }
        other => json!({ "error": format!("unsupported http op: {other}") }),
    };

    write_json(&result);
}
