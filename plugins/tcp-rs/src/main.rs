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
        "connect" => {
            let target = arg_string(&req.args, "target").unwrap_or_else(|| "localhost:0".to_string());
            json!({ "connected": true, "target": target, "session": "tcp-session-1" })
        }
        "send" => {
            let session = arg_string(&req.args, "session").unwrap_or_else(|| "tcp-session-1".to_string());
            let data = arg_string(&req.args, "data").unwrap_or_default();
            json!({ "sent": true, "session": session, "bytes": data.len() })
        }
        "receive" => {
            let session = arg_string(&req.args, "session").unwrap_or_else(|| "tcp-session-1".to_string());
            json!({ "session": session, "data": "mock-tcp-payload", "bytes": 16 })
        }
        other => json!({ "error": format!("unsupported tcp op: {other}") }),
    };

    write_json(&result);
}
