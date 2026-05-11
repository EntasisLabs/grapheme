use grapheme_artifact::{
    MirBlock, MirFunction, MirFunctionKind, MirInst, MirLoopConfig, MirLoopMergeMode,
    MirLoopUntil, MirMatchCase, MirMatchTarget, MirProgram, MirRetryConfig,
    MirTerminator, MirTimeoutConfig,
};
use grapheme_artifact::mir::MirCompareOp;
use serde_json::Value as JsonValue;

use crate::hir::{HirExecutableKind, HirProgram};

pub fn lower_from_hir(hir: &HirProgram) -> MirProgram {
    let functions = hir
        .executable_defs
        .iter()
        .map(|def| {
            let mut instructions = Vec::new();

            for pipeline in &def.pipelines {
                for step in &pipeline.steps {
                    if let Some(branch_inst) = lower_flow_branch(step, def.recursive_max_depth) {
                        instructions.push(branch_inst);
                        continue;
                    }

                    if let Some(match_inst) = lower_flow_match(step, def.recursive_max_depth) {
                        instructions.push(match_inst);
                        continue;
                    }

                    instructions.push(MirInst::Call {
                        module: step.module.clone(),
                        op: step.op.clone(),
                        capability: step.capability.clone(),
                        arg_count: step.arg_count as u16,
                        args: step.args.clone(),
                        stores_state: true,
                    });
                }
            }

            let block = MirBlock {
                id: 0,
                instructions,
                terminator: MirTerminator::ReturnState,
            };

            MirFunction {
                name: def.name.clone(),
                kind: lower_kind(&def.kind),
                retry_config: lower_retry_config(def.retry_args.as_ref()),
                timeout_config: lower_timeout_config(def.timeout_args.as_ref()),
                loop_config: lower_loop_config(def.loop_args.as_ref()),
                blocks: vec![block],
            }
        })
        .collect();

    MirProgram {
        functions,
        capabilities: hir.capabilities.clone(),
    }
}

fn lower_loop_config(loop_args: Option<&JsonValue>) -> Option<MirLoopConfig> {
    let args = loop_args?.as_object()?;
    let max = args
        .get("max")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);
    let each = args
        .get("each")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    let until = args.get("until").and_then(|value| {
        let object = value.as_object()?;
        let field = object.get("field")?.as_str()?.to_string();
        let eq = object.get("eq")?.clone();

        Some(MirLoopUntil { field, eq })
    });

    Some(MirLoopConfig {
        max,
        each,
        until,
        merge: lower_loop_merge_mode(args.get("merge")),
    })
}

fn lower_retry_config(retry_args: Option<&JsonValue>) -> Option<MirRetryConfig> {
    let args = retry_args?.as_object()?;
    let max = args.get("max")?.as_u64()? as u32;
    let on_fail = branch_target_from_value(args.get("on_fail")?)?;
    let backoff_ms = args
        .get("backoff_ms")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);

    Some(MirRetryConfig {
        max,
        backoff_ms,
        on_fail,
    })
}

fn lower_timeout_config(timeout_args: Option<&JsonValue>) -> Option<MirTimeoutConfig> {
    let args = timeout_args?.as_object()?;
    let ms = args.get("ms")?.as_u64()? as u32;
    let on_timeout = branch_target_from_value(args.get("on_timeout")?)?;

    Some(MirTimeoutConfig { ms, on_timeout })
}

fn lower_loop_merge_mode(value: Option<&JsonValue>) -> MirLoopMergeMode {
    match value.and_then(|v| v.as_str()) {
        Some("append") => MirLoopMergeMode::Append,
        Some("reduce") => MirLoopMergeMode::Reduce,
        Some("none") => MirLoopMergeMode::None,
        _ => MirLoopMergeMode::Replace,
    }
}

fn lower_flow_branch(step: &crate::hir::HirStep, recursive_max_depth: Option<u32>) -> Option<MirInst> {
    let module = step.module.as_deref()?;
    if !module.eq_ignore_ascii_case("flow") || step.op != "branch" {
        return None;
    }

    let args = step.args.as_object()?;
    let when = args.get("when")?.as_object()?;
    let (field, cmp, value) = lower_flow_branch_when(when)?;
    let then_target = branch_target_from_value(args.get("then")?)?;
    let else_target = args
        .get("else")
        .and_then(branch_target_from_value);

    let max_depth = args
        .get("max_depth")
        .and_then(|v| v.as_i64())
        .and_then(|v| if v >= 1 { Some(v as u32) } else { None })
        .or(recursive_max_depth);

    Some(MirInst::BranchCall {
        field,
        cmp,
        value,
        then_target,
        else_target,
        max_depth,
    })
}

