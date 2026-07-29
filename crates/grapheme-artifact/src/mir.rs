use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::capability::Capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirProgram {
    pub functions: Vec<MirFunction>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirParam {
    pub name: String,
    #[serde(default)]
    pub type_name: Option<String>,
    #[serde(default)]
    pub default: Option<JsonValue>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirFunction {
    pub name: String,
    pub kind: MirFunctionKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<MirParam>,
    #[serde(default)]
    pub retry_config: Option<MirRetryConfig>,
    #[serde(default)]
    pub timeout_config: Option<MirTimeoutConfig>,
    #[serde(default)]
    pub intent_config: Option<MirIntentConfig>,
    pub loop_config: Option<MirLoopConfig>,
    pub blocks: Vec<MirBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirIntentConfig {
    pub goal: Option<String>,
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirRetryConfig {
    pub max: u32,
    pub backoff_ms: Option<u32>,
    pub on_fail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirTimeoutConfig {
    pub ms: u32,
    pub on_timeout: String,
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
    MatchCall {
        field: String,
        cases: Vec<MirMatchCase>,
        default_target: MirMatchTarget,
        max_depth: Option<u32>,
    },
    UsingEnter {
        scope_id: u32,
        activations: Vec<MirTagActivation>,
    },
    UsingExit {
        scope_id: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirTagActivation {
    pub tag: String,
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub mutability: Option<String>,
    pub fields: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirMatchCase {
    pub eq: JsonValue,
    pub then_target: MirMatchTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirMatchTarget {
    Target(String),
    Nested {
        field: String,
        cases: Vec<MirMatchCase>,
        default_target: Box<MirMatchTarget>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirTerminator {
    ReturnState,
}
