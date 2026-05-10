use grapheme_artifact::{
    ArtifactEnvelope, Capability, CapabilityPolicy, ExecutionOutcome, ExecutionResult, MirFunction,
    MirInst, TraceSummary,
};
use serde_json::{Map, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::error::RuntimeError as GraphemeError;
use crate::host::{CapabilityCall, CapabilityHost, HostCallError};
use crate::module_manifest::ModuleAbi;
use crate::module_registry::ModuleRegistry;
use crate::policy::PolicyGuard;
use crate::state::AgentState;
#[cfg(feature = "wasix-runtime")]
use crate::wasix_backend::WasixBackend;

const DEFAULT_MAX_CALL_DEPTH: usize = 16;

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

        let functions = &artifact.payload.mir.functions;
        let function_index = build_function_index(functions);
        let entrypoint_index = function_index
            .get(artifact.entrypoint.as_str())
            .copied()
            .ok_or_else(|| {
                GraphemeError::ArtifactCompatibilityError(format!(
                    "entrypoint '{}' not found in artifact MIR",
                    artifact.entrypoint
                ))
            })?;

        let mut state = AgentState::new();
        let mut step_index = 0usize;

        if let Some(result) = self.execute_function(
            functions,
            &function_index,
            entrypoint_index,
            host,
            &mut state,
            &mut step_index,
            0,
            DEFAULT_MAX_CALL_DEPTH,
        )? {
            return Ok((state, result));
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

impl RuntimeEngine {
    fn execute_function(
        &self,
        functions: &[MirFunction],
        function_index: &HashMap<String, usize>,
        function_idx: usize,
        host: &mut dyn CapabilityHost,
        state: &mut AgentState,
        step_index: &mut usize,
        call_depth: usize,
        max_call_depth: usize,
    ) -> Result<Option<ExecutionResult>, GraphemeError> {
        let function = &functions[function_idx];
        let iteration_max = function
            .loop_config
            .as_ref()
            .map(|cfg| cfg.max as usize)
            .unwrap_or(1);

        for _iteration in 0..iteration_max {
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
                            let message =
                                format!("capability '{}' denied by runtime policy", capability.0);
                            return Ok(Some(fail_execution(
                                state,
                                *step_index,
                                capability,
                                "CAPABILITY_DENIED",
                                message,
                                ExecutionOutcome::FatalFailure,
                            )));
                        }

                        if is_call_step(module) {
                            let call_max_depth = resolve_call_max_depth(args, max_call_depth)?;
                            if call_depth + 1 > call_max_depth {
                                let message = format!(
                                    "max call depth exceeded while invoking '{}': depth {} > max_depth {}",
                                    op,
                                    call_depth + 1,
                                    call_max_depth
                                );
                                return Ok(Some(fail_execution(
                                    state,
                                    *step_index,
                                    capability,
                                    "MAX_CALL_DEPTH_EXCEEDED",
                                    message,
                                    ExecutionOutcome::FatalFailure,
                                )));
                            }

                            let target_index = function_index.get(op.as_str()).copied().ok_or_else(|| {
                                GraphemeError::RuntimeError(format!(
                                    "call target '{}' not found in artifact MIR",
                                    op
                                ))
                            })?;

                            if let Some(result) = self.execute_function(
                                functions,
                                function_index,
                                target_index,
                                host,
                                state,
                                step_index,
                                call_depth + 1,
                                call_max_depth,
                            )? {
                                return Ok(Some(result));
                            }

                            let output = state.current.clone();
                            state.advance_in_place(*step_index, capability.0.clone(), output);
                            *step_index += 1;
                            continue;
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
                            return Ok(Some(fail_execution(
                                state,
                                *step_index,
                                capability,
                                "POLICY_DENIED",
                                err.to_string(),
                                ExecutionOutcome::FatalFailure,
                            )));
                        }

                        if let Some(input) = call_args.get("__input") {
                            if let Some(error) = input.get("error") {
                                return Ok(Some(fail_execution(
                                    state,
                                    *step_index,
                                    capability,
                                    "EXECUTION_ERROR",
                                    error.to_string(),
                                    ExecutionOutcome::FatalFailure,
                                )));
                            }
                        }

                        let output = match resolved.abi {
                            ModuleAbi::MirV1 => {
                                let call = CapabilityCall {
                                    module: module.clone(),
                                    op: op.clone(),
                                    capability: capability.0.clone(),
                                    arg_count: *arg_count,
                                    args: call_args.clone(),
                                    step_index: *step_index,
                                };

                                match host.call(&call) {
                                    Ok(output) => output,
                                    Err(HostCallError::Retryable(message)) => {
                                        return Ok(Some(fail_execution(
                                            state,
                                            *step_index,
                                            capability,
                                            "RETRYABLE",
                                            message,
                                            ExecutionOutcome::RetryableFailure,
                                        )));
                                    }
                                    Err(HostCallError::Fatal(message)) => {
                                        return Ok(Some(fail_execution(
                                            state,
                                            *step_index,
                                            capability,
                                            "FATAL",
                                            message,
                                            ExecutionOutcome::FatalFailure,
                                        )));
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

                        state.advance_in_place(*step_index, capability.0.clone(), output);
                        *step_index += 1;
                        }
                    }
                }
            }

            if loop_until_satisfied(function, state) {
                break;
            }
        }

        Ok(None)
    }
}

fn build_function_index(functions: &[MirFunction]) -> HashMap<String, usize> {
    functions
        .iter()
        .enumerate()
        .map(|(idx, function)| (function.name.clone(), idx))
        .collect()
}

fn is_call_step(module: &Option<String>) -> bool {
    module
        .as_deref()
        .map(|m| m.eq_ignore_ascii_case("call"))
        .unwrap_or(false)
}

fn resolve_call_max_depth(args: &JsonValue, inherited_max_depth: usize) -> Result<usize, GraphemeError> {
    let Some(map) = args.as_object() else {
        return Ok(inherited_max_depth);
    };

    let Some(raw) = map.get("max_depth") else {
        return Ok(inherited_max_depth);
    };

    let value = raw.as_i64().ok_or_else(|| {
        GraphemeError::RuntimeError("call max_depth must be an integer".to_string())
    })?;

    if value < 1 {
        return Err(GraphemeError::RuntimeError(
            "call max_depth must be >= 1".to_string(),
        ));
    }

    Ok(value as usize)
}

fn fail_execution(
    state: &mut AgentState,
    step_index: usize,
    capability: &Capability,
    code: &str,
    message: String,
    outcome: ExecutionOutcome,
) -> ExecutionResult {
    state.fail_in_place(
        step_index,
        capability.0.clone(),
        code.to_string(),
        message.clone(),
    );

    ExecutionResult {
        outcome,
        output_sttp_node_id: None,
        trace_summary: TraceSummary {
            steps: step_index + 1,
            failed_step: Some(step_index),
        },
        message: Some(message),
    }
}

fn loop_until_satisfied(function: &MirFunction, state: &AgentState) -> bool {
    let Some(loop_cfg) = function.loop_config.as_ref() else {
        return false;
    };

    let Some(until) = loop_cfg.until.as_ref() else {
        return false;
    };

    state
        .current
        .as_object()
        .and_then(|obj| obj.get(&until.field))
        .map(|value| value == &until.eq)
        .unwrap_or(false)
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
    let mir_bytes = serde_json::to_vec(&artifact.payload.mir).map_err(|e| {
        GraphemeError::RuntimeError(format!("serialize MIR for integrity verification: {e}"))
    })?;

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
