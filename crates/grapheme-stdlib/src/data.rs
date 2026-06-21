//! Dataframe / analytics capability module (Polars-native).
//!
//! Enabled with the `data` Cargo feature. Implementation lands during 0.6.0.

use crate::capability::CapabilityResponse;
use serde_json::Value as JsonValue;

pub fn read_csv(args: &JsonValue) -> JsonValue {
    let path = arg_text(args, "path");
    if path.is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: path");
    }
    CapabilityResponse::scaffold("data.read_csv", "Polars CSV ingest")
}

pub fn filter(args: &JsonValue) -> JsonValue {
    if args.get("frame").is_none() {
        return CapabilityResponse::invalid_args("missing required arg: frame");
    }
    CapabilityResponse::scaffold("data.filter", "Polars predicate filter")
}

pub fn group_by(args: &JsonValue) -> JsonValue {
    if arg_text(args, "by").is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: by");
    }
    CapabilityResponse::scaffold("data.group_by", "Polars group_by")
}

pub fn aggregate(_args: &JsonValue) -> JsonValue {
    CapabilityResponse::scaffold("data.aggregate", "Polars aggregate expressions")
}

pub fn to_json(args: &JsonValue) -> JsonValue {
    if args.get("frame").is_none() {
        return CapabilityResponse::invalid_args("missing required arg: frame");
    }
    CapabilityResponse::scaffold("data.to_json", "Polars frame JSON export")
}

pub fn schema(args: &JsonValue) -> JsonValue {
    if args.get("frame").is_none() {
        return CapabilityResponse::invalid_args("missing required arg: frame");
    }
    CapabilityResponse::scaffold("data.schema", "Polars schema introspection")
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}
