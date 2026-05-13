use crate::error::GraphemeError;
use grapheme_artifact::{CapabilityPolicy, MirProgram};
use grapheme_signatures::{find_op_spec, op_specs, ArgType};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

use super::hir::{HirExecutable, HirExecutableKind, HirProgram, HirStateMachineDef, HirStep};
use crate::ast::TypeRef;
use crate::ast::ImportKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintWarning {
    pub code: String,
    pub message: String,
    pub definition: String,
    pub pipeline: usize,
    pub step: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableKindPolicyMode {
    Compatibility,
    StrictMutationOnly,
}

pub fn verify_hir(hir: &HirProgram) -> Result<(), GraphemeError> {
    let _ = verify_hir_with_lints_mode(hir, ExecutableKindPolicyMode::Compatibility)?;
    Ok(())
}

pub fn verify_hir_with_lints(hir: &HirProgram) -> Result<Vec<LintWarning>, GraphemeError> {
    verify_hir_with_lints_mode(hir, ExecutableKindPolicyMode::Compatibility)
}

pub fn verify_hir_with_lints_mode(
    hir: &HirProgram,
    policy_mode: ExecutableKindPolicyMode,
) -> Result<Vec<LintWarning>, GraphemeError> {
    if hir.executable_defs.is_empty() {
        return Err(GraphemeError::VerificationError(
            "program contains no executable definitions".to_string(),
        ));
    }

    let mut lint_warnings = Vec::new();

    let executable_names: HashSet<String> = hir
        .executable_defs
        .iter()
        .map(|d| d.name.clone())
        .collect();

    let executable_by_name = hir
        .executable_defs
        .iter()
        .map(|d| (d.name.clone(), d))
        .collect::<HashMap<_, _>>();

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

    let struct_field_types_by_name = hir
        .struct_defs
        .iter()
        .map(|struct_def| {
            (
                struct_def.name.clone(),
                struct_def
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.type_ref.clone()))
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect::<HashMap<_, _>>();

    let enum_members_by_name = hir
        .enum_defs
        .iter()
        .map(|enum_def| {
            (
                enum_def.name.clone(),
                enum_def
                    .members
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();

    let state_machines_by_enum = index_state_machines_by_enum(&hir.state_machines)?;

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
        .chain(hir.enum_defs.iter().map(|enum_def| enum_def.name.clone()))
        .collect::<HashSet<_>>();

    let imported_type_namespaces = hir
        .imports
        .iter()
        .filter(|import| matches!(import.kind, ImportKind::Types))
        .map(|import| import.alias.clone())
        .collect::<HashSet<_>>();

    verify_known_type_refs(hir, &known_type_names, &imported_type_namespaces)?;
    verify_state_machines(hir, &enum_members_by_name)?;

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
                verify_flow_branch_step(
                    &def.name,
                    i,
                    step_idx,
                    step,
                    &executable_names,
                    &executable_by_name,
                    def.input_type.as_ref(),
                    &struct_field_types_by_name,
                    &enum_members_by_name,
                    &state_machines_by_enum,
                )?;
                verify_flow_match_step(
                    &def.name,
                    i,
                    step_idx,
                    step,
                    &executable_names,
                    &executable_by_name,
                    def.input_type.as_ref(),
                    &struct_field_types_by_name,
                    &enum_members_by_name,
                    &state_machines_by_enum,
                )?;
                verify_typed_current_field_access(
                    &def.name,
                    i,
                    step_idx,
                    step,
                    def.input_type.as_ref(),
                    &struct_fields_by_name,
                )?;
                apply_executable_kind_policy(
                    &mut lint_warnings,
                    policy_mode,
                    &def.kind,
                    &def.name,
                    i,
                    step_idx,
                    step,
                )?;
            }

            verify_state_machine_transitions_in_pipeline(
                &def.name,
                i,
                pipeline.steps.as_slice(),
                def.input_type.as_ref(),
                &struct_field_types_by_name,
                &enum_members_by_name,
                &state_machines_by_enum,
            )?;

            verify_typed_output_field_population(
                &def.name,
                def.kind.clone(),
                i,
                pipeline.steps.as_slice(),
                def.output_type.as_ref(),
                &struct_fields_by_name,
                &required_struct_fields_by_name,
            )?;

            lint_warnings.extend(emit_echo_shape_clobber_lints(
                &def.name,
                i,
                pipeline.steps.as_slice(),
            ));
        }

        verify_loop_directive(def)?;
        verify_recursive_directive(def)?;
        verify_retry_directive(def, &executable_names)?;
        verify_timeout_directive(def, &executable_names)?;
    }

    Ok(lint_warnings)
}

fn apply_executable_kind_policy(
    lint_warnings: &mut Vec<LintWarning>,
    policy_mode: ExecutableKindPolicyMode,
    def_kind: &HirExecutableKind,
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    step: &HirStep,
) -> Result<(), GraphemeError> {
    if !is_bridge_write_like_step(step) {
        return Ok(());
    }

    if matches!(def_kind, HirExecutableKind::Mutation) {
        return Ok(());
    }

    let kind_label = executable_kind_label(def_kind);
    let op_label = format!(
        "{}.{}",
        step.module.as_deref().unwrap_or(""),
        step.op.as_str()
    );
    let message = format!(
        "definition '{}', pipeline {}, step {}: write-like op '{}' is restricted to mutation declarations (found in {})",
        def_name,
        pipeline_idx,
        step_idx,
        op_label,
        kind_label
    );

    if matches!(policy_mode, ExecutableKindPolicyMode::StrictMutationOnly) {
        return Err(GraphemeError::TypeError(message));
    }

    lint_warnings.push(LintWarning {
        code: "kind-write-outside-mutation".to_string(),
        message,
        definition: def_name.to_string(),
        pipeline: pipeline_idx,
        step: step_idx,
    });

    Ok(())
}

fn is_bridge_write_like_step(step: &HirStep) -> bool {
    step
        .module
        .as_deref()
        .map(|m| m.eq_ignore_ascii_case("core"))
        .unwrap_or(false)
    && matches!(step.op.as_str(), "set_path" | "inc_field" | "dec_field" | "apply_lane")
}

fn executable_kind_label(kind: &HirExecutableKind) -> &'static str {
    match kind {
        HirExecutableKind::Query => "query",
        HirExecutableKind::Mutation => "mutation",
        HirExecutableKind::Subscription => "subscription",
        HirExecutableKind::Fragment => "iterator/node",
    }
}

fn emit_echo_shape_clobber_lints(
    def_name: &str,
    pipeline_idx: usize,
    steps: &[HirStep],
) -> Vec<LintWarning> {
    let mut out = Vec::new();

    for (idx, step) in steps.iter().enumerate() {
        if !is_core_echo(step) {
            continue;
        }

        let Some(next_step) = steps.get(idx + 1) else {
            continue;
        };

        let mut refs = Vec::new();
        collect_current_field_refs(&next_step.args, &mut refs);

        let risky = refs.iter().any(|current_ref| {
            current_ref
                .strip_prefix("current.")
                .and_then(|field| field.split('.').next())
                .map(|root| !root.is_empty() && root != "message")
                .unwrap_or(false)
        });

        if risky {
            out.push(LintWarning {
                code: "llm-shape-clobber".to_string(),
                message: format!(
                    "definition '{}', pipeline {}, step {} uses core.echo before step {} reads $current.<field>; core.echo rewrites state shape. Consider core.tap for non-mutating diagnostics.",
                    def_name,
                    pipeline_idx,
                    idx,
                    idx + 1
                ),
                definition: def_name.to_string(),
                pipeline: pipeline_idx,
                step: idx,
            });
        }
    }

    out
}

fn is_core_echo(step: &HirStep) -> bool {
    step
        .module
        .as_deref()
        .map(|m| m.eq_ignore_ascii_case("core"))
        .unwrap_or(false)
        && step.op == "echo"
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

fn verify_state_machines(
    hir: &HirProgram,
    enum_members_by_name: &HashMap<String, HashSet<String>>,
) -> Result<(), GraphemeError> {
    for sm in &hir.state_machines {
        verify_state_machine(sm, enum_members_by_name)?;
    }

    Ok(())
}

fn verify_state_machine(
    sm: &HirStateMachineDef,
    enum_members_by_name: &HashMap<String, HashSet<String>>,
) -> Result<(), GraphemeError> {
    let members = enum_members_by_name.get(&sm.enum_name).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "state_machine '{}' references unknown enum '{}'",
            sm.name, sm.enum_name
        ))
    })?;

    let terminal_set = sm.terminals.iter().cloned().collect::<HashSet<_>>();
    for terminal in &sm.terminals {
        if !members.contains(terminal) {
            return Err(GraphemeError::TypeError(format!(
                "state_machine '{}' terminal '{}' is not a member of enum '{}'",
                sm.name, terminal, sm.enum_name
            )));
        }
    }

    let mut transition_pairs = HashSet::new();
    let mut outgoing = HashMap::<String, HashSet<String>>::new();
    for t in &sm.transitions {
        if !members.contains(&t.from) {
            return Err(GraphemeError::TypeError(format!(
                "state_machine '{}' transition from '{}' is not a member of enum '{}'",
                sm.name, t.from, sm.enum_name
            )));
        }
        if !members.contains(&t.to) {
            return Err(GraphemeError::TypeError(format!(
                "state_machine '{}' transition to '{}' is not a member of enum '{}'",
                sm.name, t.to, sm.enum_name
            )));
        }
        if terminal_set.contains(&t.from) {
            return Err(GraphemeError::TypeError(format!(
                "state_machine '{}' transition '{}' -> '{}' is invalid: terminals cannot have outgoing transitions",
                sm.name, t.from, t.to
            )));
        }

        if !transition_pairs.insert((t.from.clone(), t.to.clone())) {
            return Err(GraphemeError::TypeError(format!(
                "state_machine '{}' has duplicate transition '{} -> {}'",
                sm.name, t.from, t.to
            )));
        }

        outgoing
            .entry(t.from.clone())
            .or_default()
            .insert(t.to.clone());
    }

    let mut missing = members
        .iter()
        .filter(|m| !terminal_set.contains(*m) && !outgoing.contains_key(*m))
        .cloned()
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        missing.sort();
        return Err(GraphemeError::TypeError(format!(
            "state_machine '{}' non-terminal states without outgoing transitions: {}",
            sm.name,
            missing.join(", ")
        )));
    }

    Ok(())
}

