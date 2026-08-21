//! Slim MIR walker for Stage B workflow containers.

use grapheme_artifact::{
    Capability, MirCompareOp, MirFunction, MirInst, MirLoopMergeMode, MirMatchTarget, MirParam,
    MirProgram,
};
use grapheme_stdlib::registry;
use serde_json::{json, Map, Value as JsonValue};

use crate::host::{self, HostCallRequest, CALL_CAPABILITY_IMPORT};
use crate::templates::{args_with_pipeline_input, TemplateScope};
use crate::HostFulfillment;

/// Modules that may run inside the Stage B container even if Cargo feature
/// unification enables host stdlib crates in the same build graph.
const LOCAL_WASM_MODULES: &[&str] = &["core", "json", "csv", "yaml", "html"];

fn is_local_wasm_op(module: &str, op: &str) -> bool {
    LOCAL_WASM_MODULES.contains(&module) && registry::is_registered_op(module, op)
}

#[derive(Debug, Clone)]
pub struct WalkError {
    pub code: String,
    pub message: String,
    pub capability: Option<String>,
    pub step_index: usize,
}

#[derive(Debug, Clone)]
pub struct WalkResult {
    pub current: JsonValue,
    pub steps: usize,
    pub ok: bool,
    pub error: Option<WalkError>,
    pub host_calls: Vec<JsonValue>,
}

struct WalkState<'a> {
    current: JsonValue,
    steps: usize,
    host_calls: Vec<JsonValue>,
    host_fulfillments: &'a [HostFulfillment],
}

pub fn walk_program(
    mir: &MirProgram,
    entrypoint: &str,
    initial_current: JsonValue,
    call_args: &JsonValue,
    host_fulfillments: &[HostFulfillment],
) -> Result<WalkResult, WalkError> {
    let functions = &mir.functions;
    let function_idx = functions
        .iter()
        .position(|f| f.name == entrypoint)
        .ok_or_else(|| WalkError {
            code: "ENTRYPOINT_NOT_FOUND".to_string(),
            message: format!("entrypoint '{entrypoint}' not found in MIR"),
            capability: None,
            step_index: 0,
        })?;

    let locals = build_locals(&functions[function_idx].params, call_args, entrypoint)?;
    let mut state = WalkState {
        current: initial_current,
        steps: 0,
        host_calls: Vec::new(),
        host_fulfillments,
    };

    match execute_function(functions, function_idx, &mut state, &locals, 0, 32) {
        Ok(()) => Ok(WalkResult {
            current: state.current,
            steps: state.steps,
            ok: true,
            error: None,
            host_calls: state.host_calls,
        }),
        Err(err) => Ok(WalkResult {
            current: state.current,
            steps: state.steps,
            ok: false,
            error: Some(err),
            host_calls: state.host_calls,
        }),
    }
}

