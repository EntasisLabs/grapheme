use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::time::Duration;

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
    let mut stdout = io::stdout();
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let _ = stdout.write_all(&bytes);
    let _ = stdout.flush();
}

fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "only http:// URLs are supported by this plugin".to_string())?;

    let (host_port, path_part) = match rest.split_once('/') {
        Some((hp, path)) => (hp, format!("/{path}")),
        None => (rest, "/".to_string()),
    };

    if host_port.is_empty() {
        return Err("invalid URL: missing host".to_string());
    }

    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => {
            let parsed = p
                .parse::<u16>()
                .map_err(|_| format!("invalid URL port: {p}"))?;
            (h.to_string(), parsed)
        }
        None => (host_port.to_string(), 80),
    };

    Ok((host, port, path_part))
}
fn execute_http(method: &str, url: &str, body: Option<&Value>) -> Value {
    // ... (keep your existing URL check)

    // 1. Configure the agent for single-threaded WASM
    // We disable the connection pool or set it to 0 to prevent background thread spawning
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .max_idle_connections(0) // Forces ureq to not maintain a background thread pool
        .build()
        .new_agent();

    // 2. Build the request as before
    let mut builder = ureq::http::Request::builder()
        .method(method)
        .uri(url);

    if method == "POST" {
        builder = builder.header("Content-Type", "application/json");
    }

    let payload = body
        .map(|b| serde_json::to_vec(b).unwrap_or_else(|_| b"null".to_vec()))
        .unwrap_or_default();

    // 3. Execute the request
    let response = match agent.run(builder.body(payload).unwrap()) {
        Ok(resp) => resp,
        Err(err) => {
            return json!({
                "error": format!("request failed: {err}"),
                "url": url,
                "method": method
            })
        }
    };

    // 4. Extract data (Status and Body)
    let status = response.status().as_u16();
    let status_text = response.status().canonical_reason().unwrap_or("UNKNOWN");
    
    // Efficiently read body without needing extra threads
    let mut body_bytes = Vec::new();
    let mut reader = response.into_body().into_reader();
    let _ = std::io::Read::read_to_end(&mut reader, &mut body_bytes);
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    json!({
        "status": status,
        "status_line": format!("HTTP {} {}", status, status_text),
        "url": url,
        "method": method,
        "body": body_str,
    })
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
            execute_http("GET", &url, None)
        }
        "post" => {
            let url = arg_string(&req.args, "url").unwrap_or_else(|| "about:blank".to_string());
            let body = req.args.get("body").cloned().unwrap_or(Value::Null);
            execute_http("POST", &url, Some(&body))
        }
        other => json!({ "error": format!("unsupported http op: {other}") }),
    };

    write_json(&result);
}
