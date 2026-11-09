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
    Variable(String), // $name
    Symbol(String),   // unquoted symbol target
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
    Expr(Box<ValueExpr>), // pure expression over values
}

/// A pure value expression. Evaluated at runtime against the current scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueExpr {
    Unary(UnaryOp, Value),
    Binary(BinaryOp, Value, Value),
    If(Value, Value, Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl UnaryOp {
    pub fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "neg",
        }
    }
}

impl BinaryOp {
    pub fn from_token(token: &str) -> Option<Self> {
        Some(match token {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "%" => BinaryOp::Rem,
            "==" => BinaryOp::Eq,
            "!=" => BinaryOp::Ne,
            "<" => BinaryOp::Lt,
            "<=" => BinaryOp::Le,
            ">" => BinaryOp::Gt,
            ">=" => BinaryOp::Ge,
            "&&" => BinaryOp::And,
            "||" => BinaryOp::Or,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        }
    }
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
    Named(String, bool),      // name, non_null
    List(Box<TypeRef>, bool), // inner, non_null
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

/// How a store field is written and retained (RFC-0007 A.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreFieldKind {
    Plain,
    Const,
    Transient,
    Derive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreFieldDef {
    pub name: String,
    pub kind: StoreFieldKind,
    pub type_ref: TypeRef,
    /// Default for stored fields; the computed expression for derives.
    pub value: Value,
}

/// `state Name { ... }`: a typed, revisioned state store (RFC-0007 Part A).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDef {
    pub name: String,
    pub lifetime: Option<String>,
    pub owner: Option<String>,
    pub fields: Vec<StoreFieldDef>,
}

/// `view Name [on Store] { Element }`: a declarative element tree (RFC-0007 Parts C, D).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewDef {
    pub name: String,
    pub store: Option<String>,
    pub root: ViewElement,
}

/// One element: a catalog primitive or a reference to another view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewElement {
    /// `name: Element` names the child; its node key uses the name.
    pub child_name: Option<String>,
    pub kind: String,
    /// Positional content: `Text("{$count}")`.
    pub content: Option<Value>,
    pub props: Vec<(String, Value)>,
    pub events: Vec<ViewEvent>,
    pub children: Vec<ViewElement>,
}

/// `on click: Select(index: 3)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewEvent {
    pub event: String,
    pub target: String,
    pub args: Vec<(String, Value)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionDef {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineDef {
    pub name: String,
    pub enum_name: String,
    pub terminals: Vec<String>,
    pub transitions: Vec<StateTransitionDef>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportKind {
    Module,
    Types,
}

/// A resolved import: import Foo from "grapheme/foo"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDecl {
    pub kind: ImportKind,
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
    Spread(String),                  // ...FragmentName
    State(Vec<StateSelector>),       // state { current errors }
    Aliased(String, Box<FieldCall>), // alias: field(...)
    Plain(FieldCall),
    Bare(String), // just a field name, no call
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionSet {
    pub fields: Vec<SelectedField>,
}

// ── Field Calls & Pipelines ───────────────────────────────────

/// A single step in a pipeline: [Module.]op(args) @directive { selection }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCall {
    pub module: Option<String>, // Some("Database") or None
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
pub struct StructInitStep {
    pub type_name: String,
    pub fields: Vec<(String, Value)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsingMutability {
    Const,
    Mutable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsingBinding {
    pub mutability: Option<UsingMutability>,
    pub handle: Option<String>,
    pub tag: String,
    pub fields: Vec<(String, Value)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsingStep {
    pub bindings: Vec<UsingBinding>,
    pub pipelines: Vec<Pipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDef {
    pub name: String,
    pub allow_list: Vec<String>,
    pub variables: Vec<VariableDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsingDecl {
    pub mutability: UsingMutability,
    pub handle: String,
    pub tag: String,
    pub fields: Vec<(String, Value)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStep {
    Field(FieldCall),
    Call(CallStep),
    StructInit(StructInitStep),
    Using(UsingStep),
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

/// Glyph: a workflow composition root, or a UI app on a store (RFC-0007 Part H).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphDef {
    pub name: String,
    /// When set, this is a UI glyph: the body is [`root`], not [`pipelines`].
    pub store: Option<String>,
    /// `expose Name, …` — store executables and views reachable outside the tree.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<String>,
    /// Root element of a UI glyph (`on Store`). `None` for workflow glyphs.
    pub root: Option<ViewElement>,
    /// Pipelines of a workflow glyph. Empty for UI glyphs.
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

/// Iterator: a named, reusable executable pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IteratorDef {
    pub name: String,
    pub variables: Vec<VariableDef>,
    pub signature: ExecutableSignature,
    pub directives: Vec<Directive>,
    pub pipelines: Vec<Pipeline>,
}

/// Fragment: a named, reusable inline pipeline (non-executable in Phase A).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentDef {
    pub name: String,
    pub signature: ExecutableSignature,
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
    Glyph(GlyphDef),
    Query(QueryDef),
    Mutation(MutationDef),
    Iterator(IteratorDef),
    Fragment(FragmentDef),
    Subscription(SubscriptionDef),
    Struct(StructDef),
    Enum(EnumDef),
    StateMachine(StateMachineDef),
    Store(StoreDef),
    View(ViewDef),
    Tag(TagDef),
    Using(UsingDecl),
    Schema(SchemaDef),
    ModuleProposal(ModuleProposal),
}

/// The root of a parsed Grapheme program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub definitions: Vec<Definition>,
}