fn execute_function(
    functions: &[MirFunction],
    function_idx: usize,
    state: &mut WalkState<'_>,
    locals: &Map<String, JsonValue>,
    call_depth: usize,
    max_call_depth: usize,
) -> Result<(), WalkError> {
    if call_depth > max_call_depth {
        return Err(WalkError {
            code: "CALL_DEPTH_EXCEEDED".to_string(),
            message: format!("call depth exceeded max {max_call_depth}"),
            capability: Some("runtime.call_depth".to_string()),
            step_index: state.steps,
        });
    }

    let function = &functions[function_idx];
    let max_iterations = function
        .loop_config
        .as_ref()
        .and_then(|cfg| cfg.max)
        .unwrap_or(1)
        .max(1) as usize;
    let merge_mode = function
        .loop_config
        .as_ref()
        .map(|cfg| cfg.merge.clone())
        .unwrap_or(MirLoopMergeMode::Replace);
    let input_snapshot = state.current.clone();
    let mut iteration_outputs = Vec::new();

    for _iteration in 0..max_iterations {
        let mut frame_locals = locals.clone();
        for block in &function.blocks {
            for inst in &block.instructions {
                match inst {
                    MirInst::UsingEnter { activations, .. } => {
                        for activation in activations {
                            let fields = match &activation.fields {
                                JsonValue::Object(map) => map.clone(),
                                _ => Map::new(),
                            };
                            if let Some(handle) = &activation.handle {
                                frame_locals
                                    .insert(handle.clone(), JsonValue::Object(fields.clone()));
                            }
                            for (key, value) in fields {
                                frame_locals.insert(key, value);
                            }
                        }
                    }
                    MirInst::UsingExit { .. } => {}
                    MirInst::Call {
                        module,
                        op,
                        capability,
                        args,
                        ..
                    } => {
                        dispatch_call(
                            functions,
                            module.as_deref(),
                            op,
                            capability,
                            args,
                            state,
                            &frame_locals,
                            call_depth,
                            max_call_depth,
                        )?;
                    }
                    MirInst::BranchCall {
                        field,
                        cmp,
                        value,
                        then_target,
                        else_target,
                        max_depth,
                    } => {
                        let observed = state
                            .current
                            .as_object()
                            .and_then(|obj| obj.get(field))
                            .cloned()
                            .unwrap_or(JsonValue::Null);
                        let matched = compare_values(&observed, cmp, value);
                        let target = if matched {
                            Some(then_target.as_str())
                        } else {
                            else_target.as_deref()
                        };
                        if let Some(target) = target {
                            let depth = max_depth
                                .map(|d| d as usize)
                                .unwrap_or(max_call_depth)
                                .min(max_call_depth);
                            invoke_named(
                                functions,
                                target,
                                state,
                                &frame_locals,
                                call_depth,
                                depth,
                            )?;
                        }
                    }
                    MirInst::MatchCall {
                        field,
                        cases,
                        default_target,
                        max_depth,
                    } => {
                        let observed = state
                            .current
                            .as_object()
                            .and_then(|obj| obj.get(field))
                            .cloned()
                            .unwrap_or(JsonValue::Null);
                        let target = resolve_match_target(&observed, cases, default_target);
                        let depth = max_depth
                            .map(|d| d as usize)
                            .unwrap_or(max_call_depth)
                            .min(max_call_depth);
                        invoke_match_target(
                            functions,
                            &target,
                            state,
                            &frame_locals,
                            call_depth,
                            depth,
                        )?;
                    }
                }
            }
        }

        if matches!(
            merge_mode,
            MirLoopMergeMode::Append | MirLoopMergeMode::Reduce
        ) {
            iteration_outputs.push(state.current.clone());
        }

        if let Some(loop_cfg) = function.loop_config.as_ref() {
            if let Some(until) = loop_cfg.until.as_ref() {
                let done = state
                    .current
                    .as_object()
                    .and_then(|obj| obj.get(&until.field))
                    .map(|value| value == &until.eq)
                    .unwrap_or(false);
                if done {
                    break;
                }
            }
        }
    }

    match merge_mode {
        MirLoopMergeMode::Replace => {}
        MirLoopMergeMode::Append => {
            state.current = JsonValue::Array(iteration_outputs);
        }
        MirLoopMergeMode::Reduce => {
            state.current = reduce_iteration_outputs(&iteration_outputs);
        }
        MirLoopMergeMode::None => {
            state.current = input_snapshot;
        }
    }

    Ok(())
}

fn dispatch_call(
    functions: &[MirFunction],
    module: Option<&str>,
    op: &str,
    capability: &Capability,
    args: &JsonValue,
    state: &mut WalkState<'_>,
    locals: &Map<String, JsonValue>,
    call_depth: usize,
    max_call_depth: usize,
) -> Result<(), WalkError> {
    let module_id = module.unwrap_or("core");
    state.steps += 1;
    let step_index = state.steps.saturating_sub(1);

    let current = state.current.clone();
    let scope = TemplateScope {
        current: &current,
        state: &current,
        locals,
    };
    let call_args = args_with_pipeline_input(args, &current, &scope);

    if module == Some("call") {
        invoke_named(
            functions,
            op,
            state,
            locals,
            call_depth,
            max_call_depth,
        )?;
        return Ok(());
    }

    if is_local_wasm_op(module_id, op) {
        let Some(output) = registry::dispatch(module_id, op, &call_args) else {
            return Err(WalkError {
                code: "LOCAL_DISPATCH_FAILED".to_string(),
                message: format!("registered op '{module_id}.{op}' returned no value"),
                capability: Some(capability.0.clone()),
                step_index,
            });
        };
        if let Some(error) = output.get("error") {
            return Err(WalkError {
                code: error
                    .get("code")
                    .and_then(|c| c.as_str())
                    .unwrap_or("OP_ERROR")
                    .to_string(),
                message: error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("operation returned error")
                    .to_string(),
                capability: Some(capability.0.clone()),
                step_index,
            });
        }
        state.current = output;
        return Ok(());
    }

    // Host already fulfilled this step in a prior round.
    if let Some(fulfilled) = state
        .host_fulfillments
        .iter()
        .find(|f| f.step_index == step_index)
    {
        state.current = fulfilled.result.clone();
        return Ok(());
    }

    let host_req = HostCallRequest {
        import: CALL_CAPABILITY_IMPORT.to_string(),
        module: module_id.to_string(),
        op: op.to_string(),
        capability: capability.0.clone(),
        args: call_args,
    };
    let stub = host::stub_call_capability(&host_req);
    state.host_calls.push(stub.clone());
    state.current = stub.clone();

    Err(WalkError {
        code: "HOST_CALL_REQUIRED".to_string(),
        message: stub
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("host capability call required")
            .to_string(),
        capability: Some(capability.0.clone()),
        step_index,
    })
}

