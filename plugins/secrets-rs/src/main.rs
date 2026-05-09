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
        "get_secret_handle" => {
            let name = arg_string(&req.args, "name").unwrap_or_else(|| "unknown".to_string());
            json!({
                "name": name,
                "handle": format!("sec_handle:{}", name),
                "ephemeral": true
            })
        }
        "sign_request" => {
            let secret = arg_string(&req.args, "secret").unwrap_or_else(|| "unknown".to_string());
            let payload = req.args.get("payload").cloned().unwrap_or(Value::Null);
            let payload_str = serde_json::to_string(&payload).unwrap_or_else(|_| "null".to_string());
            let signature = format!("sig:{}:{}", secret, payload_str.len());
            json!({
                "secret": secret,
                "signature": signature,
                "algorithm": "mock-hmac-sha256"
            })
        }
        other => json!({ "error": format!("unsupported secrets op: {other}") }),
    };

    write_json(&result);
}
