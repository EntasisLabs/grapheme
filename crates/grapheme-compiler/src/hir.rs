use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use grapheme_artifact::Capability;
use std::collections::HashSet;

use crate::ast::{Definition, Directive, OpKind, Pipeline, PipelineStep, Program, StructDef, TypeRef, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirProgram {
    pub imports: Vec<HirImport>,
    pub struct_defs: Vec<HirStructDef>,
    pub executable_defs: Vec<HirExecutable>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirStructDef {
    pub name: String,
    pub fields: Vec<HirStructField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirStructField {
    pub name: String,
    pub type_ref: TypeRef,
    pub optional: bool,
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
    pub input_type: Option<TypeRef>,
    pub output_type: Option<TypeRef>,
    pub loop_directive_count: usize,
    pub loop_args: Option<JsonValue>,
    pub recursive_directive_count: usize,
    pub recursive_args: Option<JsonValue>,
    pub recursive_max_depth: Option<u32>,
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
    let executable_names = program
        .definitions
        .iter()
        .filter_map(|def| match def {
            Definition::Query(q) => Some(q.name.clone()),
            Definition::Mutation(m) => Some(m.name.clone()),
            Definition::Subscription(s) => Some(s.name.clone()),
            Definition::Fragment(f) => Some(f.name.clone()),
            Definition::Struct(_) | Definition::Schema(_) | Definition::ModuleProposal(_) => None,
        })
        .collect::<HashSet<_>>();

    let struct_defs = program
        .definitions
        .iter()
        .filter_map(|def| match def {
            Definition::Struct(struct_def) => Some(lower_struct_def(struct_def)),
            _ => None,
        })
        .collect::<Vec<_>>();

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
                input_type: q.signature.as_ref().map(|sig| sig.input.clone()),
                output_type: q.signature.as_ref().and_then(|sig| sig.output.clone()),
                loop_directive_count: loop_directive_count(&q.directives),
                loop_args: first_loop_args(&q.directives),
                recursive_directive_count: recursive_directive_count(&q.directives),
                recursive_args: first_recursive_args(&q.directives),
                recursive_max_depth: first_recursive_max_depth(first_recursive_args(&q.directives).as_ref()),
                pipelines: lower_pipelines(
                    &q.name,
                    &q.pipelines,
                    &executable_names,
                    first_recursive_max_depth(first_recursive_args(&q.directives).as_ref()),
                ),
            }),
            Definition::Mutation(m) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Mutation,
                name: m.name.clone(),
                input_type: m.signature.as_ref().map(|sig| sig.input.clone()),
                output_type: m.signature.as_ref().and_then(|sig| sig.output.clone()),
                loop_directive_count: loop_directive_count(&m.directives),
                loop_args: first_loop_args(&m.directives),
                recursive_directive_count: recursive_directive_count(&m.directives),
                recursive_args: first_recursive_args(&m.directives),
                recursive_max_depth: first_recursive_max_depth(first_recursive_args(&m.directives).as_ref()),
                pipelines: lower_pipelines(
                    &m.name,
                    &m.pipelines,
                    &executable_names,
                    first_recursive_max_depth(first_recursive_args(&m.directives).as_ref()),
                ),
            }),
            Definition::Subscription(s) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Subscription,
                name: s.name.clone(),
                input_type: s.signature.as_ref().map(|sig| sig.input.clone()),
                output_type: s.signature.as_ref().and_then(|sig| sig.output.clone()),
                loop_directive_count: loop_directive_count(&s.directives),
                loop_args: first_loop_args(&s.directives),
                recursive_directive_count: recursive_directive_count(&s.directives),
                recursive_args: first_recursive_args(&s.directives),
                recursive_max_depth: first_recursive_max_depth(first_recursive_args(&s.directives).as_ref()),
                pipelines: lower_pipelines(
                    &s.name,
                    &s.pipelines,
                    &executable_names,
                    first_recursive_max_depth(first_recursive_args(&s.directives).as_ref()),
                ),
            }),
            Definition::Fragment(f) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Fragment,
                name: f.name.clone(),
                input_type: Some(f.signature.input.clone()),
                output_type: f.signature.output.clone(),
                loop_directive_count: loop_directive_count(&f.directives),
                loop_args: first_loop_args(&f.directives),
                recursive_directive_count: recursive_directive_count(&f.directives),
                recursive_args: first_recursive_args(&f.directives),
                recursive_max_depth: first_recursive_max_depth(first_recursive_args(&f.directives).as_ref()),
                pipelines: lower_pipelines(
                    &f.name,
                    &f.pipelines,
                    &executable_names,
                    first_recursive_max_depth(first_recursive_args(&f.directives).as_ref()),
                ),
            }),
            Definition::Struct(_) | Definition::Schema(_) | Definition::ModuleProposal(_) => {}
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
        struct_defs,
        executable_defs,
        capabilities,
    }
}