fn invoke_named(
    functions: &[MirFunction],
    name: &str,
    state: &mut WalkState<'_>,
    locals: &Map<String, JsonValue>,
    call_depth: usize,
    max_call_depth: usize,
) -> Result<(), WalkError> {
    let idx = functions
        .iter()
        .position(|f| f.name == name)
        .ok_or_else(|| WalkError {
            code: "CALL_TARGET_NOT_FOUND".to_string(),
            message: format!("call target '{name}' not found"),
            capability: Some(format!("call.{name}")),
            step_index: state.steps,
        })?;
    execute_function(
        functions,
        idx,
        state,
        locals,
        call_depth + 1,
        max_call_depth,
    )
}

fn invoke_match_target(
    functions: &[MirFunction],
    target: &MirMatchTarget,
    state: &mut WalkState<'_>,
    locals: &Map<String, JsonValue>,
    call_depth: usize,
    max_call_depth: usize,
) -> Result<(), WalkError> {
    match target {
        MirMatchTarget::Target(name) => {
            invoke_named(functions, name, state, locals, call_depth, max_call_depth)
        }
        MirMatchTarget::Nested {
            field,
            cases,
            default_target,
        } => {
            let observed = state
                .current
                .as_object()
                .and_then(|obj| obj.get(field))
                .cloned()
                .unwrap_or(JsonValue::Null);
            let nested = resolve_match_target(&observed, cases, default_target);
            invoke_match_target(
                functions,
                &nested,
                state,
                locals,
                call_depth,
                max_call_depth,
            )
        }
    }
}

fn resolve_match_target(
    observed: &JsonValue,
    cases: &[grapheme_artifact::MirMatchCase],
    default_target: &MirMatchTarget,
) -> MirMatchTarget {
    for case in cases {
        if &case.eq == observed {
            return case.then_target.clone();
        }
    }
    default_target.clone()
}

fn compare_values(observed: &JsonValue, cmp: &MirCompareOp, expected: &JsonValue) -> bool {
    match cmp {
        MirCompareOp::Eq => observed == expected,
        MirCompareOp::Gt => numbers(observed, expected).is_some_and(|(a, b)| a > b),
        MirCompareOp::Gte => numbers(observed, expected).is_some_and(|(a, b)| a >= b),
        MirCompareOp::Lt => numbers(observed, expected).is_some_and(|(a, b)| a < b),
        MirCompareOp::Lte => numbers(observed, expected).is_some_and(|(a, b)| a <= b),
    }
}

fn numbers(a: &JsonValue, b: &JsonValue) -> Option<(f64, f64)> {
    Some((a.as_f64()?, b.as_f64()?))
}

fn reduce_iteration_outputs(outputs: &[JsonValue]) -> JsonValue {
    let mut acc = Map::new();
    for item in outputs {
        if let JsonValue::Object(map) = item {
            for (k, v) in map {
                acc.insert(k.clone(), v.clone());
            }
        }
    }
    JsonValue::Object(acc)
}

fn build_locals(
    params: &[MirParam],
    call_args: &JsonValue,
    function_name: &str,
) -> Result<Map<String, JsonValue>, WalkError> {
    let arg_map = match call_args {
        JsonValue::Null => Map::new(),
        JsonValue::Object(map) => map.clone(),
        _ => {
            return Err(WalkError {
                code: "INVALID_CALL_ARGS".to_string(),
                message: format!("call args for '{function_name}' must be an object"),
                capability: None,
                step_index: 0,
            })
        }
    };

    let mut locals = Map::new();
    for param in params {
        if let Some(value) = arg_map.get(&param.name) {
            locals.insert(param.name.clone(), value.clone());
            continue;
        }
        if let Some(default) = &param.default {
            locals.insert(param.name.clone(), default.clone());
            continue;
        }
        if param.required {
            return Err(WalkError {
                code: "MISSING_PARAM".to_string(),
                message: format!(
                    "call '{function_name}' missing required parameter '{}'",
                    param.name
                ),
                capability: None,
                step_index: 0,
            });
        }
    }
    Ok(locals)
}

pub fn walk_result_to_json(result: &WalkResult) -> JsonValue {
    let mut out = json!({
        "ok": result.ok,
        "current": result.current,
        "steps": result.steps,
        "host_calls": result.host_calls,
    });
    if let Some(err) = &result.error {
        out["error"] = json!({
            "code": err.code,
            "message": err.message,
            "capability": err.capability,
            "step_index": err.step_index,
        });
    }
    out
}
