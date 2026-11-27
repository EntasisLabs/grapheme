use crate::ast::Program;
use crate::error::GraphemeError;
use crate::verifier::ExecutableKindPolicyMode;
use crate::verifier::LintWarning;
use grapheme_artifact::{CapabilityPolicy, MirProgram};

use super::hir::{self, HirProgram};
use super::mir_lower;
use super::verifier;

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub capability_policy: CapabilityPolicy,
    pub executable_kind_policy_mode: ExecutableKindPolicyMode,
    /// Elements, props, prop types, events and payload params checked while
    /// lowering views. The default is the five built-in elements. A document
    /// loaded with [`crate::ElementCatalog::parse`] replaces it. The compiler
    /// does not render the elements.
    pub element_catalog: crate::ElementCatalog,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            capability_policy: CapabilityPolicy::default(),
            executable_kind_policy_mode: ExecutableKindPolicyMode::Compatibility,
            element_catalog: crate::ElementCatalog::builtin(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompilationArtifact {
    pub ast: Program,
    pub hir: HirProgram,
    pub mir: MirProgram,
    pub lint_warnings: Vec<LintWarning>,
}

pub fn compile_program(
    ast: Program,
    options: CompileOptions,
) -> Result<CompilationArtifact, GraphemeError> {
    let hir = hir::lower_from_ast_with_catalog(&ast, &options.element_catalog)?;
    let lint_warnings = verifier::verify_hir_with_lints_mode_catalog(
        &hir,
        options.executable_kind_policy_mode,
        &options.element_catalog,
    )?;

    let mir = mir_lower::lower_from_hir(&hir);
    verifier::verify_mir(&mir, &options.capability_policy)?;

    Ok(CompilationArtifact {
        ast,
        hir,
        mir,
        lint_warnings,
    })
}
