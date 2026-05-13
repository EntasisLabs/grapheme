use grapheme_artifact::artifact::ArtifactError;
use grapheme_artifact::{
    build_aot_from_artifact, build_artifact_from_mir, build_stage_b_container_from_aot,
    AotEnvelope, ArtifactEnvelope,
};

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

#[derive(Debug, Clone)]
pub struct CompiledAotScript {
    pub compilation: CompilationArtifact,
    pub artifact: ArtifactEnvelope,
    pub aot: AotEnvelope,
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

    pub fn compile_source_to_aot(
        source: &str,
        options: CompilerOptions,
    ) -> Result<CompiledAotScript, CompilerError> {
        let compiled = Self::compile_source(source, options)?;
        let aot = build_aot_from_artifact(&compiled.artifact).map_err(map_artifact_error)?;

        Ok(CompiledAotScript {
            compilation: compiled.compilation,
            artifact: compiled.artifact,
            aot,
        })
    }

    pub fn compile_source_to_aot_stage_b(
        source: &str,
        options: CompilerOptions,
        workflow_wasm: &[u8],
        allowed_imports: &[String],
    ) -> Result<CompiledAotScript, CompilerError> {
        let stage_a = Self::compile_source_to_aot(source, options)?;
        let stage_b = build_stage_b_container_from_aot(
            &stage_a.aot,
            workflow_wasm,
            allowed_imports,
        )
        .map_err(map_artifact_error)?;

        Ok(CompiledAotScript {
            compilation: stage_a.compilation,
            artifact: stage_a.artifact,
            aot: stage_b,
        })
    }
}

fn map_artifact_error(err: ArtifactError) -> CompilerError {
    CompilerError::ArtifactEmitError(err.to_string())
}
