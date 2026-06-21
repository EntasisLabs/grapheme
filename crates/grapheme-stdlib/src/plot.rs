//! Plotting capability module (Wasm-first via plotters).

use crate::capability::CapabilityResponse;
use serde_json::Value as JsonValue;

pub fn line(args: &JsonValue) -> JsonValue {
    if !args.get("series").and_then(|v| v.as_array()).is_some_and(|s| !s.is_empty()) {
        return CapabilityResponse::invalid_args("missing required arg: series (non-empty array)");
    }
    CapabilityResponse::scaffold("plot.line", "plotters line chart")
}

pub fn bar(args: &JsonValue) -> JsonValue {
    if !args.get("series").and_then(|v| v.as_array()).is_some_and(|s| !s.is_empty()) {
        return CapabilityResponse::invalid_args("missing required arg: series (non-empty array)");
    }
    CapabilityResponse::scaffold("plot.bar", "plotters bar chart")
}

pub fn scatter(args: &JsonValue) -> JsonValue {
    if !args.get("points").and_then(|v| v.as_array()).is_some_and(|s| !s.is_empty()) {
        return CapabilityResponse::invalid_args("missing required arg: points (non-empty array)");
    }
    CapabilityResponse::scaffold("plot.scatter", "plotters scatter chart")
}
