use grapheme_artifact::Capability;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::collections::{HashMap, HashSet};

use crate::ast::ImportKind;
use crate::ast::{
    Definition, Directive, FragmentDef, OpKind, Pipeline, PipelineStep, Program, StructDef,
    TypeRef, Value, VariableDef,
};
use crate::error::GraphemeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirProgram {
    pub imports: Vec<HirImport>,
    pub struct_defs: Vec<HirStructDef>,
    pub enum_defs: Vec<HirEnumDef>,
    pub state_machines: Vec<HirStateMachineDef>,
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
pub struct HirEnumDef {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirStateTransitionDef {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirStateMachineDef {
    pub name: String,
    pub enum_name: String,
    pub terminals: Vec<String>,
    pub transitions: Vec<HirStateTransitionDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirImport {
    pub kind: ImportKind,
    pub alias: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirParam {
    pub name: String,
    pub type_ref: TypeRef,
    pub default: Option<JsonValue>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirExecutable {
    pub kind: HirExecutableKind,
    pub name: String,
    pub params: Vec<HirParam>,
    pub input_type: Option<TypeRef>,
    pub output_type: Option<TypeRef>,
    pub loop_directive_count: usize,
    pub loop_args: Option<JsonValue>,
    pub recursive_directive_count: usize,
    pub recursive_args: Option<JsonValue>,
    pub recursive_max_depth: Option<u32>,
    pub retry_directive_count: usize,
    pub retry_args: Option<JsonValue>,
    pub timeout_directive_count: usize,
    pub timeout_args: Option<JsonValue>,
    pub intent_args: Option<JsonValue>,
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

pub fn lower_from_ast(program: &Program) -> Result<HirProgram, GraphemeError> {
    let fragment_defs = program
        .definitions
        .iter()
        .filter_map(|def| match def {
            Definition::Fragment(fragment) => Some((fragment.name.clone(), fragment)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    let executable_names = program
        .definitions
        .iter()
        .filter_map(|def| match def {
            Definition::Glyph(g) => Some(g.name.clone()),
            Definition::Query(q) => Some(q.name.clone()),
            Definition::Mutation(m) => Some(m.name.clone()),
            Definition::Subscription(s) => Some(s.name.clone()),
            Definition::Iterator(f) => Some(f.name.clone()),
            Definition::Fragment(_)
            | Definition::Struct(_)
            | Definition::Enum(_)
            | Definition::StateMachine(_)
            | Definition::Schema(_)
            | Definition::ModuleProposal(_) => None,
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

    let enum_defs = program
        .definitions
        .iter()
        .filter_map(|def| match def {
            Definition::Enum(enum_def) => Some(HirEnumDef {
                name: enum_def.name.clone(),
                members: enum_def.members.clone(),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();

    let state_machines = program
        .definitions
        .iter()
        .filter_map(|def| match def {
            Definition::StateMachine(sm) => Some(HirStateMachineDef {
                name: sm.name.clone(),
                enum_name: sm.enum_name.clone(),
                terminals: sm.terminals.clone(),
                transitions: sm
                    .transitions
                    .iter()
                    .map(|t| HirStateTransitionDef {
                        from: t.from.clone(),
                        to: t.to.clone(),
                    })
                    .collect(),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();

    let imports = program
        .imports
        .iter()
        .map(|i| HirImport {
            kind: i.kind.clone(),
            alias: i.alias.clone(),
            path: i.path.clone(),
        })
        .collect::<Vec<_>>();

    let mut executable_defs = Vec::new();

    for def in &program.definitions {
        match def {
            Definition::Glyph(g) => executable_defs.push(HirExecutable {
                kind: HirExecutableKind::Query,
                name: g.name.clone(),
                params: Vec::new(),
                input_type: None,
                output_type: None,
                loop_directive_count: 0,
                loop_args: None,
                recursive_directive_count: 0,
                recursive_args: None,
                recursive_max_depth: None,
                retry_directive_count: 0,
                retry_args: None,
                timeout_directive_count: 0,
                timeout_args: None,
                intent_args: None,
                pipelines: lower_pipelines(
                    &g.name,
                    &g.pipelines,
                    &executable_names,
                    None,
                    &fragment_defs,
                    false,
                )?,
            }),
            Definition::Query(q) => {
                let directives = normalize_executable_directives(&q.name, &q.directives)?;
                executable_defs.push(HirExecutable {
                    kind: HirExecutableKind::Query,
                    name: q.name.clone(),
                    params: lower_params(&q.variables),
                    input_type: q.signature.as_ref().map(|sig| sig.input.clone()),
                    output_type: q.signature.as_ref().and_then(|sig| sig.output.clone()),
                    loop_directive_count: loop_directive_count(&directives),
                    loop_args: first_loop_args(&directives),
                    recursive_directive_count: recursive_directive_count(&directives),
                    recursive_args: first_recursive_args(&directives),
                    recursive_max_depth: first_recursive_max_depth(
                        first_recursive_args(&directives).as_ref(),
                    ),
                    retry_directive_count: retry_directive_count(&directives),
                    retry_args: first_retry_args(&directives),
                    timeout_directive_count: timeout_directive_count(&directives),
                    timeout_args: first_timeout_args(&directives),
                    intent_args: first_intent_args(&directives),
                    pipelines: lower_pipelines(
                        &q.name,
                        &q.pipelines,
                        &executable_names,
                        first_recursive_max_depth(first_recursive_args(&directives).as_ref()),
                        &fragment_defs,
                        has_core_default_directive(&directives),
                    )?,
                })
            }
            Definition::Mutation(m) => {
                let directives = normalize_executable_directives(&m.name, &m.directives)?;
                executable_defs.push(HirExecutable {
                    kind: HirExecutableKind::Mutation,
                    name: m.name.clone(),
                    params: lower_params(&m.variables),
                    input_type: m.signature.as_ref().map(|sig| sig.input.clone()),
                    output_type: m.signature.as_ref().and_then(|sig| sig.output.clone()),
                    loop_directive_count: loop_directive_count(&directives),
                    loop_args: first_loop_args(&directives),
                    recursive_directive_count: recursive_directive_count(&directives),
                    recursive_args: first_recursive_args(&directives),
                    recursive_max_depth: first_recursive_max_depth(
                        first_recursive_args(&directives).as_ref(),
                    ),
                    retry_directive_count: retry_directive_count(&directives),
                    retry_args: first_retry_args(&directives),
                    timeout_directive_count: timeout_directive_count(&directives),
                    timeout_args: first_timeout_args(&directives),
                    intent_args: first_intent_args(&directives),
                    pipelines: lower_pipelines(
                        &m.name,
                        &m.pipelines,
                        &executable_names,
                        first_recursive_max_depth(first_recursive_args(&directives).as_ref()),
                        &fragment_defs,
                        has_core_default_directive(&directives),
                    )?,
                })
            }
            Definition::Subscription(s) => {
                let directives = normalize_executable_directives(&s.name, &s.directives)?;
                executable_defs.push(HirExecutable {
                    kind: HirExecutableKind::Subscription,
                    name: s.name.clone(),
                    params: lower_params(&s.variables),
                    input_type: s.signature.as_ref().map(|sig| sig.input.clone()),
                    output_type: s.signature.as_ref().and_then(|sig| sig.output.clone()),
                    loop_directive_count: loop_directive_count(&directives),
                    loop_args: first_loop_args(&directives),
                    recursive_directive_count: recursive_directive_count(&directives),
                    recursive_args: first_recursive_args(&directives),
                    recursive_max_depth: first_recursive_max_depth(
                        first_recursive_args(&directives).as_ref(),
                    ),
                    retry_directive_count: retry_directive_count(&directives),
                    retry_args: first_retry_args(&directives),
                    timeout_directive_count: timeout_directive_count(&directives),
                    timeout_args: first_timeout_args(&directives),
                    intent_args: first_intent_args(&directives),
                    pipelines: lower_pipelines(
                        &s.name,
                        &s.pipelines,
                        &executable_names,
                        first_recursive_max_depth(first_recursive_args(&directives).as_ref()),
                        &fragment_defs,
                        has_core_default_directive(&directives),
                    )?,
                })
            }
            Definition::Iterator(f) => {
                let directives = normalize_executable_directives(&f.name, &f.directives)?;
                executable_defs.push(HirExecutable {
                    kind: HirExecutableKind::Fragment,
                    name: f.name.clone(),
                    params: lower_params(&f.variables),
                    input_type: Some(f.signature.input.clone()),
                    output_type: f.signature.output.clone(),
                    loop_directive_count: loop_directive_count(&directives),
                    loop_args: first_loop_args(&directives),
                    recursive_directive_count: recursive_directive_count(&directives),
                    recursive_args: first_recursive_args(&directives),
                    recursive_max_depth: first_recursive_max_depth(
                        first_recursive_args(&directives).as_ref(),
                    ),
                    retry_directive_count: retry_directive_count(&directives),
                    retry_args: first_retry_args(&directives),
                    timeout_directive_count: timeout_directive_count(&directives),
                    timeout_args: first_timeout_args(&directives),
                    intent_args: first_intent_args(&directives),
                    pipelines: lower_pipelines(
                        &f.name,
                        &f.pipelines,
                        &executable_names,
                        first_recursive_max_depth(first_recursive_args(&directives).as_ref()),
                        &fragment_defs,
                        has_core_default_directive(&directives),
                    )?,
                })
            }
            Definition::Fragment(_)
            | Definition::Struct(_)
            | Definition::Enum(_)
            | Definition::StateMachine(_)
            | Definition::Schema(_)
            | Definition::ModuleProposal(_) => {}
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

    Ok(HirProgram {
        imports,
        struct_defs,
        enum_defs,
        state_machines,
        executable_defs,
        capabilities,
    })
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
    fragment_defs: &HashMap<String, &FragmentDef>,
    default_core_module: bool,
) -> Result<Vec<HirPipeline>, GraphemeError> {
    let mut expansion_stack = Vec::new();
    pipelines
        .iter()
        .map(|p| {
            let expanded_steps = expand_fragment_steps(
                p.steps.as_slice(),
                executable_name,
                &mut expansion_stack,
                fragment_defs,
            )?;

            Ok(HirPipeline {
                steps: expanded_steps
                    .iter()
                    .map(|step| {
                        lower_step(
                            step,
                            executable_name,
                            executable_names,
                            recursive_max_depth,
                            default_core_module,
                        )
                    })
                    .collect(),
            })
        })
        .collect()
}

fn expand_fragment_steps(
    steps: &[PipelineStep],
    owner_name: &str,
    expansion_stack: &mut Vec<String>,
    fragment_defs: &HashMap<String, &FragmentDef>,
) -> Result<Vec<PipelineStep>, GraphemeError> {
    let mut out = Vec::new();

    for step in steps {
        match step {
            PipelineStep::Call(call) if fragment_defs.contains_key(&call.target) => {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}': fragment '{}' cannot be invoked via call; use a bare fragment step",
                    owner_name, call.target
                )));
            }
            PipelineStep::Field(field) => {
                let Some(fragment_def) = fragment_defs.get(&field.name) else {
                    out.push(step.clone());
                    continue;
                };

                if field.module.is_some()
                    || !field.args.is_empty()
                    || !field.directives.is_empty()
                    || field.selection.is_some()
                {
                    return Err(GraphemeError::TypeError(format!(
                        "definition '{}': fragment '{}' must be invoked as a bare step with no args/directives/selection",
                        owner_name, field.name
                    )));
                }

                if expansion_stack.iter().any(|name| name == &field.name) {
                    let mut cycle = expansion_stack.clone();
                    cycle.push(field.name.clone());
                    return Err(GraphemeError::TypeError(format!(
                        "definition '{}': fragment expansion cycle detected: {}",
                        owner_name,
                        cycle.join(" -> ")
                    )));
                }

                expansion_stack.push(field.name.clone());
                for fragment_pipeline in &fragment_def.pipelines {
                    let expanded = expand_fragment_steps(
                        fragment_pipeline.steps.as_slice(),
                        owner_name,
                        expansion_stack,
                        fragment_defs,
                    )?;
                    out.extend(expanded);
                }
                expansion_stack.pop();
            }
            _ => out.push(step.clone()),
        }
    }

    Ok(out)
}

fn lower_step(
    step: &PipelineStep,
    executable_name: &str,
    executable_names: &HashSet<String>,
    recursive_max_depth: Option<u32>,
    default_core_module: bool,
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

            let resolved_module = if field.module.is_some() {
                field.module.clone()
            } else if default_core_module {
                Some("core".to_string())
            } else {
                None
            };

            let capability = match &resolved_module {
                Some(module) => Capability::from_module_op(module, &field.name),
                None => Capability::from_bare_op(&field.name),
            };

            HirStep {
                op: field.name.clone(),
                module: resolved_module,
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
        PipelineStep::StructInit(init) => {
            let fields_value = value_to_json(&Value::Object(init.fields.clone()));
            let mut args = Map::new();
            args.insert("fields".to_string(), fields_value);
            HirStep {
                op: "set_fields".to_string(),
                module: Some("core".to_string()),
                arg_count: 1,
                args: JsonValue::Object(args),
                has_selection: false,
                capability: Capability::from_module_op("core", "set_fields"),
            }
        }
    }
}


fn lower_params(variables: &[VariableDef]) -> Vec<HirParam> {
    variables
        .iter()
        .map(|variable| HirParam {
            name: variable.name.clone(),
            type_ref: variable.type_ref.clone(),
            default: variable.default.as_ref().map(value_to_json),
            required: variable.default.is_none(),
        })
        .collect()
}

fn lower_args(args: &[(String, Value)]) -> JsonValue {
    let mut object = Map::new();
    for (k, v) in args {
        object.insert(k.clone(), value_to_json(v));
    }
    JsonValue::Object(object)
}

fn normalize_executable_directives(
    executable_name: &str,
    directives: &[Directive],
) -> Result<Vec<Directive>, GraphemeError> {
    let resilient_directives = directives.iter().filter(|d| d.name == "resilient").count();

    if resilient_directives == 0 {
        return Ok(directives.to_vec());
    }

    if resilient_directives > 1 {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': multiple @resilient directives are not allowed",
            executable_name
        )));
    }

    if directives
        .iter()
        .any(|d| d.name == "loop" || d.name == "retry" || d.name == "timeout")
    {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @resilient cannot be combined with @loop/@retry/@timeout",
            executable_name
        )));
    }

    let mut normalized = directives
        .iter()
        .filter(|d| d.name != "resilient")
        .cloned()
        .collect::<Vec<_>>();

    let resilient = directives
        .iter()
        .find(|d| d.name == "resilient")
        .expect("resilient directive count checked");

    for (key, value) in &resilient.args {
        let directive_name = match key.as_str() {
            "loop" => "loop",
            "retry" | "r" => "retry",
            "timeout" | "t" => "timeout",
            other => {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}': @resilient unknown key '{}'",
                    executable_name, other
                )))
            }
        };

        let Value::Object(fields) = value else {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @resilient.{} must be an object",
                executable_name, key
            )));
        };

        normalized.push(Directive {
            name: directive_name.to_string(),
            args: fields.clone(),
        });
    }

    Ok(normalized)
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

fn retry_directive_count(directives: &[Directive]) -> usize {
    directives.iter().filter(|d| d.name == "retry").count()
}

fn first_retry_args(directives: &[Directive]) -> Option<JsonValue> {
    directives
        .iter()
        .find(|d| d.name == "retry")
        .map(|d| lower_args(&d.args))
}

fn timeout_directive_count(directives: &[Directive]) -> usize {
    directives.iter().filter(|d| d.name == "timeout").count()
}

fn first_timeout_args(directives: &[Directive]) -> Option<JsonValue> {
    directives
        .iter()
        .find(|d| d.name == "timeout")
        .map(|d| lower_args(&d.args))
}

fn first_intent_args(directives: &[Directive]) -> Option<JsonValue> {
    directives
        .iter()
        .find(|d| d.name == "intent")
        .map(|d| lower_args(&d.args))
}

fn has_core_default_directive(directives: &[Directive]) -> bool {
    directives.iter().any(|d| d.name == "core_default")
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