fn index_state_machines_by_enum(
    state_machines: &[HirStateMachineDef],
) -> Result<HashMap<String, HirStateMachineDef>, GraphemeError> {
    let mut out = HashMap::new();
    for sm in state_machines {
        if let Some(existing) = out.insert(sm.enum_name.clone(), sm.clone()) {
            return Err(GraphemeError::TypeError(format!(
                "multiple state_machines declared for enum '{}': '{}' and '{}'",
                sm.enum_name, existing.name, sm.name
            )));
        }
    }
    Ok(out)
}

fn verify_state_machine_transitions_in_pipeline(
    def_name: &str,
    pipeline_idx: usize,
    steps: &[HirStep],
    input_type: Option<&TypeRef>,
    struct_field_types_by_name: &HashMap<String, HashMap<String, TypeRef>>,
    enum_members_by_name: &HashMap<String, HashSet<String>>,
    state_machines_by_enum: &HashMap<String, HirStateMachineDef>,
) -> Result<(), GraphemeError> {
    let Some(TypeRef::Named(input_type_name, _)) = input_type else {
        return Ok(());
    };

    let Some(field_types) = struct_field_types_by_name.get(input_type_name) else {
        return Ok(());
    };

    let mut field_context = HashMap::<String, (&HirStateMachineDef, &HashSet<String>)>::new();
    for (field_name, type_ref) in field_types {
        let TypeRef::Named(enum_name, _) = type_ref else {
            continue;
        };

        let Some(sm) = state_machines_by_enum.get(enum_name) else {
            continue;
        };
        let Some(members) = enum_members_by_name.get(enum_name) else {
            continue;
        };

        field_context.insert(field_name.clone(), (sm, members));
    }

    if field_context.is_empty() {
        return Ok(());
    }

    let mut last_literal_state = HashMap::<String, String>::new();

    for (step_idx, step) in steps.iter().enumerate() {
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

        for (field, value) in literal_patch {
            let Some((sm, members)) = field_context.get(field) else {
                continue;
            };

            let Some(next_state) = parse_literal_member(value) else {
                // Dynamic values cannot be statically transition-checked.
                continue;
            };

            if !members.contains(next_state) {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}', pipeline {}, step {}: field '{}' assigns unknown enum member '{}' for enum '{}'",
                    def_name, pipeline_idx, step_idx, field, next_state, sm.enum_name
                )));
            }

            if let Some(prev_state) = last_literal_state.get(field) {
                if prev_state == next_state {
                    continue;
                }

                if sm.terminals.iter().any(|t| t == prev_state) {
                    return Err(GraphemeError::TypeError(format!(
                        "definition '{}', pipeline {}, step {}: invalid transition for field '{}' from terminal state '{}' to '{}' in state_machine '{}'",
                        def_name, pipeline_idx, step_idx, field, prev_state, next_state, sm.name
                    )));
                }

                let allowed = sm
                    .transitions
                    .iter()
                    .any(|t| t.from == *prev_state && t.to == next_state);
                if !allowed {
                    return Err(GraphemeError::TypeError(format!(
                        "definition '{}', pipeline {}, step {}: invalid transition for field '{}' from '{}' to '{}' in state_machine '{}'",
                        def_name, pipeline_idx, step_idx, field, prev_state, next_state, sm.name
                    )));
                }
            }

            last_literal_state.insert(field.clone(), next_state.to_string());
        }
    }

    Ok(())
}

