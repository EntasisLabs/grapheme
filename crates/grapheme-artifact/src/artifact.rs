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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AotStage {
    StageA,
    StageB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AotCompatibilityMetadata {
    pub compiler_version: String,
    pub artifact_version: String,
    pub artifact_integrity_hash: String,
    pub runtime_contract: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AotPayload {
    pub format: String,
    pub host_interface_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_wasm: Option<AotWorkflowWasmContainer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AotWorkflowWasmContainer {
    pub byte_len: u64,
    pub sha256: String,
    pub entry_export: String,
    pub allowed_imports: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_wasm_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AotEnvelope {
    pub aot_id: String,
    pub aot_version: String,
    pub stage: AotStage,
    pub base_artifact: ArtifactEnvelope,
    pub compatibility: AotCompatibilityMetadata,
    pub payload: AotPayload,
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

pub fn build_aot_from_artifact(artifact: &ArtifactEnvelope) -> Result<AotEnvelope, ArtifactError> {
    if artifact.payload.format != "grapheme.mir.v1" {
        return Err(ArtifactError::Message(format!(
            "unsupported artifact payload format for AOT lowering: '{}'",
            artifact.payload.format
        )));
    }

    let artifact_bytes = serde_json::to_vec(artifact)
        .map_err(|e| ArtifactError::Message(format!("serialize artifact for AOT hashing: {e}")))?;
    let hash = Sha256::digest(&artifact_bytes);
    let hash_hex = hex::encode(hash);

    Ok(AotEnvelope {
        aot_id: format!("aot-{}", &hash_hex[..16]),
        aot_version: env!("CARGO_PKG_VERSION").to_string(),
        stage: AotStage::StageA,
        base_artifact: artifact.clone(),
        compatibility: AotCompatibilityMetadata {
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            artifact_version: artifact.artifact_version.clone(),
            artifact_integrity_hash: artifact.integrity_hash.clone(),
            runtime_contract: "grapheme.runtime.host.v1".to_string(),
        },
        payload: AotPayload {
            format: "grapheme.aot.stage_a.v1".to_string(),
            host_interface_id: "grapheme.runtime.host.v1".to_string(),
            workflow_wasm: None,
        },
    })
}

pub fn build_stage_b_container_from_aot(
    stage_a: &AotEnvelope,
    workflow_wasm: &[u8],
    allowed_imports: &[String],
) -> Result<AotEnvelope, ArtifactError> {
    if stage_a.stage != AotStage::StageA {
        return Err(ArtifactError::Message(
            "stage_b build requires a stage_a envelope as input".to_string(),
        ));
    }

    validate_aot_host_interface_boundary(stage_a)?;

    let invalid_imports = allowed_imports
        .iter()
        .filter(|import| !import.starts_with("grapheme.runtime.host.v1::"))
        .cloned()
        .collect::<Vec<_>>();
    if !invalid_imports.is_empty() {
        return Err(ArtifactError::Message(format!(
            "stage_b contains imports outside host interface boundary: {invalid_imports:?}"
        )));
    }

    let mut imports = allowed_imports.to_vec();
    imports.sort();
    imports.dedup();

    let hash = Sha256::digest(workflow_wasm);
    let hash_hex = hex::encode(hash);

    let mut stage_b = stage_a.clone();
    stage_b.stage = AotStage::StageB;
    stage_b.payload = AotPayload {
        format: "grapheme.aot.stage_b.v1".to_string(),
        host_interface_id: stage_a.payload.host_interface_id.clone(),
        workflow_wasm: Some(AotWorkflowWasmContainer {
            byte_len: workflow_wasm.len() as u64,
            sha256: format!("sha256:{hash_hex}"),
            entry_export: "_start".to_string(),
            allowed_imports: imports,
            inline_wasm_hex: Some(hex::encode(workflow_wasm)),
        }),
    };

    Ok(stage_b)
}

pub fn validate_aot_host_interface_boundary(aot: &AotEnvelope) -> Result<(), ArtifactError> {
    if aot.payload.host_interface_id != aot.compatibility.runtime_contract {
        return Err(ArtifactError::Message(format!(
            "aot host interface mismatch: payload='{}' compatibility='{}'",
            aot.payload.host_interface_id, aot.compatibility.runtime_contract
        )));
    }

    match aot.stage {
        AotStage::StageA => {
            if aot.payload.format != "grapheme.aot.stage_a.v1" {
                return Err(ArtifactError::Message(format!(
                    "stage_a payload format mismatch: '{}'",
                    aot.payload.format
                )));
            }
            if aot.payload.workflow_wasm.is_some() {
                return Err(ArtifactError::Message(
                    "stage_a payload must not include workflow_wasm container".to_string(),
                ));
            }
        }
        AotStage::StageB => {
            if aot.payload.format != "grapheme.aot.stage_b.v1" {
                return Err(ArtifactError::Message(format!(
                    "stage_b payload format mismatch: '{}'",
                    aot.payload.format
                )));
            }
            let Some(container) = aot.payload.workflow_wasm.as_ref() else {
                return Err(ArtifactError::Message(
                    "stage_b payload requires workflow_wasm container metadata".to_string(),
                ));
            };
            if container.entry_export.trim().is_empty() {
                return Err(ArtifactError::Message(
                    "stage_b workflow entry export cannot be empty".to_string(),
                ));
            }

            if let Some(inline_wasm_hex) = &container.inline_wasm_hex {
                let wasm_bytes = hex::decode(inline_wasm_hex).map_err(|e| {
                    ArtifactError::Message(format!(
                        "stage_b inline_wasm_hex is not valid hex: {e}"
                    ))
                })?;

                if wasm_bytes.len() as u64 != container.byte_len {
                    return Err(ArtifactError::Message(format!(
                        "stage_b workflow byte_len mismatch: metadata={} actual={}",
                        container.byte_len,
                        wasm_bytes.len()
                    )));
                }

                let hash = Sha256::digest(&wasm_bytes);
                let hash_hex = format!("sha256:{}", hex::encode(hash));
                if hash_hex != container.sha256 {
                    return Err(ArtifactError::Message(format!(
                        "stage_b workflow sha256 mismatch: metadata='{}' actual='{}'",
                        container.sha256, hash_hex
                    )));
                }
            }

            let invalid_imports = container
                .allowed_imports
                .iter()
                .filter(|import| !import.starts_with("grapheme.runtime.host.v1::"))
                .cloned()
                .collect::<Vec<_>>();
            if !invalid_imports.is_empty() {
                return Err(ArtifactError::Message(format!(
                    "stage_b contains imports outside host interface boundary: {invalid_imports:?}"
                )));
            }
        }
    }

    Ok(())
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
