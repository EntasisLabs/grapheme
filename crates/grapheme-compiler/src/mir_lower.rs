use grapheme_artifact::{MirBlock, MirFunction, MirFunctionKind, MirInst, MirLoopConfig, MirLoopUntil, MirProgram, MirTerminator};
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
    let max = args.get("max")?.as_u64()? as u32;

    let until = args.get("until").and_then(|value| {
        let object = value.as_object()?;
        let field = object.get("field")?.as_str()?.to_string();
        let eq = object.get("eq")?.clone();

        Some(MirLoopUntil { field, eq })
    });

    Some(MirLoopConfig { max, until })
}

fn lower_kind(kind: &HirExecutableKind) -> MirFunctionKind {
    match kind {
        HirExecutableKind::Query => MirFunctionKind::Query,
        HirExecutableKind::Mutation => MirFunctionKind::Mutation,
        HirExecutableKind::Subscription => MirFunctionKind::Subscription,
        HirExecutableKind::Fragment => MirFunctionKind::Fragment,
    }
}
