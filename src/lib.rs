/// ─────────────────────────────────────────────────────────────
///  Grapheme  —  Crate Root
/// ─────────────────────────────────────────────────────────────

pub mod ast;
pub mod artifact;
pub mod compiler;
pub mod compiler_api;
pub mod error;
pub mod host;
pub mod parser;
pub mod runtime;
pub mod state;

pub use artifact::{ArtifactEnvelope, ExecutionOutcome, ExecutionResult, TraceSummary};
pub use compiler_api::{CompiledScript, Compiler, CompilerOptions};
pub use parser::parse;
pub use runtime::{RuntimeEngine, RuntimeOptions};
pub use state::AgentState;
pub use error::{AgentQLError, GraphemeError};

pub fn compile(source: &str) -> Result<compiler::CompilationArtifact, GraphemeError> {
	let ast = parse(source)?;
	compiler::compile_program(ast, compiler::CompileOptions::default())
}

pub fn compile_to_artifact(
	source: &str,
	entrypoint: Option<&str>,
) -> Result<ArtifactEnvelope, GraphemeError> {
	let compilation = compile(source)?;
	artifact::build_artifact_from_compilation(&compilation, entrypoint)
}
