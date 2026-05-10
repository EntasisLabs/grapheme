use serde::Deserialize;
use serde_json::{json, Value};
use std::io::BufRead;
use std::io::BufReader;
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

fn read_smtp_response(reader: &mut BufReader<TcpStream>) -> Result<(u16, String), String> {
    let mut last_code = 0u16;
    let mut lines = Vec::new();

    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|err| format!("read smtp response failed: {err}"))?;
        if read == 0 {
            return Err("smtp server closed connection".to_string());
        }

        let trimmed = line.trim_end().to_string();
        let code = trimmed
            .get(0..3)
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or_else(|| format!("invalid smtp response line: {trimmed}"))?;
        let continuation = trimmed.as_bytes().get(3).copied() == Some(b'-');

        last_code = code;
        lines.push(trimmed);

        if !continuation {
            return Ok((last_code, lines.join("\n")));
        }
    }
}

fn send_smtp_line(stream: &mut TcpStream, line: &str) -> Result<(), String> {
    stream
        .write_all(line.as_bytes())
        .map_err(|err| format!("smtp write failed: {err}"))?;
    stream
        .write_all(b"\r\n")
        .map_err(|err| format!("smtp write failed: {err}"))
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
        "send_mail" => {
            let to = arg_string(&req.args, "to").unwrap_or_else(|| "unknown@example.com".to_string());
            let from = arg_string(&req.args, "from").unwrap_or_else(|| "grapheme@localhost".to_string());
            let subject = arg_string(&req.args, "subject").unwrap_or_else(|| "(no subject)".to_string());
            let body = arg_string(&req.args, "body").unwrap_or_else(|| "(empty body)".to_string());
            let server = arg_string(&req.args, "server").unwrap_or_else(|| "127.0.0.1:25".to_string());

            let mut stream = match TcpStream::connect(&server) {
                Ok(s) => s,
                Err(err) => {
                    write_json(&json!({ "accepted": false, "error": format!("connect failed for {server}: {err}") }));
                    return;
                }
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));

            let mut reader = match stream.try_clone() {
                Ok(clone) => BufReader::new(clone),
                Err(err) => {
                    write_json(&json!({ "accepted": false, "error": format!("smtp clone failed: {err}") }));
                    return;
                }
            };

            let run = || -> Result<Value, String> {
                let (code, banner) = read_smtp_response(&mut reader)?;
                if code != 220 {
                    return Err(format!("expected 220 banner, got {code}: {banner}"));
                }

                send_smtp_line(&mut stream, "HELO grapheme.local")?;
                let (code, msg) = read_smtp_response(&mut reader)?;
                if code != 250 {
                    return Err(format!("HELO rejected: {code}: {msg}"));
                }

                send_smtp_line(&mut stream, &format!("MAIL FROM:<{from}>") )?;
                let (code, msg) = read_smtp_response(&mut reader)?;
                if code != 250 {
                    return Err(format!("MAIL FROM rejected: {code}: {msg}"));
                }

                send_smtp_line(&mut stream, &format!("RCPT TO:<{to}>") )?;
                let (code, msg) = read_smtp_response(&mut reader)?;
                if code != 250 && code != 251 {
                    return Err(format!("RCPT TO rejected: {code}: {msg}"));
                }

                send_smtp_line(&mut stream, "DATA")?;
                let (code, msg) = read_smtp_response(&mut reader)?;
                if code != 354 {
                    return Err(format!("DATA rejected: {code}: {msg}"));
                }

                send_smtp_line(&mut stream, &format!("Subject: {subject}"))?;
                send_smtp_line(&mut stream, "")?;
                send_smtp_line(&mut stream, &body)?;
                send_smtp_line(&mut stream, ".")?;

                let (code, msg) = read_smtp_response(&mut reader)?;
                if code != 250 {
                    return Err(format!("message rejected: {code}: {msg}"));
                }

                let _ = send_smtp_line(&mut stream, "QUIT");

                Ok(json!({
                    "accepted": true,
                    "server": server,
                    "from": from,
                    "to": to,
                    "subject": subject,
                }))
            };

            match run() {
                Ok(v) => v,
                Err(err) => json!({ "accepted": false, "server": server, "error": err }),
            }
        }
        other => json!({ "error": format!("unsupported smtp op: {other}") }),
    };

    write_json(&result);
}
