use crate::error::GraphemeError;
use grapheme_artifact::{CapabilityPolicy, MirProgram};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

use super::hir::{HirExecutableKind, HirProgram, HirStep};
use crate::ast::TypeRef;
use crate::ast::ImportKind;

#[derive(Debug, Clone, Copy)]
enum ArgType {
    String,
    Object,
    Array,
    Any,
}

#[derive(Debug, Clone, Copy)]
struct ArgSpec {
    name: &'static str,
    ty: ArgType,
    required: bool,
}

#[derive(Debug, Clone, Copy)]
struct OpSpec {
    module: &'static str,
    op: &'static str,
    args: &'static [ArgSpec],
}

const CORE_ECHO_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "message", ty: ArgType::String, required: false },
];
const CORE_MAP_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "items", ty: ArgType::Array, required: false },
    ArgSpec { name: "field", ty: ArgType::String, required: false },
];
const CORE_FILTER_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "items", ty: ArgType::Array, required: false },
    ArgSpec { name: "field", ty: ArgType::String, required: true },
    ArgSpec { name: "equals", ty: ArgType::Any, required: true },
];
const CORE_MERGE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "left", ty: ArgType::Object, required: false },
    ArgSpec { name: "right", ty: ArgType::Object, required: false },
];
const CORE_PICK_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "fields", ty: ArgType::Array, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_VALIDATE_SCHEMA_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "required", ty: ArgType::Array, required: true },
    ArgSpec { name: "data", ty: ArgType::Object, required: true },
];
const CORE_ADD_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_SUB_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_INC_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "value", ty: ArgType::Any, required: false },
];
const CORE_DEC_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "value", ty: ArgType::Any, required: false },
];
const CORE_EQ_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_LT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_GT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_GTE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_LTE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_INC_FIELD_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "field", ty: ArgType::String, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_DEC_FIELD_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "field", ty: ArgType::String, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_SET_FIELDS_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "fields", ty: ArgType::Object, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];

const IO_READ_TEXT_ARGS: &[ArgSpec] = &[ArgSpec { name: "path", ty: ArgType::String, required: true }];
const IO_WRITE_TEXT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "path", ty: ArgType::String, required: true },
    ArgSpec { name: "text", ty: ArgType::String, required: true },
];
const IO_LIST_DIR_ARGS: &[ArgSpec] = &[ArgSpec { name: "path", ty: ArgType::String, required: false }];

const HTTP_GET_ARGS: &[ArgSpec] = &[ArgSpec { name: "url", ty: ArgType::String, required: true }];
const HTTP_POST_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "url", ty: ArgType::String, required: true },
    ArgSpec { name: "body", ty: ArgType::Any, required: false },
];

const TCP_CONNECT_ARGS: &[ArgSpec] = &[ArgSpec { name: "target", ty: ArgType::String, required: true }];
const TCP_SEND_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "session", ty: ArgType::String, required: false },
    ArgSpec { name: "target", ty: ArgType::String, required: false },
    ArgSpec { name: "data", ty: ArgType::String, required: false },
];
const TCP_RECEIVE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "session", ty: ArgType::String, required: false },
    ArgSpec { name: "target", ty: ArgType::String, required: false },
    ArgSpec { name: "max_bytes", ty: ArgType::Any, required: false },
];

const SMTP_SEND_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "to", ty: ArgType::String, required: true },
    ArgSpec { name: "from", ty: ArgType::String, required: false },
    ArgSpec { name: "server", ty: ArgType::String, required: false },
    ArgSpec { name: "subject", ty: ArgType::String, required: false },
    ArgSpec { name: "body", ty: ArgType::String, required: false },
];

const SECRETS_GET_ARGS: &[ArgSpec] = &[ArgSpec { name: "name", ty: ArgType::String, required: true }];
const SECRETS_SIGN_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "secret", ty: ArgType::String, required: true },
    ArgSpec { name: "payload", ty: ArgType::Any, required: false },
];

const MEMORY_STORE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "key", ty: ArgType::String, required: false },
    ArgSpec { name: "value", ty: ArgType::Any, required: false },
];
const MEMORY_LOAD_ARGS: &[ArgSpec] = &[ArgSpec { name: "key", ty: ArgType::String, required: false }];

