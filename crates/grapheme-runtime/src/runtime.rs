use grapheme_artifact::{
    ArtifactEnvelope, Capability, CapabilityPolicy, ExecutionOutcome, ExecutionResult, MirFunction,
    MirInst, MirLoopMergeMode, TraceSummary,
};
use grapheme_artifact::mir::{MirCompareOp, MirMatchTarget};
use serde_json::{Map, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

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
    pub max_steps: Option<usize>,
    pub max_call_depth: Option<usize>,
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
            max_steps: Some(100_000),
            max_call_depth: Some(DEFAULT_MAX_CALL_DEPTH),
        }
    }
}

struct LoopFrame<'a> {
    function: &'a MirFunction,
    max_iterations: usize,
    merge_mode: MirLoopMergeMode,
    input_snapshot: JsonValue,
    each_inputs: Option<Vec<JsonValue>>,
    iteration_outputs: Vec<JsonValue>,
}

impl<'a> LoopFrame<'a> {
    fn new(function: &'a MirFunction, state: &AgentState) -> Self {
        let input_snapshot = state.current.clone();
        let each_inputs = function
            .loop_config
            .as_ref()
            .and_then(|cfg| cfg.each.as_deref())
            .map(|selector| resolve_each_inputs(selector, &input_snapshot));
        let configured_max = function
            .loop_config
            .as_ref()
            .and_then(|cfg| cfg.max)
            .map(|max| max as usize)
            .unwrap_or_else(|| {
                if function.loop_config.is_some() {
                    usize::MAX
                } else {
                    1
                }
            });
        let max_iterations = each_inputs
            .as_ref()
            .map(|inputs| configured_max.min(inputs.len()))
            .unwrap_or(configured_max);

        Self {
            function,
            max_iterations,
            merge_mode: function
                .loop_config
                .as_ref()
                .map(|cfg| cfg.merge.clone())
                .unwrap_or(MirLoopMergeMode::Replace),
            input_snapshot,
            each_inputs,
            iteration_outputs: Vec::new(),
        }
    }

