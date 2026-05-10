use grapheme_artifact::{
    ArtifactEnvelope, Capability, CapabilityPolicy, ExecutionOutcome, ExecutionResult, MirFunction,
    MirInst, MirLoopMergeMode, TraceSummary,
};
use serde_json::{Map, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::error::RuntimeError as GraphemeError;
use crate::host::{CapabilityCall, CapabilityHost, HostCallError};
use crate::module_manifest::ModuleAbi;
use crate::module_registry::ModuleRegistry;
use crate::policy::PolicyGuard;
use crate::state::{AgentState, StepContext, TracePolicy};
#[cfg(feature = "wasix-runtime")]
use crate::wasix_backend::WasixBackend;

const DEFAULT_MAX_CALL_DEPTH: usize = 16;

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub capability_policy: CapabilityPolicy,
    pub policy_guard: PolicyGuard,
    pub module_registry: ModuleRegistry,
    pub verify_integrity: bool,
    pub trace_policy: TracePolicy,
    pub stream_step_output: bool,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            capability_policy: CapabilityPolicy::default(),
            policy_guard: PolicyGuard::default(),
            module_registry: ModuleRegistry::default(),
            verify_integrity: true,
            trace_policy: TracePolicy::default(),
            stream_step_output: false,
        }
    }
}

struct LoopFrame<'a> {
    function: &'a MirFunction,
    max_iterations: usize,
    merge_mode: MirLoopMergeMode,
    input_snapshot: JsonValue,
    iteration_outputs: Vec<JsonValue>,
}

impl<'a> LoopFrame<'a> {
    fn new(function: &'a MirFunction, state: &AgentState) -> Self {
        Self {
            function,
            max_iterations: function
                .loop_config
                .as_ref()
                .map(|cfg| cfg.max as usize)
                .unwrap_or(1),
            merge_mode: function
                .loop_config
                .as_ref()
                .map(|cfg| cfg.merge.clone())
                .unwrap_or(MirLoopMergeMode::Replace),
            input_snapshot: state.current.clone(),
            iteration_outputs: Vec::new(),
        }
    }

    fn iteration_index(&self, iteration: usize) -> Option<usize> {
        self.function.loop_config.as_ref().map(|_| iteration)
    }

    fn until_satisfied(&self, state: &AgentState) -> bool {
        let Some(loop_cfg) = self.function.loop_config.as_ref() else {
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

    fn apply_merge(&self, state: &mut AgentState) {
        match self.merge_mode {
            // Replace is the current behavior: final loop iteration value is already in state.current.
            MirLoopMergeMode::Replace => {}
            MirLoopMergeMode::Append => {
                state.apply_loop_merge_current(JsonValue::Array(self.iteration_outputs.clone()));
            }
            MirLoopMergeMode::Reduce => {
                let reduced = reduce_iteration_outputs(&self.iteration_outputs);
                state.apply_loop_merge_current(reduced);
            }
            MirLoopMergeMode::None => {
                state.apply_loop_merge_current(self.input_snapshot.clone());
            }
        }
    }

    fn record_iteration(&mut self, state: &AgentState) {
        match self.merge_mode {
            MirLoopMergeMode::Append | MirLoopMergeMode::Reduce => {
                self.iteration_outputs.push(state.current.clone());
            }
            MirLoopMergeMode::Replace | MirLoopMergeMode::None => {}
        }
    }
}

pub struct RuntimeEngine {
    options: RuntimeOptions,
    #[cfg(feature = "wasix-runtime")]
    wasix_backend: WasixBackend,
}

impl RuntimeEngine {
    pub fn new(options: RuntimeOptions) -> Self {
        Self {
            options,
            #[cfg(feature = "wasix-runtime")]
            wasix_backend: WasixBackend::new(),
        }
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

        let mut state = AgentState::with_trace_policy(self.options.trace_policy.clone());
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
        let function_name = function.name.clone();
        let mut loop_frame = LoopFrame::new(function, state);

        for iteration in 0..loop_frame.max_iterations {
            let iteration_index = loop_frame.iteration_index(iteration);
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
                        let base_context = StepContext {
                            function_name: Some(function_name.clone()),
                            call_depth,
                            iteration_index,
                            call_target: None,
                        };

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
                                base_context,
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
                                    base_context,
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

                            state.record_passthrough_in_place(
                                *step_index,
                                capability.0.clone(),
                                StepContext {
                                    call_target: Some(op.clone()),
                                    ..base_context
                                },
                            );
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
                                base_context.clone(),
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
                                    base_context.clone(),
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
                                            base_context.clone(),
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
                                            base_context.clone(),
                                        )));
                                    }
                                }
                            }
                            ModuleAbi::WasixV1 | ModuleAbi::WasixWitV15 => {
                                #[cfg(feature = "wasix-runtime")]
                                {
                                    let path = resolved.wasm_path.as_deref().ok_or_else(|| {
                                        GraphemeError::RuntimeError(format!(
                                            "module '{}' requires wasm binding for op '{}'",
                                            resolved.module_id, resolved.op
                                        ))
                                    })?;
                                    // Transitional behavior: WIT ABI modules use the same backend path
                                    // until typed component-model invocation is implemented.
                                    self.wasix_backend.execute_call(path, &resolved, &call_args)?
                                }

                                #[cfg(not(feature = "wasix-runtime"))]
                                {
                                    return Err(GraphemeError::RuntimeError(
                                        "runtime built without wasix-runtime feature".to_string(),
                                    ));
                                }
                            }
                        };

                        if self.options.stream_step_output {
                            emit_streamed_step_output(op, &base_context, &output);
                        }

                        state.advance_in_place_with_context(
                            *step_index,
                            capability.0.clone(),
                            output,
                            base_context,
                        );
                        *step_index += 1;
                        }
                    }
                }
            }

            loop_frame.record_iteration(state);

            if loop_frame.until_satisfied(state) {
                break;
            }
        }

        loop_frame.apply_merge(state);

        Ok(None)
    }
}

