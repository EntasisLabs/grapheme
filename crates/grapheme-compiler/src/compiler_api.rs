use grapheme_artifact::artifact::ArtifactError;
use grapheme_artifact::{
    build_aot_from_artifact, build_artifact_from_mir, build_stage_b_container_from_aot,
    AotEnvelope, ArtifactEnvelope,
};

use crate::pipeline::{self, CompileOptions, CompilationArtifact};
use crate::ast::{Definition, Program};
use crate::error::CompilerError;
use crate::parser;

/// High-level compile options for source-based compiler APIs.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// Lowering/verifier compile options.
    pub compile_options: CompileOptions,
    /// Optional explicit artifact entrypoint override.
    pub entrypoint: Option<String>,
}

/// Source compilation result containing the lowered artifact.
#[derive(Debug, Clone)]
pub struct CompiledScript {
    /// Full compilation pipeline outputs (AST/HIR/MIR + lints).
    pub compilation: CompilationArtifact,
    /// Emitted artifact envelope.
    pub artifact: ArtifactEnvelope,
}

/// Source compilation result including AOT output.
#[derive(Debug, Clone)]
pub struct CompiledAotScript {
    /// Full compilation pipeline outputs (AST/HIR/MIR + lints).
    pub compilation: CompilationArtifact,
    /// Emitted artifact envelope.
    pub artifact: ArtifactEnvelope,
    /// Emitted AOT envelope.
    pub aot: AotEnvelope,
}

/// Stateless compiler facade for source compilation APIs.
pub struct Compiler;

impl Compiler {
    /// Compile source into a `CompiledScript`.
    pub fn compile_source(source: &str, options: CompilerOptions) -> Result<CompiledScript, CompilerError> {
        let ast = parser::parse(source)?;
        let implicit_entrypoint = glyph_entrypoint(&ast)?;
        let compilation = pipeline::compile_program(ast, options.compile_options)?;
        let explicit_entrypoint = options.entrypoint.clone();
        let artifact_entrypoint = explicit_entrypoint
            .as_deref()
            .or(implicit_entrypoint.as_deref());
        let artifact = build_artifact_from_mir(&compilation.mir, artifact_entrypoint)
            .map_err(map_artifact_error)?;

        Ok(CompiledScript {
            compilation,
            artifact,
        })
    }

    /// Compile source into Stage A AOT.
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

    /// Compile source into Stage B AOT with provided workflow bytes/import contract.
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

fn glyph_entrypoint(ast: &Program) -> Result<Option<String>, CompilerError> {
    let glyphs = ast
        .definitions
        .iter()
        .filter_map(|def| match def {
            Definition::Glyph(g) => Some(g.name.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    if glyphs.len() > 1 {
        return Err(CompilerError::TypeError(format!(
            "only one glyph is allowed per file, found: {}",
            glyphs.join(", ")
        )));
    }

    if glyphs.is_empty() {
        let executable_roots = ast
            .definitions
            .iter()
            .filter_map(|def| match def {
                Definition::Query(q) => Some(format!("query {}", q.name)),
                Definition::Mutation(m) => Some(format!("mutation {}", m.name)),
                _ => None,
            })
            .collect::<Vec<_>>();

        if executable_roots.len() > 1 {
            return Err(CompilerError::TypeError(format!(
                "ambiguous entrypoint: file has multiple query/mutation roots ({}) and no glyph; add `glyph <Name> {{ ... }}` or pass an explicit entrypoint",
                executable_roots.join(", ")
            )));
        }
    }

    Ok(glyphs.into_iter().next())
}

fn map_artifact_error(err: ArtifactError) -> CompilerError {
    CompilerError::ArtifactEmitError(err.to_string())
}
