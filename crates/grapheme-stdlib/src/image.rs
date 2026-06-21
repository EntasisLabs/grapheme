//! Image capability module (Wasm-first via photon).

use crate::capability::CapabilityResponse;
use serde_json::Value as JsonValue;

pub fn resize(args: &JsonValue) -> JsonValue {
    if args.get("width").and_then(|v| v.as_u64()).is_none()
        || args.get("height").and_then(|v| v.as_u64()).is_none()
    {
        return CapabilityResponse::invalid_args("missing required args: width and height");
    }
    CapabilityResponse::scaffold(
        "image.resize",
        "bind Wasm via modules/image.module.json or run: grapheme modules activate image",
    )
}

pub fn convert(args: &JsonValue) -> JsonValue {
    if arg_text(args, "format").is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: format");
    }
    CapabilityResponse::scaffold(
        "image.convert",
        "bind Wasm via modules/image.module.json or run: grapheme modules activate image",
    )
}

pub fn metadata(args: &JsonValue) -> JsonValue {
    if arg_text(args, "path").is_empty() && args.get("bytes").is_none() {
        return CapabilityResponse::invalid_args("missing required arg: path or bytes");
    }
    CapabilityResponse::scaffold(
        "image.metadata",
        "bind Wasm via modules/image.module.json or run: grapheme modules activate image",
    )
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}