fn lower_struct_def(struct_def: &StructDef) -> HirStructDef {
    HirStructDef {
        name: struct_def.name.clone(),
        fields: struct_def
            .fields
            .iter()
            .map(|field| HirStructField {
                name: field.name.clone(),
                type_ref: field.type_ref.clone(),
                optional: field.optional,
            })
            .collect(),
    }
}

fn lower_pipelines(
    executable_name: &str,
    pipelines: &[Pipeline],
    executable_names: &HashSet<String>,
    recursive_max_depth: Option<u32>,
) -> Vec<HirPipeline> {
    pipelines
        .iter()
        .map(|p| HirPipeline {
            steps: p
                .steps
                .iter()
                .map(|step| {
                    lower_step(
                        step,
                        executable_name,
                        executable_names,
                        recursive_max_depth,
                    )
                })
                .collect(),
        })
        .collect()
}

fn lower_step(
    step: &PipelineStep,
    executable_name: &str,
    executable_names: &HashSet<String>,
    recursive_max_depth: Option<u32>,
) -> HirStep {
    match step {
        PipelineStep::Field(field) => {
            if field.module.is_none() && executable_names.contains(&field.name) {
                let mut args = lower_args(&field.args);
                maybe_inject_recursive_max_depth(
                    &mut args,
                    executable_name,
                    &field.name,
                    recursive_max_depth,
                );
                return HirStep {
                    op: field.name.clone(),
                    module: Some("call".to_string()),
                    arg_count: field.args.len(),
                    args,
                    has_selection: field.selection.is_some(),
                    capability: Capability::from_module_op("call", &field.name),
                };
            }

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
        PipelineStep::Call(call) => {
            let mut args = lower_args(&call.args);
            maybe_inject_recursive_max_depth(
                &mut args,
                executable_name,
                &call.target,
                recursive_max_depth,
            );
            HirStep {
                op: call.target.clone(),
                module: Some("call".to_string()),
                arg_count: call.args.len(),
                args,
                has_selection: call.selection.is_some(),
                capability: Capability::from_module_op("call", &call.target),
            }
        }
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

fn recursive_directive_count(directives: &[Directive]) -> usize {
    directives.iter().filter(|d| d.name == "recursive").count()
}

fn first_recursive_args(directives: &[Directive]) -> Option<JsonValue> {
    directives
        .iter()
        .find(|d| d.name == "recursive")
        .map(|d| lower_args(&d.args))
}

fn first_recursive_max_depth(recursive_args: Option<&JsonValue>) -> Option<u32> {
    recursive_args
        .and_then(|args| args.as_object())
        .and_then(|args| args.get("max_depth"))
        .and_then(|value| value.as_i64())
        .and_then(|value| if value >= 1 { Some(value as u32) } else { None })
}

fn maybe_inject_recursive_max_depth(
    args: &mut JsonValue,
    executable_name: &str,
    call_target: &str,
    recursive_max_depth: Option<u32>,
) {
    if executable_name != call_target {
        return;
    }

    let Some(max_depth) = recursive_max_depth else {
        return;
    };

    let Some(args_map) = args.as_object_mut() else {
        return;
    };

    if args_map.contains_key("max_depth") {
        return;
    }

    args_map.insert("max_depth".to_string(), JsonValue::from(max_depth));
}

fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Int(v) => JsonValue::from(*v),
        Value::Float(v) => JsonValue::from(*v),
        Value::Bool(v) => JsonValue::from(*v),
        Value::Null => JsonValue::Null,
        Value::String(v) => JsonValue::from(v.clone()),
        Value::Variable(name) => {
            let mut object = Map::new();
            object.insert("$var".to_string(), JsonValue::from(name.clone()));
            JsonValue::Object(object)
        }
        Value::Symbol(name) => {
            let mut object = Map::new();
            object.insert("$symbol".to_string(), JsonValue::from(name.clone()));
            JsonValue::Object(object)
        }
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
