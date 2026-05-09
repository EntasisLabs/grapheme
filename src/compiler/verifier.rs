use super::mir::MirProgram;
use super::{hir::HirProgram, capability::CapabilityPolicy};
use crate::error::GraphemeError;

pub fn verify_hir(hir: &HirProgram) -> Result<(), GraphemeError> {
    if hir.executable_defs.is_empty() {
        return Err(GraphemeError::VerificationError(
            "program contains no executable definitions".to_string(),
        ));
    }

    for def in &hir.executable_defs {
        if def.name.trim().is_empty() {
            return Err(GraphemeError::VerificationError(
                "executable definition has empty name".to_string(),
            ));
        }

        if def.pipelines.is_empty() {
            return Err(GraphemeError::VerificationError(format!(
                "definition '{}' has no pipeline",
                def.name
            )));
        }

        for (i, pipeline) in def.pipelines.iter().enumerate() {
            if pipeline.steps.is_empty() {
                return Err(GraphemeError::VerificationError(format!(
                    "definition '{}' has empty pipeline at index {}",
                    def.name, i
                )));
            }
        }
    }

    Ok(())
}

pub fn verify_mir(mir: &MirProgram, policy: &CapabilityPolicy) -> Result<(), GraphemeError> {
    if mir.functions.is_empty() {
        return Err(GraphemeError::VerificationError(
            "MIR has no functions".to_string(),
        ));
    }

    for function in &mir.functions {
        if function.blocks.is_empty() {
            return Err(GraphemeError::VerificationError(format!(
                "MIR function '{}' has no blocks",
                function.name
            )));
        }
    }

    for capability in &mir.capabilities {
        if !policy.is_allowed(capability) {
            return Err(GraphemeError::CapabilityDenied(capability.0.clone()));
        }
    }

    Ok(())
}
