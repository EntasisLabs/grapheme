use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::mir::MirProgram;

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("artifact error: {0}")]
    Message(String),
}

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
    pub mir: MirProgram,
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

pub fn build_artifact_from_mir(
    mir: &MirProgram,
    entrypoint: Option<&str>,
) -> Result<ArtifactEnvelope, ArtifactError> {
    let resolved_entrypoint = resolve_entrypoint(mir, entrypoint)?;

    let mut required_capabilities = mir
        .capabilities
        .iter()
        .map(|c| c.0.clone())
        .collect::<Vec<_>>();
    required_capabilities.sort();
    required_capabilities.dedup();

    let mir_bytes = serde_json::to_vec(mir)
        .map_err(|e| ArtifactError::Message(format!("serialize MIR for artifact hashing: {e}")))?;

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
            mir: mir.clone(),
        },
    })
}

fn resolve_entrypoint(
    mir: &MirProgram,
    entrypoint: Option<&str>,
) -> Result<String, ArtifactError> {
    if let Some(ep) = entrypoint {
        let exists = mir.functions.iter().any(|f| f.name == ep);
        if exists {
            return Ok(ep.to_string());
        }

        return Err(ArtifactError::Message(format!(
            "entrypoint '{}' does not exist in MIR",
            ep
        )));
    }

    mir
        .functions
        .first()
        .map(|f| f.name.clone())
        .ok_or_else(|| ArtifactError::Message("no MIR functions available for entrypoint selection".to_string()))
}