fn parse_literal_member(value: &JsonValue) -> Option<&str> {
    if let Some(s) = value.as_str() {
        return Some(s);
    }

    value
        .as_object()
        .and_then(|obj| obj.get("$symbol"))
        .and_then(|v| v.as_str())
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
        JsonValue::String(text) => {
            collect_current_refs_from_text(text, out);
        }
        _ => {}
    }
}

fn collect_current_refs_from_text(text: &str, out: &mut Vec<String>) {
    for token in text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '$' || c == '_' || c == '.'))
    {
        if token == "$current" || token.starts_with("$current.") {
            out.push(token.trim_start_matches('$').to_string());
        }
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
        let selector = loop_each_selector(each).ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @loop each must be a string or variable reference",
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

fn loop_each_selector(value: &JsonValue) -> Option<String> {
    if let Some(selector) = value.as_str() {
        return Some(selector.to_string());
    }

    value
        .as_object()
        .and_then(|obj| obj.get("$var"))
        .and_then(|v| v.as_str())
        .map(|v| format!("${v}"))
}

fn verify_retry_directive(
    def: &super::hir::HirExecutable,
    executable_names: &HashSet<String>,
) -> Result<(), GraphemeError> {
    if def.retry_directive_count == 0 {
        return Ok(());
    }

    if def.retry_directive_count > 1 {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': multiple @retry directives are not allowed",
            def.name
        )));
    }

    if !matches!(def.kind, HirExecutableKind::Fragment) {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @retry is only allowed on iterator definitions",
            def.name
        )));
    }

    let args = def.retry_args.as_ref().and_then(|v| v.as_object()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}': @retry requires named args",
            def.name
        ))
    })?;

    for key in args.keys() {
        if key != "max" && key != "backoff_ms" && key != "on_fail" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @retry unknown arg '{}'",
                def.name, key
            )));
        }
    }

    let max = args.get("max").and_then(|v| v.as_i64()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}': @retry max must be an integer",
            def.name
        ))
    })?;
    if max < 1 {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @retry max must be >= 1",
            def.name
        )));
    }

    if let Some(backoff_ms) = args.get("backoff_ms") {
        let value = backoff_ms.as_i64().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}': @retry backoff_ms must be an integer",
                def.name
            ))
        })?;
        if value < 0 {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @retry backoff_ms must be >= 0",
                def.name
            )));
        }
    }

    let on_fail = args
        .get("on_fail")
        .and_then(|v| parse_branch_target(Some(v)))
        .ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}': @retry on_fail must be a target (string or symbol)",
            def.name
        ))
    })?;

    if on_fail != "$return" && !executable_names.contains(&on_fail) {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @retry on_fail target '{}' not found",
            def.name, on_fail
        )));
    }

    Ok(())
}

