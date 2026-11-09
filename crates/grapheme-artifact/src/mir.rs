use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::capability::Capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirProgram {
    pub functions: Vec<MirFunction>,
    pub capabilities: Vec<Capability>,
    /// `state` stores declared by the program (RFC-0007 Part A).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stores: Vec<MirStore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirStore {
    pub name: String,
    pub fields: Vec<MirStoreField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirStoreFieldKind {
    Plain,
    Const,
    Transient,
    Derive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirStoreField {
    pub name: String,
    pub kind: MirStoreFieldKind,
    pub type_name: String,
    /// Default value expression for stored fields; computed expression for derives.
    pub value: JsonValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<MirLifecycle>,
}

/// The `state_machine` governing a lifecycle field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirLifecycle {
    pub machine: String,
    pub transitions: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terminals: Vec<String>,
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
    /// Store this function is declared `on`; its runs commit store revisions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,
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