const DOCS_GUIDE_ARGS: &[ArgSpec] = &[ArgSpec { name: "topic", ty: ArgType::String, required: false }];
const DOCS_EXAMPLE_ARGS: &[ArgSpec] = &[ArgSpec { name: "module", ty: ArgType::String, required: false }];
const HTML_TO_MD_ARGS: &[ArgSpec] = &[ArgSpec { name: "html", ty: ArgType::String, required: false }];
const JSON_PARSE_ARGS: &[ArgSpec] = &[ArgSpec { name: "text", ty: ArgType::String, required: false }];
const CSV_TO_LIST_ARGS: &[ArgSpec] = &[ArgSpec { name: "text", ty: ArgType::String, required: false }];
const YAML_TO_JSON_ARGS: &[ArgSpec] = &[ArgSpec { name: "text", ty: ArgType::String, required: false }];

const OP_SPECS: &[OpSpec] = &[
    OpSpec { module: "core", op: "echo", args: CORE_ECHO_ARGS },
    OpSpec { module: "core", op: "map", args: CORE_MAP_ARGS },
    OpSpec { module: "core", op: "filter", args: CORE_FILTER_ARGS },
    OpSpec { module: "core", op: "merge", args: CORE_MERGE_ARGS },
    OpSpec { module: "core", op: "pick", args: CORE_PICK_ARGS },
    OpSpec { module: "core", op: "validate_schema", args: CORE_VALIDATE_SCHEMA_ARGS },
    OpSpec { module: "core", op: "add", args: CORE_ADD_ARGS },
    OpSpec { module: "core", op: "sub", args: CORE_SUB_ARGS },
    OpSpec { module: "core", op: "inc", args: CORE_INC_ARGS },
    OpSpec { module: "core", op: "dec", args: CORE_DEC_ARGS },
    OpSpec { module: "core", op: "eq", args: CORE_EQ_ARGS },
    OpSpec { module: "core", op: "lt", args: CORE_LT_ARGS },
    OpSpec { module: "core", op: "gt", args: CORE_GT_ARGS },
    OpSpec { module: "core", op: "gte", args: CORE_GTE_ARGS },
    OpSpec { module: "core", op: "lte", args: CORE_LTE_ARGS },
    OpSpec { module: "core", op: "inc_field", args: CORE_INC_FIELD_ARGS },
    OpSpec { module: "core", op: "dec_field", args: CORE_DEC_FIELD_ARGS },
    OpSpec { module: "core", op: "set_fields", args: CORE_SET_FIELDS_ARGS },
    OpSpec { module: "io", op: "read_text", args: IO_READ_TEXT_ARGS },
    OpSpec { module: "io", op: "write_text", args: IO_WRITE_TEXT_ARGS },
    OpSpec { module: "io", op: "list_dir", args: IO_LIST_DIR_ARGS },
    OpSpec { module: "http", op: "get", args: HTTP_GET_ARGS },
    OpSpec { module: "http", op: "post", args: HTTP_POST_ARGS },
    OpSpec { module: "tcp", op: "connect", args: TCP_CONNECT_ARGS },
    OpSpec { module: "tcp", op: "send", args: TCP_SEND_ARGS },
    OpSpec { module: "tcp", op: "receive", args: TCP_RECEIVE_ARGS },
    OpSpec { module: "smtp", op: "send_mail", args: SMTP_SEND_ARGS },
    OpSpec { module: "secrets", op: "get_secret_handle", args: SECRETS_GET_ARGS },
    OpSpec { module: "secrets", op: "sign_request", args: SECRETS_SIGN_ARGS },
    OpSpec { module: "memory", op: "store_context", args: MEMORY_STORE_ARGS },
    OpSpec { module: "memory", op: "load_context", args: MEMORY_LOAD_ARGS },
    OpSpec { module: "memory", op: "summarize_context", args: &[] },
    OpSpec { module: "docs", op: "native_module_guide", args: DOCS_GUIDE_ARGS },
    OpSpec { module: "docs", op: "native_module_registry", args: &[] },
    OpSpec { module: "docs", op: "native_module_example", args: DOCS_EXAMPLE_ARGS },
    OpSpec { module: "html", op: "to_md", args: HTML_TO_MD_ARGS },
    OpSpec { module: "json", op: "parse", args: JSON_PARSE_ARGS },
    OpSpec { module: "csv", op: "to_list", args: CSV_TO_LIST_ARGS },
    OpSpec { module: "yaml", op: "to_json", args: YAML_TO_JSON_ARGS },
];

