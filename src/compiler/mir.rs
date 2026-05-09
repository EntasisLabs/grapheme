use serde::{Deserialize, Serialize};

use super::capability::Capability;
use super::hir::{HirExecutableKind, HirProgram};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirProgram {
    pub functions: Vec<MirFunction>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirFunction {
    pub name: String,
    pub kind: MirFunctionKind,
    pub blocks: Vec<MirBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirFunctionKind {
    Query,
    Mutation,
    Subscription,
    Fragment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirBlock {
    pub id: u32,
    pub instructions: Vec<MirInst>,
    pub terminator: MirTerminator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirInst {
    Call {
        capability: Capability,
        arg_count: u16,
        stores_state: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirTerminator {
    ReturnState,
}

pub fn lower_from_hir(hir: &HirProgram) -> MirProgram {
    let functions = hir
        .executable_defs
        .iter()
        .map(|def| {
            let mut instructions = Vec::new();

            for pipeline in &def.pipelines {
                for step in &pipeline.steps {
                    instructions.push(MirInst::Call {
                        capability: step.capability.clone(),
                        arg_count: step.arg_count as u16,
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
                blocks: vec![block],
            }
        })
        .collect();

    MirProgram {
        functions,
        capabilities: hir.capabilities.clone(),
    }
}

fn lower_kind(kind: &HirExecutableKind) -> MirFunctionKind {
    match kind {
        HirExecutableKind::Query => MirFunctionKind::Query,
        HirExecutableKind::Mutation => MirFunctionKind::Mutation,
        HirExecutableKind::Subscription => MirFunctionKind::Subscription,
        HirExecutableKind::Fragment => MirFunctionKind::Fragment,
    }
}
