use grapheme_artifact::{
    MirBlock, MirFunction, MirFunctionKind, MirInst, MirLoopConfig, MirLoopMergeMode,
    MirLoopUntil, MirProgram, MirTerminator,
};
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
    let field = when.get("field")?.as_str()?.to_string();
    let eq = when.get("eq")?.clone();
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
        eq,
        then_target,
        else_target,
        max_depth,
    })
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