pub fn verify_hir(hir: &HirProgram) -> Result<(), GraphemeError> {
    if hir.executable_defs.is_empty() {
        return Err(GraphemeError::VerificationError(
            "program contains no executable definitions".to_string(),
        ));
    }

    let executable_names: HashSet<String> = hir
        .executable_defs
        .iter()
        .map(|d| d.name.clone())
        .collect();

    let struct_fields_by_name = hir
        .struct_defs
        .iter()
        .map(|struct_def| {
            (
                struct_def.name.clone(),
                struct_def
                    .fields
                    .iter()
                    .map(|field| field.name.clone())
                    .collect::<HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();

    let required_struct_fields_by_name = hir
        .struct_defs
        .iter()
        .map(|struct_def| {
            (
                struct_def.name.clone(),
                struct_def
                    .fields
                    .iter()
                    .filter(|field| !field.optional)
                    .map(|field| field.name.clone())
                    .collect::<HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();

    let known_type_names = hir
        .struct_defs
        .iter()
        .map(|struct_def| struct_def.name.clone())
        .collect::<HashSet<_>>();

    let imported_type_namespaces = hir
        .imports
        .iter()
        .filter(|import| matches!(import.kind, ImportKind::Types))
        .map(|import| import.alias.clone())
        .collect::<HashSet<_>>();

    verify_known_type_refs(hir, &known_type_names, &imported_type_namespaces)?;

    for def in &hir.executable_defs {
        if def.name.trim().is_empty() {
            return Err(GraphemeError::VerificationError(
                "executable definition has empty name".to_string(),
            ));
        }

        if def.pipelines.is_empty() {
            return Err(GraphemeError::VerificationError(format!(
                "definition '{}' has no pipeline",
                def.name
            )));
        }

        for (i, pipeline) in def.pipelines.iter().enumerate() {
            if pipeline.steps.is_empty() {
                return Err(GraphemeError::VerificationError(format!(
                    "definition '{}' has empty pipeline at index {}",
                    def.name, i
                )));
            }

            for (step_idx, step) in pipeline.steps.iter().enumerate() {
                verify_step_types(&def.name, i, step_idx, step)?;
                verify_call_step(
                    &def.name,
                    i,
                    step_idx,
                    step,
                    &executable_names,
                    def.recursive_directive_count > 0,
                )?;
                verify_flow_branch_step(&def.name, i, step_idx, step, &executable_names)?;
                verify_typed_current_field_access(
                    &def.name,
                    i,
                    step_idx,
                    step,
                    def.input_type.as_ref(),
                    &struct_fields_by_name,
                )?;
            }

            verify_typed_output_field_population(
                &def.name,
                def.kind.clone(),
                i,
                pipeline.steps.as_slice(),
                def.output_type.as_ref(),
                &struct_fields_by_name,
                &required_struct_fields_by_name,
            )?;
        }

        verify_loop_directive(def)?;
        verify_recursive_directive(def)?;
    }

    Ok(())
}

fn verify_typed_output_field_population(
    def_name: &str,
    def_kind: HirExecutableKind,
    pipeline_idx: usize,
    steps: &[HirStep],
    output_type: Option<&TypeRef>,
    struct_fields_by_name: &HashMap<String, HashSet<String>>,
    required_struct_fields_by_name: &HashMap<String, HashSet<String>>,
) -> Result<(), GraphemeError> {
    if !matches!(def_kind, HirExecutableKind::Query | HirExecutableKind::Mutation) {
        return Ok(());
    }

    let Some(TypeRef::Named(output_type_name, _)) = output_type else {
        return Ok(());
    };

    let Some(all_fields) = struct_fields_by_name.get(output_type_name) else {
        return Ok(());
    };
    let Some(required_fields) = required_struct_fields_by_name.get(output_type_name) else {
        return Ok(());
    };

    if required_fields.is_empty() {
        return Ok(());
    }

    let mut provided_fields = HashSet::new();
    let mut saw_literal_patch = false;
    for step in steps {
        let Some(module) = step.module.as_deref() else {
            continue;
        };

        if !module.eq_ignore_ascii_case("core") {
            continue;
        }

        let Some(args_obj) = step.args.as_object() else {
            continue;
        };

        let literal_patch = match step.op.as_str() {
            "merge" => args_obj.get("right").and_then(|v| v.as_object()),
            "set_fields" => args_obj.get("fields").and_then(|v| v.as_object()),
            _ => None,
        };

        let Some(literal_patch) = literal_patch else {
            continue;
        };

        saw_literal_patch = true;

        for key in literal_patch.keys() {
            if !all_fields.contains(key) {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}', pipeline {}: field '{}' is not declared on output type '{}'",
                    def_name, pipeline_idx, key, output_type_name
                )));
            }
            provided_fields.insert(key.clone());
        }
    }

    if !saw_literal_patch {
        return Ok(());
    }

    let missing_required = required_fields
        .iter()
        .filter(|field| !provided_fields.contains(*field))
        .cloned()
        .collect::<Vec<_>>();

    if !missing_required.is_empty() {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}: output type '{}' missing required fields in literal state patches: {}",
            def_name,
            pipeline_idx,
            output_type_name,
            missing_required.join(", ")
        )));
    }

    Ok(())
}

