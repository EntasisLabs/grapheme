/// ─────────────────────────────────────────────────────────────
///  Grapheme  —  AST Node Definitions
///  Every node in the grammar maps to a typed Rust struct/enum.
///  These are pure data; no execution logic lives here.
/// ─────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

// ── Primitives ────────────────────────────────────────────────

/// A runtime value — literals, variables, or composites
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    String(String),
    Variable(String),          // $name
    Symbol(String),            // unquoted symbol target
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
}

/// A directive annotation: @name(args...)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directive {
    pub name: String,
    pub args: Vec<(String, Value)>,
}

// ── Type System ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeRef {
    Named(String, bool),       // name, non_null
    List(Box<TypeRef>, bool),  // inner, non_null
    Scalar(ScalarKind, bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalarKind {
    String,
    Int,
    Float,
    Bool,
    Any,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub type_ref: TypeRef,
    pub directives: Vec<Directive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub directives: Vec<Directive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDef {
    pub types: Vec<TypeDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructFieldDef {
    pub name: String,
    pub type_ref: TypeRef,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructFieldDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableSignature {
    pub input: TypeRef,
    pub output: Option<TypeRef>,
}

// ── Module System ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpKind {
    Query,
    Mutation,
}

/// A single operation declared inside a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpDef {
    pub kind: OpKind,
    pub name: String,
    pub args: Vec<(String, TypeRef)>,
    pub returns: TypeRef,
}

/// An AI-proposed module — submitted to the runtime for approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleProposal {
    pub name: String,
    pub ops: Vec<OpDef>,
}

/// A resolved import: import Foo from "grapheme/foo"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDecl {
    pub alias: String,
    pub path: String,
}

// ── Selection Sets ────────────────────────────────────────────

/// What subset of AgentState the caller wants inspected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateSelector {
    Current,
    Diff,
    Errors,
    Pipeline,
    Proposed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectedField {
    Spread(String),                         // ...FragmentName
    State(Vec<StateSelector>),              // state { current errors }
    Aliased(String, Box<FieldCall>),        // alias: field(...)
    Plain(FieldCall),
    Bare(String),                           // just a field name, no call
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionSet {
    pub fields: Vec<SelectedField>,
}

// ── Field Calls & Pipelines ───────────────────────────────────

/// A single step in a pipeline: [Module.]op(args) @directive { selection }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCall {
    pub module: Option<String>,            // Some("Database") or None
    pub name: String,
    pub args: Vec<(String, Value)>,
    pub directives: Vec<Directive>,
    pub selection: Option<SelectionSet>,
}

/// A direct fragment/call-target invocation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallStep {
    pub target: String,
    pub args: Vec<(String, Value)>,
    pub directives: Vec<Directive>,
    pub selection: Option<SelectionSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStep {
    Field(FieldCall),
    Call(CallStep),
}

/// A |>-chained sequence of executable steps.
/// AgentState threads through each step automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub steps: Vec<PipelineStep>,
}

// ── Top-Level Definitions ─────────────────────────────────────

/// Variable declared in the operation signature: $name: Type = default
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDef {
    pub name: String,
    pub type_ref: TypeRef,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDef {
    pub name: String,
    pub variables: Vec<VariableDef>,
    pub signature: Option<ExecutableSignature>,
    pub directives: Vec<Directive>,
    pub pipelines: Vec<Pipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationDef {
    pub name: String,
    pub variables: Vec<VariableDef>,
    pub signature: Option<ExecutableSignature>,
    pub directives: Vec<Directive>,
    pub pipelines: Vec<Pipeline>,
}

/// Fragment: a named, reusable pipeline — like a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentDef {
    pub name: String,
    pub signature: ExecutableSignature,
    pub directives: Vec<Directive>,
    pub pipelines: Vec<Pipeline>,
}

/// Subscription: event-driven reactive loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionDef {
    pub name: String,
    pub variables: Vec<VariableDef>,
    pub signature: Option<ExecutableSignature>,
    pub directives: Vec<Directive>,
    pub pipelines: Vec<Pipeline>,
}

// ── Program Root ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Definition {
    Query(QueryDef),
    Mutation(MutationDef),
    Fragment(FragmentDef),
    Subscription(SubscriptionDef),
    Struct(StructDef),
    Schema(SchemaDef),
    ModuleProposal(ModuleProposal),
}

/// The root of a parsed Grapheme program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub definitions: Vec<Definition>,
}