fn lower_flow_match(step: &crate::hir::HirStep, recursive_max_depth: Option<u32>) -> Option<MirInst> {
    let module = step.module.as_deref()?;
    if !module.eq_ignore_ascii_case("flow") || step.op != "match" {
        return None;
    }

    let args = step.args.as_object()?;
    let field = args.get("field")?.as_str()?.to_string();
    let cases = args
        .get("cases")?
        .as_array()?
        .iter()
        .map(lower_match_case)
        .collect::<Option<Vec<_>>>()?;
    let default_target = lower_match_target(args.get("default")?)?;

    let max_depth = args
        .get("max_depth")
        .and_then(|v| v.as_i64())
        .and_then(|v| if v >= 1 { Some(v as u32) } else { None })
        .or(recursive_max_depth);

    Some(MirInst::MatchCall {
        field,
        cases,
        default_target,
        max_depth,
    })
}

fn lower_match_case(value: &JsonValue) -> Option<MirMatchCase> {
    let object = value.as_object()?;
    let eq = normalize_compare_value(object.get("eq")?);
    let then_target = lower_match_target(object.get("then")?)?;
    Some(MirMatchCase { eq, then_target })
}

fn lower_match_target(value: &JsonValue) -> Option<MirMatchTarget> {
    if let Some(target) = branch_target_from_value(value) {
        return Some(MirMatchTarget::Target(target));
    }

    let object = value.as_object()?;
    let nested = object.get("$match")?.as_object()?;
    let field = nested.get("field")?.as_str()?.to_string();
    let cases = nested
        .get("cases")?
        .as_array()?
        .iter()
        .map(lower_match_case)
        .collect::<Option<Vec<_>>>()?;
    let default_target = Box::new(lower_match_target(nested.get("default")?)?);

    Some(MirMatchTarget::Nested {
        field,
        cases,
        default_target,
    })
}

fn lower_flow_branch_when(when: &serde_json::Map<String, JsonValue>) -> Option<(String, MirCompareOp, JsonValue)> {
    let field = when.get("field")?.as_str()?.to_string();

    if let Some(value) = when.get("eq") {
        return Some((field, MirCompareOp::Eq, normalize_compare_value(value)));
    }
    if let Some(value) = when.get("gt") {
        return Some((field, MirCompareOp::Gt, normalize_compare_value(value)));
    }
    if let Some(value) = when.get("gte") {
        return Some((field, MirCompareOp::Gte, normalize_compare_value(value)));
    }
    if let Some(value) = when.get("lt") {
        return Some((field, MirCompareOp::Lt, normalize_compare_value(value)));
    }
    if let Some(value) = when.get("lte") {
        return Some((field, MirCompareOp::Lte, normalize_compare_value(value)));
    }

    None
}

fn normalize_compare_value(value: &JsonValue) -> JsonValue {
    if let Some(symbol) = value
        .as_object()
        .and_then(|obj| obj.get("$symbol"))
        .and_then(|v| v.as_str())
    {
        return JsonValue::String(symbol.to_string());
    }

    value.clone()
}

fn branch_target_from_value(value: &JsonValue) -> Option<String> {
    if let Some(target) = value.as_str() {
        return Some(target.to_string());
    }

    let object = value.as_object()?;
    let symbol = object.get("$symbol")?.as_str()?;
    if symbol == "return" {
        return Some("$return".to_string());
    }

    Some(symbol.to_string())
}

fn lower_kind(kind: &HirExecutableKind) -> MirFunctionKind {
    match kind {
        HirExecutableKind::Query => MirFunctionKind::Query,
        HirExecutableKind::Mutation => MirFunctionKind::Mutation,
        HirExecutableKind::Subscription => MirFunctionKind::Subscription,
        HirExecutableKind::Fragment => MirFunctionKind::Fragment,
    }
}
