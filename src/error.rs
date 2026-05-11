/// ─────────────────────────────────────────────────────────────
///  Grapheme  —  Error Types
/// ─────────────────────────────────────────────────────────────

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphemeError {
    #[error("parse error: {0}")]
    ParseError(String),

    #[error("unexpected rule: {0}")]
    UnexpectedRule(String),

    #[error("type error: {0}")]
    TypeError(String),

    #[error("module not found: {0}")]
    ModuleNotFound(String),

    #[error("module proposal rejected: {0}")]
    ProposalRejected(String),

    #[error("runtime error: {0}")]
    RuntimeError(String),

    #[error("pipeline error at step {step}: {message}")]
    PipelineError { step: usize, message: String },

    #[error("verification error: {0}")]
    VerificationError(String),

    #[error("capability denied: {0}")]
    CapabilityDenied(String),

    #[error("artifact compatibility error: {0}")]
    ArtifactCompatibilityError(String),

    #[error("artifact integrity error: {0}")]
    ArtifactIntegrityError(String),
}
