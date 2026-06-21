//! Media capability module (ffmpeg-native bridge).

use crate::capability::CapabilityResponse;
use serde_json::Value as JsonValue;

pub fn probe(args: &JsonValue) -> JsonValue {
    if arg_text(args, "path").is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: path");
    }
    CapabilityResponse::scaffold("media.probe", "ffmpeg stream/container probe")
}

pub fn transcode(args: &JsonValue) -> JsonValue {
    if arg_text(args, "input").is_empty() || arg_text(args, "output").is_empty() {
        return CapabilityResponse::invalid_args("missing required args: input and output");
    }
    CapabilityResponse::scaffold("media.transcode", "ffmpeg transcode")
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}
