use sha2::{Digest, Sha256};

use crate::artifact::{ArtifactEnvelope, ExecutionOutcome, ExecutionResult, TraceSummary};
use crate::compiler::capability::{Capability, CapabilityPolicy};
use crate::compiler::mir::MirInst;
use crate::error::GraphemeError;
use crate::host::{CapabilityCall, CapabilityHost, HostCallError};
use crate::state::AgentState;

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub capability_policy: CapabilityPolicy,
    pub verify_integrity: bool,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            capability_policy: CapabilityPolicy::default(),
            verify_integrity: true,
        }
    }
}

pub struct RuntimeEngine {
    options: RuntimeOptions,
}

impl RuntimeEngine {
    pub fn new(options: RuntimeOptions) -> Self {
        Self { options }
    }

    pub fn execute_artifact(
        &self,
        artifact: &ArtifactEnvelope,
        host: &mut dyn CapabilityHost,
    ) -> Result<(AgentState, ExecutionResult), GraphemeError> {
        verify_artifact_compatibility(artifact)?;
        if self.options.verify_integrity {
            verify_artifact_integrity(artifact)?;
        }

        let function = artifact
            .payload
            .mir
            .functions
            .iter()
            .find(|f| f.name == artifact.entrypoint)
            .ok_or_else(|| {
                GraphemeError::ArtifactCompatibilityError(format!(
                    "entrypoint '{}' not found in artifact MIR",
                    artifact.entrypoint
                ))
            })?;

        let mut state = AgentState::new();
        let mut step_index = 0usize;

        for block in &function.blocks {
            for inst in &block.instructions {
                match inst {
                    MirInst::Call {
                        capability,
                        arg_count,
                        ..
                    } => {
                        if !self.options.capability_policy.is_allowed(capability) {
                            let message = format!("capability '{}' denied by runtime policy", capability.0);
                            state = state.fail(
                                step_index,
                                capability.0.clone(),
                                "CAPABILITY_DENIED".to_string(),
                                message.clone(),
                            );

                            return Ok((
                                state,
                                ExecutionResult {
                                    outcome: ExecutionOutcome::FatalFailure,
                                    output_sttp_node_id: None,
                                    trace_summary: TraceSummary {
                                        steps: step_index + 1,
                                        failed_step: Some(step_index),
                                    },
                                    message: Some(message),
                                },
                            ));
                        }

                        let call = CapabilityCall {
                            capability: capability.0.clone(),
                            arg_count: *arg_count,
                            step_index,
                        };

                        match host.call(&call) {
                            Ok(output) => {
                                state = state.advance(step_index, capability.0.clone(), output);
                            }
                            Err(HostCallError::Retryable(message)) => {
                                state = state.fail(
                                    step_index,
                                    capability.0.clone(),
                                    "RETRYABLE".to_string(),
                                    message.clone(),
                                );

                                return Ok((
                                    state,
                                    ExecutionResult {
                                        outcome: ExecutionOutcome::RetryableFailure,
                                        output_sttp_node_id: None,
                                        trace_summary: TraceSummary {
                                            steps: step_index + 1,
                                            failed_step: Some(step_index),
                                        },
                                        message: Some(message),
                                    },
                                ));
                            }
                            Err(HostCallError::Fatal(message)) => {
                                state = state.fail(
                                    step_index,
                                    capability.0.clone(),
                                    "FATAL".to_string(),
                                    message.clone(),
                                );

                                return Ok((
                                    state,
                                    ExecutionResult {
                                        outcome: ExecutionOutcome::FatalFailure,
                                        output_sttp_node_id: None,
                                        trace_summary: TraceSummary {
                                            steps: step_index + 1,
                                            failed_step: Some(step_index),
                                        },
                                        message: Some(message),
                                    },
                                ));
                            }
                        }

                        step_index += 1;
                    }
                }
            }
        }

        Ok((
            state,
            ExecutionResult {
                outcome: ExecutionOutcome::Succeeded,
                output_sttp_node_id: None,
                trace_summary: TraceSummary {
                    steps: step_index,
                    failed_step: None,
                },
                message: None,
            },
        ))
    }
}

impl Default for RuntimeEngine {
    fn default() -> Self {
        Self::new(RuntimeOptions::default())
    }
}

fn verify_artifact_compatibility(artifact: &ArtifactEnvelope) -> Result<(), GraphemeError> {
    if artifact.payload.format != "grapheme.mir.v1" {
        return Err(GraphemeError::ArtifactCompatibilityError(format!(
            "unsupported payload format '{}'",
            artifact.payload.format
        )));
    }

    Ok(())
}

fn verify_artifact_integrity(artifact: &ArtifactEnvelope) -> Result<(), GraphemeError> {
    let mir_bytes = serde_json::to_vec(&artifact.payload.mir)
        .map_err(|e| GraphemeError::RuntimeError(format!("serialize MIR for integrity verification: {e}")))?;

    let hash = Sha256::digest(&mir_bytes);
    let hash_hex = hex::encode(hash);
    let expected = format!("sha256:{hash_hex}");

    if artifact.integrity_hash != expected {
        return Err(GraphemeError::ArtifactIntegrityError(format!(
            "artifact integrity mismatch: expected '{}', got '{}'",
            expected, artifact.integrity_hash
        )));
    }

    for cap in &artifact.required_capabilities {
        let capability = Capability(cap.clone());
        if !artifact.payload.mir.capabilities.contains(&capability) {
            return Err(GraphemeError::ArtifactCompatibilityError(format!(
                "required capability '{}' missing from MIR capability set",
                cap
            )));
        }
    }

    Ok(())
}
