pub mod ast;
pub mod compiler_api;
pub mod error;
pub mod hir;
pub mod mir_lower;
pub mod parser;
pub mod pipeline;
pub mod verifier;

use grapheme_artifact::{build_artifact_from_mir, ArtifactEnvelope};

pub use compiler_api::{CompiledScript, Compiler, CompilerOptions};
pub use error::CompilerError;
pub use parser::parse;
pub use pipeline::{compile_program, CompilationArtifact, CompileOptions};

pub fn compile(source: &str) -> Result<CompilationArtifact, CompilerError> {
	let ast = parse(source)?;
	compile_program(ast, CompileOptions::default())
}

pub fn compile_to_artifact(source: &str, entrypoint: Option<&str>) -> Result<ArtifactEnvelope, CompilerError> {
	let compilation = compile(source)?;
	build_artifact_from_mir(&compilation.mir, entrypoint)
		.map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))
}