fn verify_timeout_directive(
    def: &super::hir::HirExecutable,
    executable_names: &HashSet<String>,
) -> Result<(), GraphemeError> {
    if def.timeout_directive_count == 0 {
        return Ok(());
    }

    if def.timeout_directive_count > 1 {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': multiple @timeout directives are not allowed",
            def.name
        )));
    }

    if !matches!(def.kind, HirExecutableKind::Fragment) {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @timeout is only allowed on iterator definitions",
            def.name
        )));
    }

    let args = def.timeout_args.as_ref().and_then(|v| v.as_object()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}': @timeout requires named args",
            def.name
        ))
    })?;

    for key in args.keys() {
        if key != "ms" && key != "on_timeout" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}': @timeout unknown arg '{}'",
                def.name, key
            )));
        }
    }

    let ms = args.get("ms").and_then(|v| v.as_i64()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}': @timeout ms must be an integer",
            def.name
        ))
    })?;
    if ms < 1 {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @timeout ms must be >= 1",
            def.name
        )));
    }

    let on_timeout = args
        .get("on_timeout")
        .and_then(|v| parse_branch_target(Some(v)))
        .ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}': @timeout on_timeout must be a target (string or symbol)",
            def.name
        ))
    })?;

    if on_timeout != "$return" && !executable_names.contains(&on_timeout) {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}': @timeout on_timeout target '{}' not found",
            def.name, on_timeout
        )));
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
    let maybe_spec = find_op_spec(&module, &step.op);

    if maybe_spec.is_none() {
        let module_known = op_specs().iter().any(|spec| spec.module == module);
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
    executable_by_name: &HashMap<String, &HirExecutable>,
    input_type: Option<&TypeRef>,
    struct_field_types_by_name: &HashMap<String, HashMap<String, TypeRef>>,
    enum_members_by_name: &HashMap<String, HashSet<String>>,
    state_machines_by_enum: &HashMap<String, HirStateMachineDef>,
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
    let mut enum_context: Option<(String, String)> = None;

    if let Some(enum_name) = resolve_enum_name_for_when_field(
        field,
        input_type,
        struct_field_types_by_name,
        enum_members_by_name,
    ) {
        enum_context = Some((field.to_string(), enum_name.clone()));
        if cmp_key != "eq" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: enum field '{}' only supports eq comparator",
                def_name, pipeline_idx, step_idx, field
            )));
        }

        if !is_variable_placeholder(cmp_value) {
            let member = parse_enum_member_value(cmp_value).ok_or_else(|| {
                GraphemeError::TypeError(format!(
                    "definition '{}', pipeline {}, step {}: enum field '{}' comparison must use a literal member",
                    def_name, pipeline_idx, step_idx, field
                ))
            })?;

            let members = enum_members_by_name
                .get(&enum_name)
                .expect("enum name resolved from existing enum map");

            if !members.contains(member) {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}', pipeline {}, step {}: unknown enum member '{}' for enum '{}'",
                    def_name, pipeline_idx, step_idx, member, enum_name
                )));
            }
        }
    }

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

    if let Some((enum_field, enum_name)) = enum_context.as_ref() {
        if let Some(sm) = state_machines_by_enum.get(enum_name) {
            if let Some(from_state) = parse_enum_member_value(cmp_value) {
                verify_branch_target_transition_from_status(
                    def_name,
                    pipeline_idx,
                    step_idx,
                    &then_target,
                    enum_field,
                    from_state,
                    sm,
                    executable_by_name,
                )?;
            }
        }
    }

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

