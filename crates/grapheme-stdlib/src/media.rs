//! Media capability module (ffmpeg/ffprobe CLI bridge).

use crate::capability::CapabilityResponse;
use serde_json::{json, Value as JsonValue};
use std::path::Path;
use std::process::Command;

pub fn probe(args: &JsonValue) -> JsonValue {
    let path = arg_text(args, "path");
    if path.is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: path");
    }

    match run_ffprobe(&path) {
        Ok(probe_json) => {
            let stream_count = probe_json
                .get("streams")
                .and_then(|streams| streams.as_array())
                .map(|streams| streams.len())
                .unwrap_or(0);
            let format_name = probe_json
                .get("format")
                .and_then(|format| format.get("format_name"))
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            let duration = probe_json
                .get("format")
                .and_then(|format| format.get("duration"))
                .and_then(|value| value.as_str())
                .map(str::to_owned);

            CapabilityResponse::ok(json!({
                "op": "media.probe",
                "path": path,
                "stream_count": stream_count,
                "format_name": format_name,
                "duration": duration,
                "probe": probe_json,
            }))
        }
        Err(err) => CapabilityResponse::invalid_args(err),
    }
}

pub fn transcode(args: &JsonValue) -> JsonValue {
    let input = arg_text(args, "input");
    let output = arg_text(args, "output");
    if input.is_empty() || output.is_empty() {
        return CapabilityResponse::invalid_args("missing required args: input and output");
    }

    match run_ffmpeg_transcode(&input, &output) {
        Ok(summary) => CapabilityResponse::ok(json!({
            "op": "media.transcode",
            "input": input,
            "output": output,
            "ok": true,
            "summary": summary,
        })),
        Err(err) => CapabilityResponse::invalid_args(err),
    }
}

fn run_ffprobe(path: &str) -> Result<JsonValue, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            path,
        ])
        .output()
        .map_err(|err| format!("ffprobe not available or failed to spawn: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe failed for '{path}': {}", stderr.trim()));
    }

    serde_json::from_slice(&output.stdout).map_err(|err| format!("parse ffprobe json: {err}"))
}

fn run_ffmpeg_transcode(input: &str, output: &str) -> Result<JsonValue, String> {
    let mut command = Command::new("ffmpeg");
    command.args([
        "-y",
        "-hide_banner",
        "-loglevel",
        "error",
        "-i",
        input,
    ]);

    match output
        .rsplit('.')
        .next()
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp4") => {
            command.args(["-c:v", "libx264", "-c:a", "aac", "-movflags", "+faststart"]);
        }
        Some("mp3") => {
            command.args(["-c:a", "libmp3lame"]);
        }
        Some("wav") => {
            command.args(["-c:a", "pcm_s16le"]);
        }
        _ => {
            command.args(["-c", "copy"]);
        }
    }

    command.arg(output);

    let result = command
        .output()
        .map_err(|err| format!("ffmpeg not available or failed to spawn: {err}"))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!(
            "ffmpeg transcode failed for '{input}' -> '{output}': {}",
            stderr.trim()
        ));
    }

    let output_path = Path::new(output);
    let bytes = fs_metadata_len(output_path).unwrap_or(0);

    Ok(json!({
        "codec_strategy": codec_strategy_label(output),
        "output_bytes": bytes,
    }))
}

fn codec_strategy_label(output: &str) -> &'static str {
    match output
        .rsplit('.')
        .next()
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp4") => "h264_aac",
        Some("mp3") => "mp3",
        Some("wav") => "pcm_s16le",
        _ => "copy",
    }
}

fn fs_metadata_len(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn probe_requires_path() {
        let out = probe(&json!({}));
        assert!(out.get("error").is_some_and(|value| !value.is_null()));
    }

    #[test]
    fn transcode_requires_input_and_output() {
        let out = transcode(&json!({ "input": "a.mp4" }));
        assert!(out.get("error").is_some_and(|value| !value.is_null()));
    }
}
