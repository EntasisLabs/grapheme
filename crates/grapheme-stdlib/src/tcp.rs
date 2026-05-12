use serde_json::{json, Value as JsonValue};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub fn connect(target: &str) -> JsonValue {
    if target.is_empty() {
        return json!({ "connected": false, "error": "missing target" });
    }

    match TcpStream::connect(target) {
        Ok(_) => json!({ "connected": true, "target": target, "session": target }),
        Err(err) => json!({ "connected": false, "target": target, "error": err.to_string() }),
    }
}

pub fn send(target: &str, data: &str) -> JsonValue {
    if target.is_empty() {
        return json!({ "sent": false, "error": "missing target/session" });
    }

    match TcpStream::connect(target) {
        Ok(mut stream) => {
            let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));
            match stream.write_all(data.as_bytes()) {
                Ok(_) => json!({ "sent": true, "target": target, "bytes": data.len() }),
                Err(err) => json!({ "sent": false, "target": target, "error": err.to_string() }),
            }
        }
        Err(err) => json!({ "sent": false, "target": target, "error": err.to_string() }),
    }
}

pub fn receive(target: &str, max_bytes: usize) -> JsonValue {
    if target.is_empty() {
        return json!({ "error": "missing target/session" });
    }

    match TcpStream::connect(target) {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
            let mut buf = vec![0u8; max_bytes];
            match stream.read(&mut buf) {
                Ok(n) => json!({
                    "target": target,
                    "bytes": n,
                    "data": String::from_utf8_lossy(&buf[..n]).to_string(),
                }),
                Err(err) => json!({ "target": target, "error": err.to_string() }),
            }
        }
        Err(err) => json!({ "target": target, "error": err.to_string() }),
    }
}