fn verify_flow_match_step(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    step: &HirStep,
    executable_names: &HashSet<String>,
    executable_by_name: &HashMap<String, &HirExecutable>,
    input_type: Option<&TypeRef>,
    struct_field_types_by_name: &HashMap<String, HashMap<String, TypeRef>>,
    enum_members_by_name: &HashMap<String, HashSet<String>>,
    state_machines_by_enum: &HashMap<String, HirStateMachineDef>,
) -> Result<(), GraphemeError> {
    let Some(module_raw) = step.module.as_deref() else {
        return Ok(());
    };

    if !module_raw.eq_ignore_ascii_case("flow") || step.op != "match" {
        return Ok(());
    }

    let args = step.args.as_object().ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: args for 'flow.match' must be an object",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    for arg_name in args.keys() {
        if arg_name != "field" && arg_name != "cases" && arg_name != "default" && arg_name != "max_depth" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: unknown arg '{}' for flow.match",
                def_name, pipeline_idx, step_idx, arg_name
            )));
        }
    }

    let field = args.get("field").and_then(|v| v.as_str()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.match field must be a string",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    if field.trim().is_empty() {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.match field cannot be empty",
            def_name, pipeline_idx, step_idx
        )));
    }

    let cases = args.get("cases").and_then(|v| v.as_array()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.match cases must be an array",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    if cases.is_empty() {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.match requires at least one case",
            def_name, pipeline_idx, step_idx
        )));
    }

    let default_target = args.get("default").ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: flow.match requires default target",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    let enum_name = resolve_enum_name_for_when_field(
        field,
        input_type,
        struct_field_types_by_name,
        enum_members_by_name,
    );
    let enum_field = field.to_string();

    for (case_idx, case) in cases.iter().enumerate() {
        let case_obj = case.as_object().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.match case {} must be an object",
                def_name, pipeline_idx, step_idx, case_idx
            ))
        })?;

        for key in case_obj.keys() {
            if key != "eq" && key != "then" {
                return Err(GraphemeError::TypeError(format!(
                    "definition '{}', pipeline {}, step {}: flow.match case {} unknown key '{}'",
                    def_name, pipeline_idx, step_idx, case_idx, key
                )));
            }
        }

        let eq_value = case_obj.get("eq").ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.match case {} missing eq",
                def_name, pipeline_idx, step_idx, case_idx
            ))
        })?;

        let transition_from_state = if let Some(enum_name) = enum_name.as_ref() {
            if is_variable_placeholder(eq_value) {
                None
            } else {
                let member = parse_enum_member_value(eq_value).ok_or_else(|| {
                    GraphemeError::TypeError(format!(
                        "definition '{}', pipeline {}, step {}: enum field '{}' match case {} must use a literal member",
                        def_name, pipeline_idx, step_idx, field, case_idx
                    ))
                })?;

                let members = enum_members_by_name
                    .get(enum_name)
                    .expect("enum name resolved from existing enum map");
                if !members.contains(member) {
                    return Err(GraphemeError::TypeError(format!(
                        "definition '{}', pipeline {}, step {}: unknown enum member '{}' for enum '{}'",
                        def_name, pipeline_idx, step_idx, member, enum_name
                    )));
                }

                Some(member.to_string())
            }
        } else {
            None
        };

        let then_target = case_obj.get("then").ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.match case {} missing then target",
                def_name, pipeline_idx, step_idx, case_idx
            ))
        })?;

        verify_match_target_value(
            def_name,
            pipeline_idx,
            step_idx,
            then_target,
            executable_names,
            executable_by_name,
            transition_from_state.as_deref(),
            enum_name.as_deref(),
            &enum_field,
            state_machines_by_enum,
        )?;
    }

    verify_match_target_value(
        def_name,
        pipeline_idx,
        step_idx,
        default_target,
        executable_names,
        executable_by_name,
        None,
        enum_name.as_deref(),
        &enum_field,
        state_machines_by_enum,
    )?;

    if let Some(max_depth) = args.get("max_depth") {
        let value = max_depth.as_i64().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.match max_depth must be an integer",
                def_name, pipeline_idx, step_idx
            ))
        })?;
        if value < 1 {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.match max_depth must be >= 1",
                def_name, pipeline_idx, step_idx
            )));
        }
    }

    Ok(())
}