    fn apply_iteration_input(&self, state: &mut AgentState, iteration: usize) {
        let Some(inputs) = &self.each_inputs else {
            return;
        };

        if let Some(input) = inputs.get(iteration) {
            state.current = input.clone();
            state.diff = None;
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
        let mut remaining_steps = self.options.max_steps;
        let max_call_depth = self.options.max_call_depth.unwrap_or(usize::MAX);

        if let Some(result) = self.execute_function(
            functions,
            &function_index,
            entrypoint_index,
            host,
            &mut state,
            &mut step_index,
            &mut remaining_steps,
            0,
            max_call_depth,
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
        remaining_steps: &mut Option<usize>,
        call_depth: usize,
        max_call_depth: usize,
    ) -> Result<Option<ExecutionResult>, GraphemeError> {
        let function = &functions[function_idx];
        let retry_max_attempts = function
            .retry_config
            .as_ref()
            .map(|cfg| cfg.max.max(1) as usize)
            .unwrap_or(1);

        for attempt in 0..retry_max_attempts {
            let state_snapshot = state.clone();
            let step_snapshot = *step_index;
            let remaining_snapshot = *remaining_steps;

            let result = self.execute_function_once(
                functions,
                function_index,
                function_idx,
                host,
                state,
                step_index,
                remaining_steps,
                call_depth,
                max_call_depth,
            )?;

            if let Some(result_value) = result {
                if matches!(result_value.outcome, ExecutionOutcome::RetryableFailure)
                    && attempt + 1 < retry_max_attempts
                {
                    *state = state_snapshot;
                    *step_index = step_snapshot;
                    *remaining_steps = remaining_snapshot;
                    continue;
                }

                if matches!(result_value.outcome, ExecutionOutcome::RetryableFailure) {
                    if let Some(retry_cfg) = function.retry_config.as_ref() {
                        let base_context = StepContext {
                            function_name: Some(function.name.clone()),
                            call_depth,
                            iteration_index: None,
                            call_target: None,
                        };
                        return self.invoke_target(
                            functions,
                            function_index,
                            host,
                            state,
                            step_index,
                            remaining_steps,
                            call_depth,
                            max_call_depth,
                            &retry_cfg.on_fail,
                            "runtime.retry",
                            base_context,
                        );
                    }
                }

                return Ok(Some(result_value));
            }

            return Ok(None);
        }

        Ok(None)
    }

    fn execute_function_once(
        &self,
        functions: &[MirFunction],
        function_index: &HashMap<String, usize>,
        function_idx: usize,
        host: &mut dyn CapabilityHost,
        state: &mut AgentState,
        step_index: &mut usize,
        remaining_steps: &mut Option<usize>,
        call_depth: usize,
        max_call_depth: usize,
    ) -> Result<Option<ExecutionResult>, GraphemeError> {
        let function = &functions[function_idx];
        let function_name = function.name.clone();
        let mut loop_frame = LoopFrame::new(function, state);
        let timeout_started = Instant::now();

        for iteration in 0..loop_frame.max_iterations {
            loop_frame.apply_iteration_input(state, iteration);
            let iteration_index = loop_frame.iteration_index(iteration);
            for block in &function.blocks {
                for inst in &block.instructions {
                    let base_context = StepContext {
                        function_name: Some(function_name.clone()),
                        call_depth,
                        iteration_index,
                        call_target: None,
                    };

                    if let Some(timeout_cfg) = function.timeout_config.as_ref() {
                        if timeout_started.elapsed().as_millis() >= timeout_cfg.ms as u128 {
                            loop_frame.apply_merge(state);
                            return self.invoke_target(
                                functions,
                                function_index,
                                host,
                                state,
                                step_index,
                                remaining_steps,
                                call_depth,
                                max_call_depth,
                                &timeout_cfg.on_timeout,
                                "runtime.timeout",
                                base_context,
                            );
                        }
                    }

                    if !consume_step_budget(remaining_steps) {
                        return Ok(Some(fail_execution(
                            state,
                            *step_index,
                            &Capability::from_module_op("runtime", "step_budget"),
                            "STEP_BUDGET_EXCEEDED",
                            "runtime step budget exhausted".to_string(),
                            ExecutionOutcome::FatalFailure,
                            base_context,
                        )));
                    }

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
                                if let Some(result) = self.invoke_target(
                                    functions,
                                    function_index,
                                    host,
                                    state,
                                    step_index,
                                    remaining_steps,
                                    call_depth,
                                    call_max_depth,
                                    op,
                                    &capability.0,
                                    base_context,
                                )? {
                                    return Ok(Some(result));
                                }
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
                        MirInst::BranchCall {
                            field,
                            cmp,
                            value,
                            then_target,
                            else_target,
                            max_depth,
                        } => {
                            let base_context = StepContext {
                                function_name: Some(function_name.clone()),
                                call_depth,
                                iteration_index,
                                call_target: None,
                            };

                            let compare_to = resolve_current_templates(value, &state.current);
                            let branch_matches = select_json_path(&state.current, field)
                                .map(|current_value| branch_compare(current_value, cmp, &compare_to))
                                .unwrap_or(false);

                            let target = if branch_matches {
                                Some(then_target.as_str())
                            } else {
                                else_target.as_deref()
                            };

                            if let Some(target) = target {
                                if target == "$return" {
                                    loop_frame.apply_merge(state);
                                    return Ok(None);
                                }

                                let call_max_depth = max_depth
                                    .map(|v| v as usize)
                                    .unwrap_or(max_call_depth);

                                if let Some(result) = self.invoke_target(
                                    functions,
                                    function_index,
                                    host,
                                    state,
                                    step_index,
                                    remaining_steps,
                                    call_depth,
                                    call_max_depth,
                                    target,
                                    "flow.branch",
                                    base_context,
                                )? {
                                    return Ok(Some(result));
                                }
                            }
                        }
                        MirInst::MatchCall {
                            field,
                            cases,
                            default_target,
                            max_depth,
                        } => {
                            let base_context = StepContext {
                                function_name: Some(function_name.clone()),
                                call_depth,
                                iteration_index,
                                call_target: None,
                            };

                            let compare_value = select_json_path(&state.current, field);
                            let mut chosen = None;

                            if let Some(current_value) = compare_value {
                                for case in cases {
                                    let expected = resolve_current_templates(&case.eq, &state.current);
                                    if current_value == &expected {
                                        chosen = Some(&case.then_target);
                                        break;
                                    }
                                }
                            }

                            let resolved_target = chosen
                                .and_then(|target| resolve_match_target(&state.current, target))
                                .or_else(|| resolve_match_target(&state.current, default_target));

                            if let Some(target) = resolved_target {
                                if target == "$return" {
                                    loop_frame.apply_merge(state);
                                    return Ok(None);
                                }

                                let call_max_depth = max_depth
                                    .map(|v| v as usize)
                                    .unwrap_or(max_call_depth);

                                if let Some(result) = self.invoke_target(
                                    functions,
                                    function_index,
                                    host,
                                    state,
                                    step_index,
                                    remaining_steps,
                                    call_depth,
                                    call_max_depth,
                                    &target,
                                    "flow.match",
                                    base_context,
                                )? {
                                    return Ok(Some(result));
                                }
                            }
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

    fn invoke_target(
        &self,
        functions: &[MirFunction],
        function_index: &HashMap<String, usize>,
        host: &mut dyn CapabilityHost,
        state: &mut AgentState,
        step_index: &mut usize,
        remaining_steps: &mut Option<usize>,
        call_depth: usize,
        max_call_depth: usize,
        target: &str,
        capability_label: &str,
        base_context: StepContext,
    ) -> Result<Option<ExecutionResult>, GraphemeError> {
        if target == "$return" {
            return Ok(None);
        }

        if call_depth + 1 > max_call_depth {
            let message = format!(
                "max call depth exceeded while invoking '{}': depth {} > max_depth {}",
                target,
                call_depth + 1,
                max_call_depth
            );
            return Ok(Some(fail_execution(
                state,
                *step_index,
                &Capability::from_module_op("runtime", "call_depth"),
                "MAX_CALL_DEPTH_EXCEEDED",
                message,
                ExecutionOutcome::FatalFailure,
                base_context,
            )));
        }

        let target_index = function_index.get(target).copied().ok_or_else(|| {
            GraphemeError::RuntimeError(format!(
                "call target '{}' not found in artifact MIR",
                target
            ))
        })?;

        if let Some(result) = self.execute_function(
            functions,
            function_index,
            target_index,
            host,
            state,
            step_index,
            remaining_steps,
            call_depth + 1,
            max_call_depth,
        )? {
            return Ok(Some(result));
        }

        state.record_passthrough_in_place(
            *step_index,
            capability_label.to_string(),
            StepContext {
                call_target: Some(target.to_string()),
                ..base_context
            },
        );
        *step_index += 1;

        Ok(None)
    }
}

fn resolve_match_target(current: &JsonValue, target: &MirMatchTarget) -> Option<String> {
    match target {
        MirMatchTarget::Target(target) => Some(target.clone()),
        MirMatchTarget::Nested {
            field,
            cases,
            default_target,
        } => {
            let compare_value = select_json_path(current, field);
            if let Some(current_value) = compare_value {
                for case in cases {
                    let expected = resolve_current_templates(&case.eq, current);
                    if current_value == &expected {
                        return resolve_match_target(current, &case.then_target);
                    }
                }
            }

            resolve_match_target(current, default_target)
        }
    }
}

fn branch_compare(current_value: &JsonValue, cmp: &MirCompareOp, compare_to: &JsonValue) -> bool {
    match cmp {
        MirCompareOp::Eq => current_value == compare_to,
        MirCompareOp::Gt => compare_numbers(current_value, compare_to, |a, b| a > b),
        MirCompareOp::Gte => compare_numbers(current_value, compare_to, |a, b| a >= b),
        MirCompareOp::Lt => compare_numbers(current_value, compare_to, |a, b| a < b),
        MirCompareOp::Lte => compare_numbers(current_value, compare_to, |a, b| a <= b),
    }
}

fn compare_numbers(
    current_value: &JsonValue,
    compare_to: &JsonValue,
    predicate: impl Fn(f64, f64) -> bool,
) -> bool {
    let Some(a) = current_value.as_f64() else {
        return false;
    };
    let Some(b) = compare_to.as_f64() else {
        return false;
    };
    predicate(a, b)
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
    let Some(body) = printable_stream_body(op, output) else {
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

fn printable_stream_body(op: &str, value: &JsonValue) -> Option<String> {
    if op.eq_ignore_ascii_case("echo") {
        if let Some(message) = value.get("message").and_then(|v| v.as_str()) {
            return Some(message.to_string());
        }
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

    None
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
    let mut merged = match resolve_current_templates(args, input) {
        JsonValue::Object(map) => map,
        _ => Map::new(),
    };

    merged.insert("__input".to_string(), input.clone());
    JsonValue::Object(merged)
}

fn consume_step_budget(remaining_steps: &mut Option<usize>) -> bool {
    let Some(remaining) = remaining_steps.as_mut() else {
        return true;
    };

    if *remaining == 0 {
        return false;
    }

    *remaining -= 1;
    true
}

fn resolve_each_inputs(selector: &str, input_snapshot: &JsonValue) -> Vec<JsonValue> {
    if selector == "$current" {
        return input_snapshot
            .as_array()
            .cloned()
            .unwrap_or_default();
    }

    let Some(path) = selector.strip_prefix("$current.") else {
        return Vec::new();
    };

    let Some(selected) = select_json_path(input_snapshot, path) else {
        return Vec::new();
    };

    selected.as_array().cloned().unwrap_or_default()
}

fn resolve_current_templates(value: &JsonValue, current: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            if let Some(var_ref) = variable_ref_from_object(map) {
                return resolve_variable_reference(var_ref, current);
            }

            let mapped = map
                .iter()
                .map(|(k, v)| (k.clone(), resolve_current_templates(v, current)))
                .collect::<Map<String, JsonValue>>();
            JsonValue::Object(mapped)
        }
        JsonValue::Array(items) => JsonValue::Array(
            items
                .iter()
                .map(|item| resolve_current_templates(item, current))
                .collect(),
        ),
        JsonValue::String(s) => resolve_current_string_template(s, current),
        _ => value.clone(),
    }
}

fn variable_ref_from_object(map: &Map<String, JsonValue>) -> Option<&str> {
    if map.len() != 1 {
        return None;
    }

    map.get("$var")?.as_str()
}

fn resolve_variable_reference(reference: &str, current: &JsonValue) -> JsonValue {
    if reference == "current" {
        return current.clone();
    }

    if let Some(path) = reference.strip_prefix("current.") {
        return select_json_path(current, path)
            .cloned()
            .unwrap_or(JsonValue::Null);
    }

    JsonValue::String(format!("${reference}"))
}

fn resolve_current_string_template(template: &str, current: &JsonValue) -> JsonValue {
    if template == "$current" {
        return current.clone();
    }

    if let Some(path) = template.strip_prefix("$current.") {
        if path.chars().all(is_selector_char) {
            return select_json_path(current, path)
                .cloned()
                .unwrap_or(JsonValue::Null);
        }
    }

    let mut out = String::new();
    let bytes = template.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && template[i + 1..].starts_with("$current") {
            let mut j = i + 1 + "$current".len();
            let mut resolved = None;

            if j < bytes.len() && bytes[j] == b'.' {
                j += 1;
                let path_start = j;
                while j < bytes.len() && is_selector_char(bytes[j] as char) {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'}' {
                    let path = &template[path_start..j];
                    resolved = Some(
                        select_json_path(current, path)
                            .map(json_value_to_inline_string)
                            .unwrap_or_default(),
                    );
                    j += 1;
                }
            } else if j < bytes.len() && bytes[j] == b'}' {
                resolved = Some(json_value_to_inline_string(current));
                j += 1;
            }

            if let Some(text) = resolved {
                out.push_str(&text);
                i = j;
                continue;
            }
        }

        if bytes[i] == b'$' && template[i..].starts_with("$current") {
            let mut j = i + "$current".len();
            if j < bytes.len() && bytes[j] == b'.' {
                j += 1;
                while j < bytes.len() && is_selector_char(bytes[j] as char) {
                    j += 1;
                }
                let path = &template[i + "$current.".len()..j];
                if let Some(value) = select_json_path(current, path) {
                    out.push_str(&json_value_to_inline_string(value));
                }
                i = j;
                continue;
            }

            out.push_str(&json_value_to_inline_string(current));
            i += "$current".len();
            continue;
        }

        out.push(bytes[i] as char);
        i += 1;
    }

    JsonValue::String(out)
}

fn is_selector_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

fn select_json_path<'a>(root: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    let mut current = root;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        current = current.get(segment)?;
    }
    Some(current)
}

fn json_value_to_inline_string(value: &JsonValue) -> String {
    if let Some(s) = value.as_str() {
        return s.to_string();
    }

    serde_json::to_string(value).unwrap_or_default()
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

    #[test]
    fn loop_each_selector_reads_array_from_current_path() {
        let snapshot = json!({
            "jobs": [
                {"id": "a"},
                {"id": "b"}
            ]
        });

        let items = resolve_each_inputs("$current.jobs", &snapshot);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].get("id"), Some(&JsonValue::String("a".to_string())));
        assert_eq!(items[1].get("id"), Some(&JsonValue::String("b".to_string())));
    }

    #[test]
    fn args_with_pipeline_input_interpolates_current_templates() {
        let args = json!({
            "url": "https://example.com/job/$current.id",
            "payload": "$current",
            "id": "$current.id"
        });
        let current = json!({"id": "123", "status": "ready"});

        let resolved = args_with_pipeline_input(&args, &current);
        assert_eq!(
            resolved.get("url"),
            Some(&JsonValue::String("https://example.com/job/123".to_string()))
        );
        assert_eq!(resolved.get("payload"), Some(&current));
        assert_eq!(resolved.get("id"), Some(&JsonValue::String("123".to_string())));
        assert_eq!(resolved.get("__input"), Some(&current));
    }

    #[test]
    fn args_with_pipeline_input_interpolates_brace_current_templates() {
        let args = json!({
            "message": "fib:{$current.a}",
            "status": "{$current.status}",
            "snapshot": "{$current}"
        });
        let current = json!({"a": 21, "status": "ready"});

        let resolved = args_with_pipeline_input(&args, &current);
        assert_eq!(
            resolved.get("message"),
            Some(&JsonValue::String("fib:21".to_string()))
        );
        assert_eq!(
            resolved.get("status"),
            Some(&JsonValue::String("ready".to_string()))
        );
        assert_eq!(
            resolved.get("snapshot"),
            Some(&JsonValue::String("{\"a\":21,\"status\":\"ready\"}".to_string()))
        );
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
            retry_config: None,
            timeout_config: None,
            loop_config: Some(MirLoopConfig {
                max: Some(max),
                each: None,
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
