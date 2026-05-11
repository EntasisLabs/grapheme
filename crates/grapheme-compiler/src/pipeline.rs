use crate::ast::Program;
use crate::error::GraphemeError;
use grapheme_artifact::{CapabilityPolicy, MirProgram};

use super::hir::{self, HirProgram};
use super::mir_lower;
use super::verifier;

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub capability_policy: CapabilityPolicy,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            capability_policy: CapabilityPolicy::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompilationArtifact {
    pub ast: Program,
    pub hir: HirProgram,
    pub mir: MirProgram,
}

pub fn compile_program(ast: Program, options: CompileOptions) -> Result<CompilationArtifact, GraphemeError> {
    let hir = hir::lower_from_ast(&ast)?;
    verifier::verify_hir(&hir)?;

    let mir = mir_lower::lower_from_hir(&hir);
    verifier::verify_mir(&mir, &options.capability_policy)?;

    Ok(CompilationArtifact { ast, hir, mir })
}