fn verify_known_type_refs(
    hir: &HirProgram,
    known_type_names: &HashSet<String>,
    imported_type_namespaces: &HashSet<String>,
) -> Result<(), GraphemeError> {
    for struct_def in &hir.struct_defs {
        for field in &struct_def.fields {
            verify_type_ref_known(
                &field.type_ref,
                known_type_names,
                imported_type_namespaces,
                &format!("struct '{}' field '{}'", struct_def.name, field.name),
            )?;
        }
    }

    for def in &hir.executable_defs {
        if let Some(input_type) = def.input_type.as_ref() {
            verify_type_ref_known(
                input_type,
                known_type_names,
                imported_type_namespaces,
                &format!("definition '{}' input type", def.name),
            )?;
        }

        if let Some(output_type) = def.output_type.as_ref() {
            verify_type_ref_known(
                output_type,
                known_type_names,
                imported_type_namespaces,
                &format!("definition '{}' output type", def.name),
            )?;
        }
    }

    Ok(())
}

fn verify_type_ref_known(
    type_ref: &TypeRef,
    known_type_names: &HashSet<String>,
    imported_type_namespaces: &HashSet<String>,
    context: &str,
) -> Result<(), GraphemeError> {
    match type_ref {
        TypeRef::Named(name, _) => {
            if let Some((namespace, _type_name)) = name.split_once("::") {
                if imported_type_namespaces.contains(namespace) {
                    return Ok(());
                }

                return Err(GraphemeError::TypeError(format!(
                    "{} references unknown type namespace '{}'",
                    context, namespace
                )));
            }

            if !known_type_names.contains(name) {
                return Err(GraphemeError::TypeError(format!(
                    "{} references unknown type '{}'",
                    context, name
                )));
            }
        }
        TypeRef::List(inner, _) => {
            verify_type_ref_known(inner, known_type_names, imported_type_namespaces, context)?
        }
        TypeRef::Scalar(_, _) => {}
    }

    Ok(())
}

fn verify_typed_current_field_access(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    step: &HirStep,
    input_type: Option<&TypeRef>,
    struct_fields_by_name: &HashMap<String, HashSet<String>>,
) -> Result<(), GraphemeError> {
    let Some(TypeRef::Named(input_type_name, _)) = input_type else {
        return Ok(());
    };

    let Some(known_fields) = struct_fields_by_name.get(input_type_name) else {
        return Ok(());
    };

    let mut refs = Vec::new();
    collect_current_field_refs(&step.args, &mut refs);

    for current_ref in refs {
        if let Some(field) = current_ref.strip_prefix("current.") {
            if field.is_empty() {
                continue;
            }

            let root_field = field.split('.').next().unwrap_or(field);
            if !known_fields.contains(root_field) {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}', pipeline {}, step {}: unknown field '$current.{}' for input type '{}'",
                    def_name, pipeline_idx, step_idx, root_field, input_type_name
                )));
            }
        }
    }

    Ok(())
}

