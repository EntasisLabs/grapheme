use crate::ast::Program;
use crate::error::GraphemeError;

use super::capability::CapabilityPolicy;
use super::hir::{self, HirProgram};
use super::mir::{self, MirProgram};
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
    let hir = hir::lower_from_ast(&ast);
    verifier::verify_hir(&hir)?;

    let mir = mir::lower_from_hir(&hir);
    verifier::verify_mir(&mir, &options.capability_policy)?;

    Ok(CompilationArtifact { ast, hir, mir })
}
