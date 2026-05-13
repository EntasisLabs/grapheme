use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::mir::MirProgram;

/// Errors returned while building or validating artifact/AOT envelopes.
#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("artifact error: {0}")]
    Message(String),
}

/// Canonical artifact envelope produced from MIR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    /// Stable identifier derived from MIR payload hash.
    pub artifact_id: String,
    /// Artifact schema/compiler version tag.
    pub artifact_version: String,
    /// MIR function name used as runtime entrypoint.
    pub entrypoint: String,
    /// Capability ids required by this artifact.
    pub required_capabilities: Vec<String>,
    /// Location of payload bytes (currently inline).
    pub payload_ref: String,
    /// SHA-256 integrity hash of payload.
    pub integrity_hash: String,
    /// Typed artifact payload.
    pub payload: ArtifactPayload,
}

/// Artifact payload content and format metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPayload {
    /// Payload format identifier.
    pub format: String,
    /// Lowered MIR program.
    pub mir: MirProgram,
}

/// AOT lowering stage marker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AotStage {
    /// Stage A: host-runtime-parity envelope.
    StageA,
    /// Stage B: includes workflow Wasm container metadata.
    StageB,
}

/// Compiler/runtime compatibility metadata attached to AOT envelopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AotCompatibilityMetadata {
    /// Compiler version that produced this AOT envelope.
    pub compiler_version: String,
    /// Source artifact version.
    pub artifact_version: String,
    /// Source artifact integrity hash.
    pub artifact_integrity_hash: String,
    /// Runtime contract this AOT payload targets.
    pub runtime_contract: String,
}

/// AOT payload metadata and optional Stage B workflow container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AotPayload {
    /// Payload format identifier.
    pub format: String,
    /// Host interface id expected by this payload.
    pub host_interface_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Stage B workflow Wasm metadata, absent for Stage A.
    pub workflow_wasm: Option<AotWorkflowWasmContainer>,
}

/// Stage B workflow Wasm metadata and inline byte transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AotWorkflowWasmContainer {
    /// Workflow Wasm byte length.
    pub byte_len: u64,
    /// Workflow Wasm SHA-256 digest (`sha256:<hex>`).
    pub sha256: String,
    /// Entry export executed by runtime.
    pub entry_export: String,
    /// Allowed host imports constrained to runtime contract namespace.
    pub allowed_imports: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Optional inline Wasm bytes encoded as hex.
    pub inline_wasm_hex: Option<String>,
}

/// Top-level AOT envelope used by SDK/CLI/runtime execution paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AotEnvelope {
    /// Stable AOT identifier derived from artifact hash.
    pub aot_id: String,
    /// AOT schema/compiler version tag.
    pub aot_version: String,
    /// Lowering stage.
    pub stage: AotStage,
    /// Base source artifact.
    pub base_artifact: ArtifactEnvelope,
    /// Compiler/runtime compatibility fields.
    pub compatibility: AotCompatibilityMetadata,
    /// Stage-specific payload metadata.
    pub payload: AotPayload,
}

/// Runtime execution response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Terminal outcome classification.
    pub outcome: ExecutionOutcome,
    /// Optional emitted STTP node id.
    pub output_sttp_node_id: Option<String>,
    /// Trace step summary.
    pub trace_summary: TraceSummary,
    /// Optional human-readable execution message.
    pub message: Option<String>,
}

/// Normalized execution outcome classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcome {
    /// Execution completed successfully.
    Succeeded,
    /// Execution failed in a potentially retryable way.
    RetryableFailure,
    /// Execution failed fatally.
    FatalFailure,
}

/// Compact step-count trace summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    /// Total steps executed.
    pub steps: usize,
    /// Failing step index, if any.
    pub failed_step: Option<usize>,
}

/// Build a canonical artifact envelope from a MIR program.
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

/// Build a Stage A AOT envelope from an artifact envelope.
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

/// Build a Stage B envelope by attaching workflow container metadata to Stage A.
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

/// Validate host-interface boundary and stage-specific AOT payload invariants.
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
