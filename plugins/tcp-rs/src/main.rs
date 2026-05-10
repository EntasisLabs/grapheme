use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

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

fn resolve_target(args: &Value) -> Option<String> {
    arg_string(args, "target").or_else(|| arg_string(args, "session"))
}

fn connect_with_timeout(target: &str) -> Result<TcpStream, String> {
    let stream = TcpStream::connect(target)
        .map_err(|err| format!("connect failed for {target}: {err}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));
    Ok(stream)
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
            match connect_with_timeout(&target) {
                Ok(_) => json!({ "connected": true, "target": target, "session": target }),
                Err(err) => json!({ "connected": false, "target": target, "error": err }),
            }
        }
        "send" => {
            let session = arg_string(&req.args, "session").unwrap_or_default();
            let data = arg_string(&req.args, "data").unwrap_or_default();
            let Some(target) = resolve_target(&req.args) else {
                write_json(&json!({ "error": "missing target/session for tcp.send" }));
                return;
            };

            match connect_with_timeout(&target) {
                Ok(mut stream) => match stream.write_all(data.as_bytes()) {
                    Ok(_) => json!({ "sent": true, "session": session, "target": target, "bytes": data.len() }),
                    Err(err) => json!({ "sent": false, "target": target, "error": format!("send failed: {err}") }),
                },
                Err(err) => json!({ "sent": false, "target": target, "error": err }),
            }
        }
        "receive" => {
            let session = arg_string(&req.args, "session").unwrap_or_default();
            let Some(target) = resolve_target(&req.args) else {
                write_json(&json!({ "error": "missing target/session for tcp.receive" }));
                return;
            };

            let max_bytes = req
                .args
                .get("max_bytes")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(1024);

            match connect_with_timeout(&target) {
                Ok(mut stream) => {
                    let mut buf = vec![0u8; max_bytes];
                    match stream.read(&mut buf) {
                        Ok(n) => {
                            let data = String::from_utf8_lossy(&buf[..n]).to_string();
                            json!({ "session": session, "target": target, "data": data, "bytes": n })
                        }
                        Err(err) => json!({ "target": target, "error": format!("receive failed: {err}") }),
                    }
                }
                Err(err) => json!({ "target": target, "error": err }),
            }
        }
        other => json!({ "error": format!("unsupported tcp op: {other}") }),
    };

    write_json(&result);
}
