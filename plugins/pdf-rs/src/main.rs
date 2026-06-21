use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Read, Write};

const ENVELOPE_SCHEMA: &str = "grapheme.host.result.envelope/v1";

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

fn envelope(data: Value) -> Value {
    json!({
        "data": data,
        "meta": {
            "schema": ENVELOPE_SCHEMA,
            "engine": "pdf-rs-wasm",
            "version": "0.1.0",
        },
        "error": null,
    })
}

fn failure(message: impl Into<String>) -> Value {
    json!({
        "data": null,
        "meta": { "schema": ENVELOPE_SCHEMA, "engine": "pdf-rs-wasm" },
        "error": message.into(),
    })
}

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(ToOwned::to_owned)
}

fn escape_pdf_text(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn build_minimal_pdf(title: &str, body: &str) -> Vec<u8> {
    let content = format!("{}\\n\\n{}", escape_pdf_text(title), escape_pdf_text(body));
    let stream = format!(
        "BT /F1 14 Tf 72 720 Td ({title}) Tj 0 -24 Td /F1 11 Tf ({body}) Tj ET",
        title = escape_pdf_text(title),
        body = escape_pdf_text(body),
    );

    let mut pdf = String::new();
    pdf.push_str("%PDF-1.4\n");
    let offsets = [
        format!("1 0 obj<< /Type /Catalog /Pages 2 0 R >>endobj\n"),
        format!("2 0 obj<< /Type /Pages /Kids [3 0 R] /Count 1 >>endobj\n"),
        format!(
            "3 0 obj<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>endobj\n"
        ),
        format!(
            "4 0 obj<< /Length {} >>stream\n{}\nendstream\nendobj\n",
            stream.len(),
            stream
        ),
        format!("5 0 obj<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>endobj\n"),
    ];

    let mut xref_positions = Vec::new();
    for part in &offsets {
        xref_positions.push(pdf.len());
        pdf.push_str(part);
    }

    let xref_start = pdf.len();
    pdf.push_str("xref\n");
    pdf.push_str("0 6\n");
    pdf.push_str("0000000000 65535 f \n");
    for pos in xref_positions {
        pdf.push_str(&format!("{:010} 00000 n \n", pos));
    }
    pdf.push_str("trailer<< /Size 6 /Root 1 0 R >>\n");
    pdf.push_str(&format!("startxref\n{xref_start}\n%%EOF\n"));

    let _ = content;
    pdf.into_bytes()
}

fn extract_text_from_pdf_bytes(bytes: &[u8]) -> String {
    let raw = String::from_utf8_lossy(bytes);
    let mut chunks = Vec::new();
    for segment in raw.split("stream") {
        if let Some(end) = segment.find("endstream") {
            let body = segment[..end].trim();
            if body.starts_with('\n') || body.starts_with('\r') {
                let cleaned = body
                    .chars()
                    .filter(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
                    .collect::<String>();
                if !cleaned.is_empty() {
                    chunks.push(cleaned);
                }
            }
        }
    }
    chunks.join("\n")
}

fn op_generate(args: &Value) -> Value {
    let title = arg_str(args, "title").unwrap_or_else(|| "Untitled".to_string());
    let body = arg_str(args, "body").unwrap_or_default();
    if title.trim().is_empty() && body.trim().is_empty() {
        return failure("missing required arg: title or body");
    }

    let pdf_bytes = build_minimal_pdf(&title, &body);
    envelope(json!({
        "ok": true,
        "op": "pdf.generate",
        "title": title,
        "body": body,
        "page_count": 1,
        "format": "pdf",
        "bytes_base64": STANDARD.encode(pdf_bytes),
    }))
}

fn op_extract_text(args: &Value) -> Value {
    if let Some(path) = arg_str(args, "path") {
        match std::fs::read(&path) {
            Ok(bytes) => {
                return envelope(json!({
                    "ok": true,
                    "op": "pdf.extract_text",
                    "path": path,
                    "text": extract_text_from_pdf_bytes(&bytes),
                }));
            }
            Err(err) => return failure(format!("read pdf '{}': {err}", path)),
        }
    }

    if let Some(encoded) = arg_str(args, "bytes") {
        match STANDARD.decode(encoded.as_bytes()) {
            Ok(bytes) => {
                return envelope(json!({
                    "ok": true,
                    "op": "pdf.extract_text",
                    "text": extract_text_from_pdf_bytes(&bytes),
                }));
            }
            Err(err) => return failure(format!("decode bytes_base64: {err}")),
        }
    }

    failure("missing required arg: path or bytes")
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        print_json(&failure("failed to read stdin"));
        return;
    }

    let request: Request = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => {
            print_json(&failure("invalid request json"));
            return;
        }
    };

    let result = match request.op.as_str() {
        "generate" => op_generate(&request.args),
        "extract_text" => op_extract_text(&request.args),
        other => failure(format!("unsupported pdf op: {other}")),
    };

    print_json(&result);
}
