//! Dataframe / analytics capability module (Polars-native ingest, JSON frame pipeline).

mod frame;

use crate::capability::CapabilityResponse;
use frame::{filter_frame, frame_from_arg, group_frame, is_frame, rows_from_frame, schema_from_frame};
use serde_json::{json, Value as JsonValue};

const DEFAULT_MAX_ROWS: usize = 10_000;

pub fn read_csv(args: &JsonValue) -> JsonValue {
    let path = arg_text(args, "path");
    if path.is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: path");
    }

    let max_rows = arg_usize(args, "max_rows").unwrap_or(DEFAULT_MAX_ROWS).max(1);

    #[cfg(feature = "data")]
    {
        match frame::read_csv_path(&path, max_rows) {
            Ok(frame) => CapabilityResponse::ok(json!({
                "op": "data.read_csv",
                "frame": frame,
            })),
            Err(err) => CapabilityResponse::invalid_args(err),
        }
    }

    #[cfg(not(feature = "data"))]
    {
        let _ = (path, max_rows);
        CapabilityResponse::scaffold("data.read_csv", "enable grapheme-stdlib/data for Polars CSV ingest")
    }
}

pub fn filter(args: &JsonValue) -> JsonValue {
    let Some(frame) = frame_from_arg(args) else {
        return CapabilityResponse::invalid_args("missing required arg: frame");
    };
    if !is_frame(frame) {
        return CapabilityResponse::invalid_args("frame must be a grapheme.data.frame/v1 handle");
    }

    let column = arg_text(args, "column");
    let Some(eq) = args.get("eq") else {
        return CapabilityResponse::invalid_args("missing required arg: eq");
    };
    if column.is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: column");
    }

    match filter_frame(frame, &column, eq) {
        Ok(filtered) => CapabilityResponse::ok(json!({
            "op": "data.filter",
            "frame": filtered,
        })),
        Err(err) => CapabilityResponse::invalid_args(err),
    }
}

pub fn group_by(args: &JsonValue) -> JsonValue {
    let Some(frame) = frame_from_arg(args) else {
        return CapabilityResponse::invalid_args("missing required arg: frame");
    };
    if !is_frame(frame) {
        return CapabilityResponse::invalid_args("frame must be a grapheme.data.frame/v1 handle");
    }

    let by = arg_text(args, "by");
    if by.is_empty() {
        return CapabilityResponse::invalid_args("missing required arg: by");
    }

    match group_frame(frame, &by) {
        Ok(grouped) => CapabilityResponse::ok(json!({
            "op": "data.group_by",
            "group": grouped,
        })),
        Err(err) => CapabilityResponse::invalid_args(err),
    }
}

pub fn aggregate(args: &JsonValue) -> JsonValue {
    let Some(frame) = frame_from_arg(args) else {
        return CapabilityResponse::invalid_args("missing required arg: frame");
    };
    if !is_frame(frame) {
        return CapabilityResponse::invalid_args("frame must be a grapheme.data.frame/v1 handle");
    }

    let op = arg_text(args, "op");
    if op != "count" {
        return CapabilityResponse::scaffold("data.aggregate", "only aggregate op=count is implemented in 0.6.0");
    }

    let row_count = rows_from_frame(frame).map(|rows| rows.len()).unwrap_or(0);
    CapabilityResponse::ok(json!({
        "op": "data.aggregate",
        "result": {
            "op": "count",
            "value": row_count,
        },
    }))
}

pub fn to_json(args: &JsonValue) -> JsonValue {
    let Some(frame) = frame_from_arg(args) else {
        return CapabilityResponse::invalid_args("missing required arg: frame");
    };
    if !is_frame(frame) {
        return CapabilityResponse::invalid_args("frame must be a grapheme.data.frame/v1 handle");
    }

    CapabilityResponse::ok(json!({
        "op": "data.to_json",
        "rows": rows_from_frame(frame).cloned().unwrap_or_default(),
    }))
}

pub fn schema(args: &JsonValue) -> JsonValue {
    let Some(frame) = frame_from_arg(args) else {
        return CapabilityResponse::invalid_args("missing required arg: frame");
    };
    if !is_frame(frame) {
        return CapabilityResponse::invalid_args("frame must be a grapheme.data.frame/v1 handle");
    }

    CapabilityResponse::ok(json!({
        "op": "data.schema",
        "schema": schema_from_frame(frame),
    }))
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

fn arg_usize(args: &JsonValue, key: &str) -> Option<usize> {
    args.get(key)
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .map(|v| v as usize)
}