fn verify_match_target_value(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    value: &JsonValue,
    executable_names: &HashSet<String>,
    executable_by_name: &HashMap<String, &HirExecutable>,
    transition_from_state: Option<&str>,
    enum_name: Option<&str>,
    enum_field: &str,
    state_machines_by_enum: &HashMap<String, HirStateMachineDef>,
) -> Result<(), GraphemeError> {
    if let Some(target) = parse_branch_target(Some(value)) {
        verify_match_target_exists(def_name, pipeline_idx, step_idx, &target, executable_names)?;

        if let (Some(from_state), Some(enum_name)) = (transition_from_state, enum_name) {
            if let Some(sm) = state_machines_by_enum.get(enum_name) {
                verify_branch_target_transition_from_status(
                    def_name,
                    pipeline_idx,
                    step_idx,
                    &target,
                    enum_field,
                    from_state,
                    sm,
                    executable_by_name,
                )?;
            }
        }

        return Ok(());
    }

    let nested = value
        .as_object()
        .and_then(|obj| obj.get("$match"))
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: flow.match target must be a symbol target or nested match",
                def_name, pipeline_idx, step_idx
            ))
        })?;

    for key in nested.keys() {
        if key != "field" && key != "cases" && key != "default" {
            return Err(GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: nested match unknown key '{}'",
                def_name, pipeline_idx, step_idx, key
            )));
        }
    }

    let field = nested.get("field").and_then(|v| v.as_str()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: nested match field must be a string",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    if field.trim().is_empty() {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: nested match field cannot be empty",
            def_name, pipeline_idx, step_idx
        )));
    }

    let cases = nested.get("cases").and_then(|v| v.as_array()).ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: nested match cases must be an array",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    if cases.is_empty() {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: nested match requires at least one case",
            def_name, pipeline_idx, step_idx
        )));
    }

    for (case_idx, case) in cases.iter().enumerate() {
        let case_obj = case.as_object().ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: nested match case {} must be an object",
                def_name, pipeline_idx, step_idx, case_idx
            ))
        })?;

        let then_target = case_obj.get("then").ok_or_else(|| {
            GraphemeError::TypeError(format!(
                "definition '{}', pipeline {}, step {}: nested match case {} missing then target",
                def_name, pipeline_idx, step_idx, case_idx
            ))
        })?;

        verify_match_target_value(
            def_name,
            pipeline_idx,
            step_idx,
            then_target,
            executable_names,
            executable_by_name,
            transition_from_state,
            enum_name,
            enum_field,
            state_machines_by_enum,
        )?;
    }

    let default_target = nested.get("default").ok_or_else(|| {
        GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: nested match missing default target",
            def_name, pipeline_idx, step_idx
        ))
    })?;

    verify_match_target_value(
        def_name,
        pipeline_idx,
        step_idx,
        default_target,
        executable_names,
        executable_by_name,
        transition_from_state,
        enum_name,
        enum_field,
        state_machines_by_enum,
    )
}

