use grapheme_artifact::MirFunctionKind;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone)]
pub struct CapabilityCall {
    pub module: Option<String>,
    pub op: String,
    pub capability: String,
    pub arg_count: u16,
    pub args: JsonValue,
    pub step_index: usize,
}

#[derive(Debug, Clone)]
pub enum HostCallError {
    Retryable(String),
    Fatal(String),
}

pub trait CapabilityHost {
    fn call(&mut self, call: &CapabilityCall) -> Result<JsonValue, HostCallError>;

    /// A Grapheme `call` is entering `name`. Store tasks use this to make a
    /// called mutation one transaction (RFC-0007 F.3).
    fn enter_call(&mut self, _name: &str, _kind: &MirFunctionKind) {}

    /// That call returned. `failed` is true when the callee did not succeed.
    /// A store task commits a called mutation's applies here, once.
    fn leave_call(&mut self, _name: &str, _failed: bool) -> Result<(), HostCallError> {
        Ok(())
    }

    /// Policy or the capability allow-list refused this call before dispatch.
    /// Store tasks record a `failed` effect receipt (RFC-0007 F.4).
    fn refuse_call(&mut self, _module: Option<&str>, _op: &str, _args: &JsonValue, _message: &str) {
    }
}
