use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compiler::CompilationArtifact;
use crate::error::GraphemeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    pub artifact_id: String,
    pub artifact_version: String,
    pub entrypoint: String,
    pub required_capabilities: Vec<String>,
    pub payload_ref: String,
    pub integrity_hash: String,
    pub payload: ArtifactPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPayload {
    pub format: String,
    pub mir: crate::compiler::mir::MirProgram,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub outcome: ExecutionOutcome,
    pub output_sttp_node_id: Option<String>,
    pub trace_summary: TraceSummary,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcome {
    Succeeded,
    RetryableFailure,
    FatalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    pub steps: usize,
    pub failed_step: Option<usize>,
}

pub fn build_artifact_from_compilation(
    compilation: &CompilationArtifact,
    entrypoint: Option<&str>,
) -> Result<ArtifactEnvelope, GraphemeError> {
    let resolved_entrypoint = resolve_entrypoint(compilation, entrypoint)?;

    let mut required_capabilities = compilation
        .mir
        .capabilities
        .iter()
        .map(|c| c.0.clone())
        .collect::<Vec<_>>();
    required_capabilities.sort();
    required_capabilities.dedup();

    let mir_bytes = serde_json::to_vec(&compilation.mir)
        .map_err(|e| GraphemeError::RuntimeError(format!("serialize MIR for artifact hashing: {e}")))?;

    let hash = Sha256::digest(&mir_bytes);
    let hash_hex = hex::encode(hash);
    let artifact_id = format!("gph-{}", &hash_hex[..16]);

    Ok(ArtifactEnvelope {
        artifact_id,
        artifact_version: env!("CARGO_PKG_VERSION").to_string(),
        entrypoint: resolved_entrypoint,
        required_capabilities,
        payload_ref: "inline:mir".to_string(),
        integrity_hash: format!("sha256:{hash_hex}"),
        payload: ArtifactPayload {
            format: "grapheme.mir.v1".to_string(),
            mir: compilation.mir.clone(),
        },
    })
}

fn resolve_entrypoint(
    compilation: &CompilationArtifact,
    entrypoint: Option<&str>,
) -> Result<String, GraphemeError> {
    if let Some(ep) = entrypoint {
        let exists = compilation.mir.functions.iter().any(|f| f.name == ep);
        if exists {
            return Ok(ep.to_string());
        }

        return Err(GraphemeError::VerificationError(format!(
            "entrypoint '{}' does not exist in MIR",
            ep
        )));
    }

    compilation
        .mir
        .functions
        .first()
        .map(|f| f.name.clone())
        .ok_or_else(|| GraphemeError::VerificationError("no MIR functions available for entrypoint selection".to_string()))
}
