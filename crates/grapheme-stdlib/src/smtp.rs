use serde_json::{json, Value as JsonValue};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

pub fn send_mail(args: &JsonValue) -> JsonValue {
    let server = args
        .get("server")
        .and_then(|v| v.as_str())
        .unwrap_or("127.0.0.1:25");
    let from = args
        .get("from")
        .and_then(|v| v.as_str())
        .unwrap_or("grapheme@localhost");
    let to = args.get("to").and_then(|v| v.as_str()).unwrap_or("");
    let subject = args
        .get("subject")
        .and_then(|v| v.as_str())
        .unwrap_or("(no subject)");
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("(empty body)");

    if to.is_empty() {
        return json!({ "accepted": false, "error": "missing smtp recipient: to" });
    }

    let mut stream = match TcpStream::connect(server) {
        Ok(s) => s,
        Err(err) => return json!({ "accepted": false, "server": server, "error": err.to_string() }),
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));

    let mut reader = match stream.try_clone() {
        Ok(s) => BufReader::new(s),
        Err(err) => return json!({ "accepted": false, "server": server, "error": err.to_string() }),
    };

    let mut run = || -> Result<(), String> {
        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 220 {
            return Err(format!("expected 220 banner, got {code}: {msg}"));
        }

        send_smtp_line(&mut stream, "HELO grapheme.local")?;
        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 250 {
            return Err(format!("HELO rejected: {code}: {msg}"));
        }

        send_smtp_line(&mut stream, &format!("MAIL FROM:<{from}>"))?;
        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 250 {
            return Err(format!("MAIL FROM rejected: {code}: {msg}"));
        }

        send_smtp_line(&mut stream, &format!("RCPT TO:<{to}>"))?;
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
        send_smtp_line(&mut stream, body)?;
        send_smtp_line(&mut stream, ".")?;

        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 250 {
            return Err(format!("message rejected: {code}: {msg}"));
        }

        let _ = send_smtp_line(&mut stream, "QUIT");
        Ok(())
    };

    match run() {
        Ok(()) => json!({
            "accepted": true,
            "server": server,
            "from": from,
            "to": to,
            "subject": subject,
        }),
        Err(err) => json!({ "accepted": false, "server": server, "error": err }),
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

fn read_smtp_response(reader: &mut BufReader<TcpStream>) -> Result<(u16, String), String> {
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

        lines.push(trimmed);

        if !continuation {
            return Ok((code, lines.join("\n")));
        }
    }
}
