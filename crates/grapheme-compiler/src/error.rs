use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilerError {
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

    #[error("artifact emit error: {0}")]
    ArtifactEmitError(String),
}

pub type GraphemeError = CompilerError;