fn reduce_iteration_outputs(outputs: &[JsonValue]) -> JsonValue {
    if outputs.is_empty() {
        return JsonValue::Null;
    }

    if outputs.iter().all(|value| value.is_number()) {
        let sum = outputs
            .iter()
            .filter_map(|value| value.as_f64())
            .fold(0.0, |acc, value| acc + value);
        return serde_json::Number::from_f64(sum)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null);
    }

    if outputs.iter().all(|value| value.is_array()) {
        let merged = outputs
            .iter()
            .filter_map(|value| value.as_array())
            .flat_map(|items| items.iter().cloned())
            .collect::<Vec<_>>();
        return JsonValue::Array(merged);
    }

    if outputs.iter().all(|value| value.is_object()) {
        let mut merged = serde_json::Map::new();
        for object in outputs.iter().filter_map(|value| value.as_object()) {
            for (key, value) in object {
                merged.insert(key.clone(), value.clone());
            }
        }
        return JsonValue::Object(merged);
    }

    outputs.last().cloned().unwrap_or(JsonValue::Null)
}

fn emit_streamed_step_output(op: &str, context: &StepContext, output: &JsonValue) {
    let Some(body) = printable_stream_body(output) else {
        return;
    };

    let mut prefix_parts = Vec::new();
    if let Some(iteration_index) = context.iteration_index {
        prefix_parts.push(format!("iter {}", iteration_index + 1));
    }
    if context.call_depth > 0 {
        prefix_parts.push(format!("depth {}", context.call_depth));
    }
    prefix_parts.push(op.to_string());

    println!("[{}] {}", prefix_parts.join(" | "), body);
}