fn collect_current_field_refs(value: &JsonValue, out: &mut Vec<String>) {
    match value {
        JsonValue::Object(map) => {
            if let Some(var_ref) = map.get("$var").and_then(|v| v.as_str()) {
                out.push(var_ref.to_string());
            }

            for nested in map.values() {
                collect_current_field_refs(nested, out);
            }
        }
        JsonValue::Array(items) => {
            for item in items {
                collect_current_field_refs(item, out);
            }
        }
        _ => {}
    }
}

fn verify_recursive_directive(def: &super::hir::HirExecutable) -> Result<(), GraphemeError> {
    if def.recursive_directive_count == 0 {
        return Ok(());
    }

    if def.recursive_directive_count > 1 {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': multiple @recursive directives are not allowed",
            def.name
        )));
    }

    if !matches!(def.kind, HirExecutableKind::Fragment) {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @recursive is only allowed on iterator definitions",
            def.name
        )));
    }

    let args = def
        .recursive_args
        .as_ref()
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @recursive requires named args",
                def.name
            ))
        })?;

    for key in args.keys() {
        if key != "max_depth" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @recursive unknown arg '{}'",
                def.name, key
            )));
        }
    }

    if let Some(max_depth) = args.get("max_depth") {
        let value = max_depth.as_i64().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @recursive max_depth must be an integer",
                def.name
            ))
        })?;

        if value < 1 {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @recursive max_depth must be >= 1",
                def.name
            )));
        }
    }

    Ok(())
}

fn verify_loop_directive(def: &super::hir::HirExecutable) -> Result<(), GraphemeError> {
    if def.loop_directive_count == 0 {
        return Ok(());
    }

    if def.loop_directive_count > 1 {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': multiple @loop directives are not allowed",
            def.name
        )));
    }

    if !matches!(def.kind, HirExecutableKind::Fragment) {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @loop is only allowed on iterator definitions",
            def.name
        )));
    }

    let args = def.loop_args.as_ref().and_then(|v| v.as_object()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}': @loop requires named args",
            def.name
        ))
    })?;

    for key in args.keys() {
        if key != "max" && key != "each" && key != "until" && key != "merge" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @loop unknown arg '{}'",
                def.name, key
            )));
        }
    }

    if let Some(max) = args.get("max") {
        let max_value = max.as_i64().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @loop max must be an integer",
                def.name
            ))
        })?;

        if max_value < 1 {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @loop max must be >= 1",
                def.name
            )));
        }
    }

    if let Some(each) = args.get("each") {
        let selector = each.as_str().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @loop each must be a string",
                def.name
            ))
        })?;

        if selector.trim().is_empty() {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @loop each cannot be empty",
                def.name
            )));
        }

        if selector != "$current" && !selector.starts_with("$current.") {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @loop each must start with '$current'",
                def.name
            )));
        }
    }

    if let Some(until) = args.get("until") {
        let until_obj = until.as_object().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @loop until must be an object",
                def.name
            ))
        })?;

        for key in until_obj.keys() {
            if key != "field" && key != "eq" {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}': @loop until unknown field '{}'",
                    def.name, key
                )));
            }
        }

        let field = until_obj.get("field").and_then(|v| v.as_str()).ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @loop until.field must be a string",
                def.name
            ))
        })?;

        if field.trim().is_empty() {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @loop until.field cannot be empty",
                def.name
            )));
        }

        if !until_obj.contains_key("eq") {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @loop until requires eq",
                def.name
            )));
        }
    }

    if let Some(merge) = args.get("merge") {
        let merge_value = merge.as_str().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @loop merge must be a string",
                def.name
            ))
        })?;

        if merge_value != "replace"
            && merge_value != "append"
            && merge_value != "reduce"
            && merge_value != "none"
        {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @loop merge must be one of replace|append|reduce|none",
                def.name
            )));
        }
    }

    Ok(())
}

