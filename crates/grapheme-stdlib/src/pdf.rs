//! PDF capability module (Wasm-first via printpdf; host adapter scaffold).

use crate::capability::CapabilityResponse;
use serde_json::Value as JsonValue;

pub fn generate(args: &JsonValue) -> JsonValue {
    if arg_text(args, "title").is_empty() && arg_text(args, "body").is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: title or body");
    }
    CapabilityResponse::scaffold("pdf.generate", "printpdf document generation")
}

pub fn extract_text(args: &JsonValue) -> JsonValue {
    if arg_text(args, "path").is_empty() && arg_text(args, "bytes").is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: path or bytes");
    }
    CapabilityResponse::scaffold("pdf.extract_text", "PDF text extraction")
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}
