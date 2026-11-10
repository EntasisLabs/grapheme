use serde_json::Value as JsonValue;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("runtime error: {0}")]
    RuntimeError(String),

    #[error("artifact compatibility error: {0}")]
    ArtifactCompatibilityError(String),

    #[error("artifact integrity error: {0}")]
    ArtifactIntegrityError(String),

    /// A store task finished without success. Earlier `apply` commits stay.
    /// `effects` is a JSON array of [`crate::EffectReceipt`] values from the
    /// run, including calls that failed or were refused.
    #[error("{message}")]
    TaskFailed { message: String, effects: JsonValue },
}
