use std::cell::RefCell;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ast::SourcePos;

/// 1-based source range. `line`/`column` is the start; `end_line`/`end_column`
/// is the end (exclusive of the end column when the span covers a token).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    /// 1-based start line.
    pub line: u32,
    /// 1-based start column.
    pub column: u32,
    /// 1-based end line.
    pub end_line: u32,
    /// 1-based end column.
    pub end_column: u32,
}

impl SourceSpan {
    /// A span that points at one position.
    pub fn point(line: u32, column: u32) -> Self {
        Self {
            line,
            column,
            end_line: line,
            end_column: column,
        }
    }

    /// Span covering a parser [`SourcePos`] (a single point).
    pub fn from_pos(pos: SourcePos) -> Self {
        Self::point(pos.line, pos.column)
    }
}

/// One parse or type failure with an optional source span.
///
/// `kind` is `"parse"` or `"type"`. `message` is the diagnostic text without
/// the `"parse error:"` / `"type error:"` display prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Always `"error"` for compile failures.
    pub severity: String,
    /// `"parse"` or `"type"`.
    pub kind: String,
    /// Diagnostic text.
    pub message: String,
    /// Where the failure points, when the compiler has a position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

thread_local! {
    static SPAN_STACK: RefCell<Vec<SourceSpan>> = const { RefCell::new(Vec::new()) };
}

/// Pushes a span for the duration of a compile step so [`CompilerError::type_error`]
/// and [`CompilerError::parse_error`] attach it.
pub struct SpanGuard;

impl SpanGuard {
    /// Remember `span` until this guard drops.
    pub fn push(span: SourceSpan) -> Self {
        SPAN_STACK.with(|stack| stack.borrow_mut().push(span));
        Self
    }

    /// [`SpanGuard::push`] for a parser point.
    pub fn for_pos(pos: SourcePos) -> Self {
        Self::push(SourceSpan::from_pos(pos))
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        SPAN_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

fn current_span() -> Option<SourceSpan> {
    SPAN_STACK.with(|stack| stack.borrow().last().copied())
}

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("parse error: {message}")]
    ParseError {
        message: String,
        span: Option<SourceSpan>,
    },

    #[error("unexpected rule: {0}")]
    UnexpectedRule(String),

    #[error("type error: {message}")]
    TypeError {
        message: String,
        span: Option<SourceSpan>,
    },

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

impl CompilerError {
    /// Parse failure. Uses the span on the current [`SpanGuard`], if any.
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
            span: current_span(),
        }
    }

    /// Parse failure at `span`, ignoring the span stack.
    pub fn parse_error_at(message: impl Into<String>, span: SourceSpan) -> Self {
        Self::ParseError {
            message: message.into(),
            span: Some(span),
        }
    }

    /// Type failure. Uses the span on the current [`SpanGuard`], if any.
    pub fn type_error(message: impl Into<String>) -> Self {
        Self::TypeError {
            message: message.into(),
            span: current_span(),
        }
    }

    /// Type failure at `span`, ignoring the span stack.
    pub fn type_error_at(message: impl Into<String>, span: SourceSpan) -> Self {
        Self::TypeError {
            message: message.into(),
            span: Some(span),
        }
    }

    /// Position of a parse or type error.
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::ParseError { span, .. } | Self::TypeError { span, .. } => *span,
            _ => None,
        }
    }

    /// Structured form of a parse or type error. Other variants are `None`.
    pub fn diagnostic(&self) -> Option<Diagnostic> {
        let (kind, message, span) = match self {
            Self::ParseError { message, span } => ("parse", message, span),
            Self::TypeError { message, span } => ("type", message, span),
            _ => return None,
        };
        Some(Diagnostic {
            severity: "error".to_string(),
            kind: kind.to_string(),
            message: message.clone(),
            span: *span,
        })
    }

    /// [`Self::diagnostic`] as a list. Empty when this error is not a parse or type failure.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostic().into_iter().collect()
    }
}

pub type GraphemeError = CompilerError;