fn verify_call_step(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    step: &HirStep,
    executable_names: &HashSet<String>,
    has_recursive_directive: bool,
) -> Result<(), GraphemeError> {
    let Some(module_raw) = step.module.as_deref() else {
        return Ok(());
    };

    if !module_raw.eq_ignore_ascii_case("call") {
        return Ok(());
    }

    if !executable_names.contains(&step.op) {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: unknown call target '{}'",
            def_name, pipeline_idx, step_idx, step.op
        )));
    }

    let args = step.args.as_object().ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: args for call '{}' must be an object",
            def_name, pipeline_idx, step_idx, step.op
        ))
    })?;

    for arg_name in args.keys() {
        if arg_name != "max_depth" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: unknown call arg '{}' for target '{}'",
                def_name, pipeline_idx, step_idx, arg_name, step.op
            )));
        }
    }

    if let Some(max_depth) = args.get("max_depth") {
        if !is_variable_placeholder(max_depth) {
            let value = max_depth.as_i64().ok_or_else(|| {
                GraphemeError::TypeError(format!(
                    "definition '{}', pipeline {}, step {}: call '{}': max_depth must be an integer",
                    def_name, pipeline_idx, step_idx, step.op
                ))
            })?;

            if value < 1 {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}', pipeline {}, step {}: call '{}': max_depth must be >= 1",
                    def_name, pipeline_idx, step_idx, step.op
                )));
            }
        }
    }

    if step.op == def_name && !args.contains_key("max_depth") && !has_recursive_directive {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: self-recursive call requires max_depth",
            def_name, pipeline_idx, step_idx
        )));
    }

    Ok(())
}

fn verify_step_types(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    step: &HirStep,
) -> Result<(), GraphemeError> {
    let Some(module_raw) = step.module.as_deref() else {
        return Ok(());
    };

    let module = module_raw.to_lowercase();
    let maybe_spec = OP_SPECS
        .iter()
        .find(|spec| spec.module == module && spec.op == step.op);

    if maybe_spec.is_none() {
        let module_known = OP_SPECS.iter().any(|spec| spec.module == module);
        if module_known {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: unknown op '{}.{}'",
                def_name, pipeline_idx, step_idx, module_raw, step.op
            )));
        }

        return Ok(());
    }

    let spec = maybe_spec.unwrap();
    let args = step.args.as_object().ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: args for '{}.{}' must be an object",
            def_name, pipeline_idx, step_idx, module_raw, step.op
        ))
    })?;

    for arg_spec in spec.args {
        match args.get(arg_spec.name) {
            Some(value) => {
                if is_variable_placeholder(value) {
                    continue;
                }

                if !value_matches_arg_type(value, arg_spec.ty) {
                    return Err(GraphemeError::TypeError(format!(
                        "definition '{}', pipeline {}, step {}: '{}.{}' arg '{}' expected {}, got {}",
                        def_name,
                        pipeline_idx,
                        step_idx,
                        module_raw,
                        step.op,
                        arg_spec.name,
                        arg_type_label(arg_spec.ty),
                        json_type_label(value)
                    )));
                }
            }
            None => {
                if arg_spec.required {
                    return Err(GraphemeError::TypeError(format!(
                        "definition '{}', pipeline {}, step {}: '{}.{}' missing required arg '{}'",
                        def_name, pipeline_idx, step_idx, module_raw, step.op, arg_spec.name
                    )));
                }
            }
        }
    }

    Ok(())
}

