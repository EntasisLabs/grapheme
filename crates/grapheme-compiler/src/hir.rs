use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use grapheme_artifact::Capability;

use crate::ast::{Definition, Directive, OpKind, Pipeline, PipelineStep, Program, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirProgram {
    pub imports: Vec<HirImport>,
    pub executable_defs: Vec<HirExecutable>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirImport {
    pub alias: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirExecutable {
    pub kind: HirExecutableKind,
    pub name: String,
    pub loop_directive_count: usize,
    pub loop_args: Option<JsonValue>,
    pub pipelines: Vec<HirPipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HirExecutableKind {
    Query,
    Mutation,
    Subscription,
    Fragment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirPipeline {
    pub steps: Vec<HirStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirStep {
    pub op: String,
    pub module: Option<String>,
    pub arg_count: usize,
    pub args: JsonValue,
    pub has_selection: bool,
    pub capability: Capability,
}

pub fn lower_from_ast(program: &Program) -> HirProgram {
    let imports = program
        .imports
        .iter()
        .map(|i| HirImport {
            alias: i.alias.clone(),
            path: i.path.clone(),
        })
        .collect::<Vec<_>>();

    let mut executable_defs = Vec::new();

    for def in &program.definitions {
        match def {
            Definition::Query(q) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Query,
                name: q.name.clone(),
                loop_directive_count: loop_directive_count(&q.directives),
                loop_args: first_loop_args(&q.directives),
                pipelines: lower_pipelines(&q.pipelines),
            }),
            Definition::Mutation(m) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Mutation,
                name: m.name.clone(),
                loop_directive_count: loop_directive_count(&m.directives),
                loop_args: first_loop_args(&m.directives),
                pipelines: lower_pipelines(&m.pipelines),
            }),
            Definition::Subscription(s) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Subscription,
                name: s.name.clone(),
                loop_directive_count: loop_directive_count(&s.directives),
                loop_args: first_loop_args(&s.directives),
                pipelines: lower_pipelines(&s.pipelines),
            }),
            Definition::Fragment(f) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Fragment,
                name: f.name.clone(),
                loop_directive_count: loop_directive_count(&f.directives),
                loop_args: first_loop_args(&f.directives),
                pipelines: lower_pipelines(&f.pipelines),
            }),
            Definition::Schema(_) | Definition::ModuleProposal(_) => {}
        }
    }

    let mut capabilities = executable_defs
        .iter()
        .flat_map(|d| d.pipelines.iter())
        .flat_map(|p| p.steps.iter())
        .map(|s| s.capability.clone())
        .collect::<Vec<_>>();

    capabilities.sort();
    capabilities.dedup();

    HirProgram {
        imports,
        executable_defs,
        capabilities,
    }
}

fn lower_pipelines(pipelines: &[Pipeline]) -> Vec<HirPipeline> {
    pipelines
        .iter()
        .map(|p| HirPipeline {
            steps: p.steps.iter().map(lower_step).collect(),
        })
        .collect()
}

fn lower_step(step: &PipelineStep) -> HirStep {
    match step {
        PipelineStep::Field(field) => {
            let capability = match &field.module {
                Some(module) => Capability::from_module_op(module, &field.name),
                None => Capability::from_bare_op(&field.name),
            };

            HirStep {
                op: field.name.clone(),
                module: field.module.clone(),
                arg_count: field.args.len(),
                args: lower_args(&field.args),
                has_selection: field.selection.is_some(),
                capability,
            }
        }
        PipelineStep::Call(call) => HirStep {
            op: call.target.clone(),
            module: Some("call".to_string()),
            arg_count: call.args.len(),
            args: lower_args(&call.args),
            has_selection: call.selection.is_some(),
            capability: Capability::from_module_op("call", &call.target),
        },
    }
}

fn lower_args(args: &[(String, Value)]) -> JsonValue {
    let mut object = Map::new();
    for (k, v) in args {
        object.insert(k.clone(), value_to_json(v));
    }
    JsonValue::Object(object)
}

fn loop_directive_count(directives: &[Directive]) -> usize {
    directives.iter().filter(|d| d.name == "loop").count()
}

fn first_loop_args(directives: &[Directive]) -> Option<JsonValue> {
    directives
        .iter()
        .find(|d| d.name == "loop")
        .map(|d| lower_args(&d.args))
}

fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Int(v) => JsonValue::from(*v),
        Value::Float(v) => JsonValue::from(*v),
        Value::Bool(v) => JsonValue::from(*v),
        Value::Null => JsonValue::Null,
        Value::String(v) => JsonValue::from(v.clone()),
        Value::Variable(name) => JsonValue::from(format!("${name}")),
        Value::List(items) => JsonValue::Array(items.iter().map(value_to_json).collect()),
        Value::Object(fields) => {
            let mut object = Map::new();
            for (k, v) in fields {
                object.insert(k.clone(), value_to_json(v));
            }
            JsonValue::Object(object)
        }
    }
}

pub fn op_kind_to_str(kind: &OpKind) -> &'static str {
    match kind {
        OpKind::Query => "query",
        OpKind::Mutation => "mutation",
    }
}