fn verify_match_target_exists(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
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
        "definition '{}', pipeline {}, step {}: flow.match target '{}' not found",
        def_name, pipeline_idx, step_idx, target
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

fn resolve_enum_name_for_when_field(
    field: &str,
    input_type: Option<&TypeRef>,
    struct_field_types_by_name: &HashMap<String, HashMap<String, TypeRef>>,
    enum_members_by_name: &HashMap<String, HashSet<String>>,
) -> Option<String> {
    let TypeRef::Named(input_type_name, _) = input_type? else {
        return None;
    };

    let field_types = struct_field_types_by_name.get(input_type_name)?;
    let root_field = field.split('.').next().unwrap_or(field);
    let TypeRef::Named(field_type_name, _) = field_types.get(root_field)? else {
        return None;
    };

    if enum_members_by_name.contains_key(field_type_name) {
        Some(field_type_name.clone())
    } else {
        None
    }
}

fn parse_enum_member_value(value: &JsonValue) -> Option<&str> {
    if let Some(member) = value.as_str() {
        return Some(member);
    }

    value
        .as_object()
        .and_then(|map| map.get("$symbol"))
        .and_then(|v| v.as_str())
}

fn verify_branch_target_transition_from_status(
    def_name: &str,
    pipeline_idx: usize,
    step_idx: usize,
    target: &str,
    enum_field: &str,
    from_state: &str,
    state_machine: &HirStateMachineDef,
    executable_by_name: &HashMap<String, &HirExecutable>,
) -> Result<(), GraphemeError> {
    if target == "$return" {
        return Ok(());
    }

    let Some(target_exec) = executable_by_name.get(target) else {
        // existence is validated elsewhere
        return Ok(());
    };

    let Some(to_state) = first_literal_field_assignment(target_exec, enum_field) else {
        // No literal assignment in target, cannot statically verify path transition.
        return Ok(());
    };

    if to_state == from_state {
        return Ok(());
    }

    if state_machine.terminals.iter().any(|t| t == from_state) {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: branch then target '{}' makes invalid transition for '{}' from terminal '{}' in state_machine '{}'",
            def_name, pipeline_idx, step_idx, target, enum_field, from_state, state_machine.name
        )));
    }

    let allowed = state_machine
        .transitions
        .iter()
        .any(|t| t.from == from_state && t.to == to_state);
    if !allowed {
        return Err(GraphemeError::TypeError(format!(
            "definition '{}', pipeline {}, step {}: branch then target '{}' makes invalid transition for '{}' from '{}' to '{}' in state_machine '{}'",
            def_name, pipeline_idx, step_idx, target, enum_field, from_state, to_state, state_machine.name
        )));
    }

    Ok(())
}

fn first_literal_field_assignment<'a>(executable: &'a HirExecutable, field: &str) -> Option<&'a str> {
    for pipeline in &executable.pipelines {
        for step in &pipeline.steps {
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

            if let Some(value) = literal_patch.get(field) {
                if let Some(member) = parse_literal_member(value) {
                    return Some(member);
                }
            }
        }
    }

    None
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
