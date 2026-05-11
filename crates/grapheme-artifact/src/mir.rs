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
    pub max: Option<u32>,
    pub each: Option<String>,
    pub until: Option<MirLoopUntil>,
    pub merge: MirLoopMergeMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirLoopMergeMode {
    Replace,
    Append,
    Reduce,
    None,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirCompareOp {
    #[default]
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
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
    BranchCall {
        field: String,
        #[serde(default)]
        cmp: MirCompareOp,
        #[serde(alias = "eq")]
        value: JsonValue,
        then_target: String,
        else_target: Option<String>,
        max_depth: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirTerminator {
    ReturnState,
}
