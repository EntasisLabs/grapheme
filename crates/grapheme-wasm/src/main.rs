//! WASI `_start` entry for runtime-in-Wasm execution (RFC-0006).

use grapheme_wasm::{execute_to_json, parse_stdin_request};
use serde_json::json;
use std::io::{self, Read, Write};

fn write_json(value: &serde_json::Value) {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let mut stdout = io::stdout();
    let _ = stdout.write_all(&bytes);
    let _ = stdout.flush();
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        write_json(&json!({
            "ok": false,
            "error": { "code": "STDIN_READ_FAILED", "message": "failed to read request" }
        }));
        return;
    }

    let request = match parse_stdin_request(&input) {
        Ok(req) => req,
        Err(message) => {
            write_json(&json!({
                "ok": false,
                "error": { "code": "INVALID_REQUEST", "message": message }
            }));
            return;
        }
    };

    write_json(&execute_to_json(&request));
}