fn verify_flow_branch_step(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    step: &HirStep,
    executable_names: &HashSet<String>,
) -> Result<(), GraphemeError> {
    let Some(module_raw) = step.module.as_deref() else {
        return Ok(());
    };

    if !module_raw.eq_ignore_ascii_case("flow") || step.op != "branch" {
        return Ok(());
    }

    let args = step.args.as_object().ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: args for 'flow.branch' must be an object",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    for arg_name in args.keys() {
        if arg_name != "when" && arg_name != "then" && arg_name != "else" && arg_name != "max_depth" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: unknown arg '{}' for flow.branch",
                def_name, pipeline_idx, step_idx, arg_name
            )));
        }
    }

    let when = args.get("when").and_then(|v| v.as_object()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.branch requires object arg 'when'",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    let field = when.get("field").and_then(|v| v.as_str()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.branch when.field must be a string",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    if field.trim().is_empty() {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.branch when.field cannot be empty",
            def_name, pipeline_idx, step_idx
        )));
    }

    let comparator_keys = ["eq", "gt", "gte", "lt", "lte"];
    let provided = comparator_keys
        .iter()
        .filter_map(|key| when.get(*key).map(|value| (*key, value)))
        .collect::<Vec<_>>();

    if provided.len() != 1 {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.branch when requires exactly one comparator: eq|gt|gte|lt|lte",
            def_name, pipeline_idx, step_idx
        )));
    }

    let (cmp_key, cmp_value) = provided[0];
    if matches!(cmp_key, "gt" | "gte" | "lt" | "lte")
        && !is_variable_placeholder(cmp_value)
        && !cmp_value.is_number()
    {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.branch when.{} must be a number or variable reference",
            def_name, pipeline_idx, step_idx, cmp_key
        )));
    }

    let then_target = parse_branch_target(args.get("then")).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.branch then must be a target (string or symbol)",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    verify_branch_target(def_name, pipeline_idx, step_idx, "then", &then_target, executable_names)?;

    if let Some(else_value) = args.get("else") {
        let else_target = parse_branch_target(Some(else_value)).ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.branch else must be a target (string or symbol)",
                def_name, pipeline_idx, step_idx
            ))
        })?;
        verify_branch_target(def_name, pipeline_idx, step_idx, "else", &else_target, executable_names)?;
    }

    if let Some(max_depth) = args.get("max_depth") {
        let value = max_depth.as_i64().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.branch max_depth must be an integer",
                def_name, pipeline_idx, step_idx
            ))
        })?;
        if value < 1 {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.branch max_depth must be >= 1",
                def_name, pipeline_idx, step_idx
            )));
        }
    }

    Ok(())
}

fn verify_branch_target(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    label: &str,
    target: &str,
    executable_names: &HashSet<String>,
) -> Result<(), GraphemeError> {
    if target == "$return" {
        return Ok(());
    }

    if executable_names.contains(target) {
        return Ok(());
    }

    Err(GraphemeError::TypeError(format!(
        "definition '{}', pipeline {}, step {}: flow.branch {} target '{}' not found",
        def_name, pipeline_idx, step_idx, label, target
    )))
}

fn parse_branch_target(value: Option<&JsonValue>) -> Option<String> {
    let value = value?;

    if let Some(target) = value.as_str() {
        return Some(target.to_string());
    }

    let object = value.as_object()?;
    let symbol = object.get("$symbol")?.as_str()?;
    if symbol == "return" {
        return Some("$return".to_string());
    }

    Some(symbol.to_string())
}

fn is_variable_placeholder(value: &JsonValue) -> bool {
    if matches!(value, JsonValue::String(s) if s.starts_with('$')) {
        return true;
    }

    value
        .as_object()
        .and_then(|map| map.get("$var"))
        .and_then(|v| v.as_str())
        .is_some()
}

fn value_matches_arg_type(value: &JsonValue, expected: ArgType) -> bool {
    match expected {
        ArgType::String => value.is_string(),
        ArgType::Object => value.is_object(),
        ArgType::Array => value.is_array(),
        ArgType::Any => true,
    }
}

fn arg_type_label(expected: ArgType) -> &'static str {
    match expected {
        ArgType::String => "string",
        ArgType::Object => "object",
        ArgType::Array => "array",
        ArgType::Any => "any",
    }
}

fn json_type_label(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "bool",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

pub fn verify_mir(mir: &MirProgram, policy: &CapabilityPolicy) -> Result<(), GraphemeError> {
    if mir.functions.is_empty() {
        return Err(GraphemeError::VerificationError(
            "MIR has no functions".to_string(),
        ));
    }

    for function in &mir.functions {
        if function.blocks.is_empty() {
            return Err(GraphemeError::VerificationError(format!(
                "MIR function '{}' has no blocks",
                function.name
            )));
        }
    }

    for capability in &mir.capabilities {
        if !policy.is_allowed(capability) {
            return Err(GraphemeError::CapabilityDenied(capability.0.clone()));
        }
    }

    Ok(())
}
