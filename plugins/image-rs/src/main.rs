use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Cursor, Read, Write};

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
            "engine": "image-rs-wasm",
            "version": "0.1.0",
        },
        "error": null,
    })
}

fn failure(message: impl Into<String>) -> Value {
    json!({
        "data": null,
        "meta": { "schema": ENVELOPE_SCHEMA, "engine": "image-rs-wasm" },
        "error": message.into(),
    })
}

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(ToOwned::to_owned)
}

fn arg_u64(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| v.as_u64())
}

fn load_image(args: &Value) -> Result<DynamicImage, String> {
    if let Some(path) = arg_str(args, "path") {
        return ImageReader::open(&path)
            .map_err(|err| format!("open image '{path}': {err}"))?
            .decode()
            .map_err(|err| format!("decode image '{path}': {err}"));
    }

    let encoded = arg_str(args, "bytes").or_else(|| arg_str(args, "bytes_base64"));
    if let Some(encoded) = encoded {
        let bytes = STANDARD
            .decode(encoded.as_bytes())
            .map_err(|err| format!("decode bytes_base64: {err}"))?;
        return ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|err| format!("guess image format: {err}"))?
            .decode()
            .map_err(|err| format!("decode image bytes: {err}"));
    }

    Err("missing required arg: path or bytes".to_string())
}

fn parse_output_format(name: &str) -> Result<ImageFormat, String> {
    match name.to_ascii_lowercase().as_str() {
        "png" => Ok(ImageFormat::Png),
        "jpeg" | "jpg" => Ok(ImageFormat::Jpeg),
        "gif" => Ok(ImageFormat::Gif),
        "webp" => Ok(ImageFormat::WebP),
        other => Err(format!("unsupported image format '{other}'; expected png|jpeg|gif|webp")),
    }
}

fn encode_image(img: &DynamicImage, format: ImageFormat) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), format)
        .map_err(|err| format!("encode image: {err}"))?;
    Ok(buf)
}

fn format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpeg",
        ImageFormat::Gif => "gif",
        ImageFormat::WebP => "webp",
        _ => "unknown",
    }
}

fn op_metadata(args: &Value) -> Value {
    match load_image(args) {
        Ok(img) => envelope(json!({
            "ok": true,
            "op": "image.metadata",
            "width": img.width(),
            "height": img.height(),
            "color_type": format!("{:?}", img.color()),
        })),
        Err(err) => failure(err),
    }
}

fn op_resize(args: &Value) -> Value {
    let Some(width) = arg_u64(args, "width") else {
        return failure("missing required arg: width");
    };
    let Some(height) = arg_u64(args, "height") else {
        return failure("missing required arg: height");
    };
    if width == 0 || height == 0 {
        return failure("width and height must be >= 1");
    }

    match load_image(args) {
        Ok(img) => {
            let resized = img.resize(width as u32, height as u32, FilterType::Lanczos3);
            match encode_image(&resized, ImageFormat::Png) {
                Ok(bytes) => envelope(json!({
                    "ok": true,
                    "op": "image.resize",
                    "width": width,
                    "height": height,
                    "format": "png",
                    "bytes_base64": STANDARD.encode(bytes),
                })),
                Err(err) => failure(err),
            }
        }
        Err(err) => failure(err),
    }
}

fn op_convert(args: &Value) -> Value {
    let Some(format_name_arg) = arg_str(args, "format") else {
        return failure("missing required arg: format");
    };

    let format = match parse_output_format(&format_name_arg) {
        Ok(format) => format,
        Err(err) => return failure(err),
    };

    match load_image(args) {
        Ok(img) => match encode_image(&img, format) {
            Ok(bytes) => envelope(json!({
                "ok": true,
                "op": "image.convert",
                "format": format_name(format),
                "width": img.width(),
                "height": img.height(),
                "bytes_base64": STANDARD.encode(bytes),
            })),
            Err(err) => failure(err),
        },
        Err(err) => failure(err),
    }
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
        "metadata" => op_metadata(&request.args),
        "resize" => op_resize(&request.args),
        "convert" => op_convert(&request.args),
        other => failure(format!("unsupported image op: {other}")),
    };

    print_json(&result);
}
