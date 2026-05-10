use sha2::{Digest, Sha256};
use serde_json::{Map, Value as JsonValue};
use grapheme_artifact::{ArtifactEnvelope, Capability, CapabilityPolicy, ExecutionOutcome, ExecutionResult, MirInst, TraceSummary};

use crate::error::RuntimeError as GraphemeError;
use crate::host::{CapabilityCall, CapabilityHost, HostCallError};
use crate::module_manifest::ModuleAbi;
use crate::module_registry::ModuleRegistry;
use crate::policy::PolicyGuard;
use crate::state::AgentState;
#[cfg(feature = "wasix-runtime")]
use crate::wasix_backend::WasixBackend;

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub capability_policy: CapabilityPolicy,
    pub policy_guard: PolicyGuard,
    pub module_registry: ModuleRegistry,
    pub verify_integrity: bool,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            capability_policy: CapabilityPolicy::default(),
            policy_guard: PolicyGuard::default(),
            module_registry: ModuleRegistry::default(),
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
                        module,
                        op,
                        capability,
                        arg_count,
                        args,
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

                        let resolved = self
                            .options
                            .module_registry
                            .resolve_call(module.as_deref(), op, &capability.0)
                            .ok_or_else(|| {
                                GraphemeError::RuntimeError(format!(
                                    "module/op not registered for capability '{}': module={:?}, op={}",
                                    capability.0, module, op
                                ))
                            })?;

                        let call_args = args_with_pipeline_input(args, &state.current);

                        if let Err(err) = self.options.policy_guard.check(&resolved, &call_args) {
                            let message = err.to_string();
                            state = state.fail(
                                step_index,
                                capability.0.clone(),
                                "POLICY_DENIED".to_string(),
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

                        let output = match resolved.abi {
                            ModuleAbi::MirV1 => {
                                let call = CapabilityCall {
                                    module: module.clone(),
                                    op: op.clone(),
                                    capability: capability.0.clone(),
                                    arg_count: *arg_count,
                                    args: call_args.clone(),
                                    step_index,
                                };

                                match host.call(&call) {
                                    Ok(output) => output,
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
                            }
                            ModuleAbi::WasixV1 => {
                                #[cfg(feature = "wasix-runtime")]
                                {
                                    let path = resolved.wasm_path.as_deref().ok_or_else(|| {
                                        GraphemeError::RuntimeError(format!(
                                            "module '{}' requires wasm binding for op '{}'",
                                            resolved.module_id, resolved.op
                                        ))
                                    })?;
                                    let backend = WasixBackend::new();
                                    backend.execute_call(path, &resolved, &call_args)?
                                }

                                #[cfg(not(feature = "wasix-runtime"))]
                                {
                                    return Err(GraphemeError::RuntimeError(
                                        "runtime built without wasix-runtime feature".to_string(),
                                    ));
                                }
                            }
                        };

                        state = state.advance(step_index, capability.0.clone(), output);

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

fn args_with_pipeline_input(args: &JsonValue, input: &JsonValue) -> JsonValue {
    let mut merged = match args {
        JsonValue::Object(map) => map.clone(),
        _ => Map::new(),
    };

    merged.insert("__input".to_string(), input.clone());
    JsonValue::Object(merged)
}
