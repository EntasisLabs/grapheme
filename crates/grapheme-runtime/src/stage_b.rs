//! Stage B container execution with host import fulfillment.
//!
//! Runs the same MIR walker as `grapheme-aot-container` in-process, fulfilling
//! `grapheme.runtime.host.v1::call.capability` (and state read/write) via
//! [`CapabilityHost`] across rounds.

use grapheme_aot_container::{
    host::{is_host_call_error, CALL_CAPABILITY_IMPORT, STATE_READ_IMPORT, STATE_WRITE_IMPORT},
    ExecuteRequest, HostFulfillment,
};
use grapheme_artifact::{
    AotEnvelope, Capability, ExecutionOutcome, ExecutionResult, TraceSummary,
};
use serde_json::{json, Map, Value as JsonValue};

use crate::error::RuntimeError as GraphemeError;
use crate::host::{CapabilityCall, CapabilityHost, HostCallError};
use crate::state::AgentState;

const MAX_HOST_ROUNDS: usize = 64;

pub struct StageBHostExecution {
    pub state: AgentState,
    pub result: ExecutionResult,
    pub rounds: usize,
    pub host_calls_fulfilled: usize,
}

pub fn execute_stage_b_in_process(
    aot: &AotEnvelope,
    host: &mut dyn CapabilityHost,
    trace_policy: crate::state::TracePolicy,
) -> Result<StageBHostExecution, GraphemeError> {
    let mir = &aot.base_artifact.payload.mir;
    let entrypoint = aot.base_artifact.entrypoint.clone();
    let mut fulfillments: Vec<HostFulfillment> = Vec::new();
    let mut host_calls_fulfilled = 0usize;
    let mut agent = AgentState::with_trace_policy(trace_policy);

    for round in 1..=MAX_HOST_ROUNDS {
        let request = ExecuteRequest {
            entrypoint: Some(entrypoint.clone()),
            mir: mir.clone(),
            initial_current: Some(agent.current.clone()),
            args: None,
            allowed_imports: Some(grapheme_aot_container::default_allowed_imports()),
            host_fulfillments: fulfillments.clone(),
        };

        let walk = grapheme_aot_container::execute(&request).map_err(|err| {
            GraphemeError::RuntimeError(format!(
                "stage_b container walk failed: {} ({})",
                err.message, err.code
            ))
        })?;

        if walk.ok {
            agent.advance_in_place(
                walk.steps.saturating_sub(1),
                format!("aot.stage_b::{}", entrypoint),
                walk.current.clone(),
            );
            return Ok(StageBHostExecution {
                state: agent,
                result: ExecutionResult {
                    outcome: ExecutionOutcome::Succeeded,
                    output_sttp_node_id: None,
                    trace_summary: TraceSummary {
                        steps: walk.steps,
                        failed_step: None,
                    },
                    message: Some(format!(
                        "stage_b container executed in-process (rounds={round}, host_calls={host_calls_fulfilled})"
                    )),
                },
                rounds: round,
                host_calls_fulfilled,
            });
        }

        let err = walk.error.as_ref().ok_or_else(|| {
            GraphemeError::RuntimeError(
                "stage_b container returned ok=false without error payload".to_string(),
            )
        })?;

        if err.code != "HOST_CALL_REQUIRED" {
            agent.fail_in_place(
                err.step_index,
                err.capability
                    .clone()
                    .unwrap_or_else(|| "aot.stage_b".to_string()),
                err.code.clone(),
                err.message.clone(),
            );
            return Ok(StageBHostExecution {
                state: agent,
                result: ExecutionResult {
                    outcome: ExecutionOutcome::FatalFailure,
                    output_sttp_node_id: None,
                    trace_summary: TraceSummary {
                        steps: walk.steps,
                        failed_step: Some(err.step_index),
                    },
                    message: Some(err.message.clone()),
                },
                rounds: round,
                host_calls_fulfilled,
            });
        }

        let stub = walk
            .host_calls
            .last()
            .cloned()
            .or_else(|| {
                if is_host_call_error(&walk.current) {
                    Some(walk.current.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                GraphemeError::RuntimeError(
                    "HOST_CALL_REQUIRED without host_calls stub payload".to_string(),
                )
            })?;

        let fulfilled = fulfill_host_stub(host, &stub, &mut agent, err.step_index)?;
        fulfillments.push(HostFulfillment {
            step_index: err.step_index,
            result: fulfilled,
        });
        host_calls_fulfilled += 1;
    }

    Err(GraphemeError::RuntimeError(format!(
        "stage_b host fulfillment exceeded max rounds ({MAX_HOST_ROUNDS})"
    )))
}

fn fulfill_host_stub(
    host: &mut dyn CapabilityHost,
    stub: &JsonValue,
    agent: &mut AgentState,
    step_index: usize,
) -> Result<JsonValue, GraphemeError> {
    let error = stub.get("error").ok_or_else(|| {
        GraphemeError::RuntimeError("host stub missing error object".to_string())
    })?;
    let import = error
        .get("import")
        .and_then(|v| v.as_str())
        .unwrap_or(CALL_CAPABILITY_IMPORT);

    if import == STATE_READ_IMPORT {
        return Ok(agent.current.clone());
    }
    if import == STATE_WRITE_IMPORT {
        let value = error
            .get("args")
            .and_then(|args| args.get("value"))
            .cloned()
            .unwrap_or(JsonValue::Null);
        agent.current = value.clone();
        return Ok(value);
    }

    let module = error
        .get("module")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let op = error
        .get("op")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let capability = error
        .get("capability")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let args = error
        .get("args")
        .cloned()
        .unwrap_or_else(|| JsonValue::Object(Map::new()));
    let arg_count = match &args {
        JsonValue::Object(map) => map.len() as u16,
        _ => 0,
    };

    let call = CapabilityCall {
        module,
        op,
        capability: capability.clone(),
        arg_count,
        args,
        step_index,
    };

    match host.call(&call) {
        Ok(value) => Ok(value),
        Err(HostCallError::Fatal(message)) | Err(HostCallError::Retryable(message)) => {
            Err(GraphemeError::RuntimeError(format!(
                "stage_b host fulfillment failed for '{}': {message}",
                Capability(capability).0
            )))
        }
    }
}

pub fn stage_b_host_event(rounds: usize, host_calls_fulfilled: usize) -> JsonValue {
    json!({
        "kind": "aot.stage_b.host_fulfilled",
        "rounds": rounds,
        "host_calls_fulfilled": host_calls_fulfilled,
    })
}
