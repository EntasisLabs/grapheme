use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::capability::Capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirProgram {
    pub functions: Vec<MirFunction>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirFunction {
    pub name: String,
    pub kind: MirFunctionKind,
    pub loop_config: Option<MirLoopConfig>,
    pub blocks: Vec<MirBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirLoopConfig {
    pub max: u32,
    pub until: Option<MirLoopUntil>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirLoopUntil {
    pub field: String,
    pub eq: JsonValue,
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
        module: Option<String>,
        op: String,
        capability: Capability,
        arg_count: u16,
        args: JsonValue,
        stores_state: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirTerminator {
    ReturnState,
}
