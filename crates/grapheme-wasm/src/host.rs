//! Wasm-safe stdlib capability host for runtime-in-Wasm execution.

use grapheme_runtime::{CapabilityCall, CapabilityHost, HostCallError};
use grapheme_stdlib::registry;
use serde_json::Value as JsonValue;

/// Dispatches wasm-profile stdlib ops locally (`core` / `json` / `csv` / `yaml` / `html`).
/// Host-only modules fail with a fatal error so the outer Wasm host can decide how
/// to fulfill them in a later slice.
#[derive(Debug, Default)]
pub struct WasmStdlibHost;

impl WasmStdlibHost {
    fn resolve_module(call: &CapabilityCall) -> String {
        call.module
            .as_deref()
            .map(|m| m.to_lowercase())
            .or_else(|| call.capability.split('.').next().map(|m| m.to_lowercase()))
            .unwrap_or_default()
    }
}

impl CapabilityHost for WasmStdlibHost {
    fn call(&mut self, call: &CapabilityCall) -> Result<JsonValue, HostCallError> {
        let module = Self::resolve_module(call);
        if let Some(out) = registry::dispatch(&module, &call.op, &call.args) {
            return Ok(out);
        }

        Err(HostCallError::Fatal(format!(
            "capability '{module}.{}' is outside the Wasm stdlib profile; host must provide grapheme.runtime.host.v1::call.capability",
            call.op
        )))
    }
}
