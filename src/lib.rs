/// ─────────────────────────────────────────────────────────────
///  AgentQL  —  Crate Root
/// ─────────────────────────────────────────────────────────────

pub mod ast;
pub mod error;
pub mod parser;
pub mod state;

pub use parser::parse;
pub use state::AgentState;
pub use error::AgentQLError;