fn printable_stream_body(value: &JsonValue) -> Option<String> {
    if let Some(message) = value.get("message").and_then(|v| v.as_str()) {
        return Some(message.to_string());
    }
    if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }
    if let Some(stdout) = value.get("stdout").and_then(|v| v.as_str()) {
        return Some(stdout.to_string());
    }
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }

    if value.is_null() {
        return None;
    }

    serde_json::to_string(value).ok()
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
    context: StepContext,
) -> ExecutionResult {
    state.fail_in_place_with_context(
        step_index,
        capability.0.clone(),
        code.to_string(),
        message.clone(),
        context,
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

#[cfg(test)]
mod tests {
    use super::*;
    use grapheme_artifact::{
        build_artifact_from_mir, Capability, MirBlock, MirFunction, MirFunctionKind, MirInst,
        MirLoopConfig, MirLoopMergeMode, MirProgram, MirTerminator,
    };
    use serde_json::{json, Map, Value as JsonValue};

    struct TestHost {
        mode: HostMode,
    }

    enum HostMode {
        StepIndexNumber,
        VerboseObject,
        LongString,
    }

    impl CapabilityHost for TestHost {
        fn call(&mut self, call: &CapabilityCall) -> Result<JsonValue, HostCallError> {
            match self.mode {
                HostMode::StepIndexNumber => Ok(JsonValue::from(call.step_index as i64)),
                HostMode::VerboseObject => Ok(json!({
                    "message": "ok",
                    "payload": "abcdefghijklmnopqrstuvwxyz",
                })),
                HostMode::LongString => Ok(JsonValue::String("abcdefghijklmnopqrstuvwxyz".to_string())),
            }
        }
    }

    #[test]
    fn loop_merge_append_collects_iteration_outputs() {
        let state = execute_loop(3, MirLoopMergeMode::Append, TracePolicy::lean_default(), HostMode::StepIndexNumber);
        assert_eq!(state.current, json!([0, 1, 2]));
    }

    #[test]
    fn loop_merge_reduce_sums_numeric_outputs() {
        let state = execute_loop(3, MirLoopMergeMode::Reduce, TracePolicy::lean_default(), HostMode::StepIndexNumber);
        assert_eq!(state.current, json!(3.0));
    }

    #[test]
    fn loop_merge_none_restores_pre_loop_state() {
        let state = execute_loop(3, MirLoopMergeMode::None, TracePolicy::lean_default(), HostMode::StepIndexNumber);
        assert_eq!(state.current, JsonValue::Null);
    }

    #[test]
    fn trace_policy_limits_pipeline_history() {
        let mut policy = TracePolicy::lean_default();
        policy.max_pipeline_steps = 2;
        policy.projection = crate::state::TraceProjection::Full;

        let state = execute_loop(6, MirLoopMergeMode::Replace, policy, HostMode::StepIndexNumber);
        assert_eq!(state.pipeline.len(), 2);
        assert_eq!(state.pipeline[0].output, json!(4));
        assert_eq!(state.pipeline[1].output, json!(5));
    }

    #[test]
    fn trace_policy_minimal_projection_keeps_small_summary() {
        let mut policy = TracePolicy::lean_default();
        policy.max_pipeline_steps = 8;
        policy.max_string_bytes = 8;
        policy.projection = crate::state::TraceProjection::Minimal;

        let state = execute_loop(1, MirLoopMergeMode::Replace, policy, HostMode::VerboseObject);
        let output = state.pipeline.first().expect("pipeline step").output.as_object().expect("object output");
        assert_eq!(output.get("message"), Some(&JsonValue::String("ok".to_string())));
        assert!(output.get("payload").is_none());
        assert_eq!(output.get("_kind"), Some(&JsonValue::String("object".to_string())));
    }

    #[test]
    fn trace_policy_zero_steps_drops_pipeline_history() {
        let mut policy = TracePolicy::lean_default();
        policy.max_pipeline_steps = 0;

        let state = execute_loop(4, MirLoopMergeMode::Replace, policy, HostMode::StepIndexNumber);
        assert!(state.pipeline.is_empty());
        assert_eq!(state.current, json!(3));
    }

    #[test]
    fn trace_policy_truncates_long_strings_in_minimal_mode() {
        let mut policy = TracePolicy::lean_default();
        policy.max_pipeline_steps = 4;
        policy.max_string_bytes = 5;
        policy.projection = crate::state::TraceProjection::Minimal;

        let state = execute_loop(1, MirLoopMergeMode::Replace, policy, HostMode::LongString);
        let output = state
            .pipeline
            .first()
            .expect("pipeline step")
            .output
            .as_str()
            .expect("string output");
        assert_eq!(output, "abcde...");
    }

    fn execute_loop(
        max: u32,
        merge: MirLoopMergeMode,
        trace_policy: TracePolicy,
        host_mode: HostMode,
    ) -> AgentState {
        let artifact = loop_artifact(max, merge);
        let mut host = TestHost { mode: host_mode };

        let mut options = RuntimeOptions::default();
        options.trace_policy = trace_policy;

        let runtime = RuntimeEngine::new(options);
        let (state, result) = runtime
            .execute_artifact(&artifact, &mut host)
            .expect("runtime execution succeeds");

        assert!(matches!(result.outcome, ExecutionOutcome::Succeeded));
        state
    }

    fn loop_artifact(max: u32, merge: MirLoopMergeMode) -> ArtifactEnvelope {
        let capability = Capability::from_module_op("core", "echo");
        let instruction = MirInst::Call {
            module: Some("core".to_string()),
            op: "echo".to_string(),
            capability: capability.clone(),
            arg_count: 0,
            args: JsonValue::Object(Map::new()),
            stores_state: true,
        };

        let function = MirFunction {
            name: "Main".to_string(),
            kind: MirFunctionKind::Fragment,
            loop_config: Some(MirLoopConfig {
                max,
                until: None,
                merge,
            }),
            blocks: vec![MirBlock {
                id: 0,
                instructions: vec![instruction],
                terminator: MirTerminator::ReturnState,
            }],
        };

        let mir = MirProgram {
            functions: vec![function],
            capabilities: vec![capability],
        };

        build_artifact_from_mir(&mir, Some("Main")).expect("artifact builds")
    }
}
