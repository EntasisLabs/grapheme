use grapheme_artifact::artifact::ArtifactError;
use grapheme_artifact::{build_artifact_from_mir, ArtifactEnvelope};

use crate::pipeline::{self, CompileOptions, CompilationArtifact};
use crate::error::CompilerError;
use crate::parser;

#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    pub compile_options: CompileOptions,
    pub entrypoint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledScript {
    pub compilation: CompilationArtifact,
    pub artifact: ArtifactEnvelope,
}

pub struct Compiler;

impl Compiler {
    pub fn compile_source(source: &str, options: CompilerOptions) -> Result<CompiledScript, CompilerError> {
        let ast = parser::parse(source)?;
        let compilation = pipeline::compile_program(ast, options.compile_options)?;
        let artifact = build_artifact_from_mir(&compilation.mir, options.entrypoint.as_deref())
            .map_err(map_artifact_error)?;

        Ok(CompiledScript {
            compilation,
            artifact,
        })
    }
}

fn map_artifact_error(err: ArtifactError) -> CompilerError {
    CompilerError::ArtifactEmitError(err.to_string())
}
