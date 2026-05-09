use crate::artifact::{self, ArtifactEnvelope};
use crate::compiler::{self, CompileOptions, CompilationArtifact};
use crate::error::GraphemeError;
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
    pub fn compile_source(source: &str, options: CompilerOptions) -> Result<CompiledScript, GraphemeError> {
        let ast = parser::parse(source)?;
        let compilation = compiler::compile_program(ast, options.compile_options)?;
        let artifact = artifact::build_artifact_from_compilation(&compilation, options.entrypoint.as_deref())?;

        Ok(CompiledScript {
            compilation,
            artifact,
        })
    }
}
