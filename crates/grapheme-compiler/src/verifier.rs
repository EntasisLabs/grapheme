use crate::error::GraphemeError;
use grapheme_artifact::{CapabilityPolicy, MirProgram};
use serde_json::Value as JsonValue;

use super::hir::{HirProgram, HirStep};

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

const OP_SPECS: &[OpSpec] = &[
    OpSpec { module: "core", op: "echo", args: CORE_ECHO_ARGS },
    OpSpec { module: "core", op: "map", args: CORE_MAP_ARGS },
    OpSpec { module: "core", op: "filter", args: CORE_FILTER_ARGS },
    OpSpec { module: "core", op: "merge", args: CORE_MERGE_ARGS },
    OpSpec { module: "core", op: "pick", args: CORE_PICK_ARGS },
    OpSpec { module: "core", op: "validate_schema", args: CORE_VALIDATE_SCHEMA_ARGS },
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
];

pub fn verify_hir(hir: &HirProgram) -> Result<(), GraphemeError> {
    if hir.executable_defs.is_empty() {
        return Err(GraphemeError::VerificationError(
            "program contains no executable definitions".to_string(),
        ));
    }

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
            }
        }
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

fn is_variable_placeholder(value: &JsonValue) -> bool {
    matches!(value, JsonValue::String(s) if s.starts_with('$'))
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
