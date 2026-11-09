//! Binds `state` stores (RFC-0007 Part A) to the executables declared `on` them.
//!
//! Inside a store executable, bare `$field` reads resolve to the store's current
//! revision (`$state.field`), derives are inlined, `apply { ... }` writes stored
//! fields and `transition` writes lifecycle fields. Both lower to `set_fields`.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value as JsonValue};

use crate::ast::{Definition, Program, StoreDef, StoreFieldKind, TypeRef};
use crate::error::GraphemeError;
use crate::hir::{
    HirEnumDef, HirExecutable, HirExecutableKind, HirStateMachineDef, HirStep, HirStore,
    HirStoreField, HirStructDef, HirStructField, value_to_json,
};
use crate::parser::{BARE_FIELD_PREFIX, STORE_APPLY_OP, TRANSITION_OP};

pub(crate) fn bind_stores(
    program: &Program,
    struct_defs: &mut Vec<HirStructDef>,
    enum_defs: &[HirEnumDef],
    state_machines: &[HirStateMachineDef],
    executables: &mut [HirExecutable],
) -> Result<Vec<HirStore>, GraphemeError> {
    let mut stores = Vec::new();
    for def in &program.definitions {
        if let Definition::Store(store) = def {
            let lowered = lower_store(store, struct_defs, enum_defs, state_machines)?;
            if stores.iter().any(|s: &HirStore| s.name == lowered.name) {
                return Err(type_error(format!(
                    "state '{}' is declared more than once",
                    lowered.name
                )));
            }
            stores.push(lowered);
        }
    }
    for store in &stores {
        struct_defs.push(HirStructDef {
            name: store.name.clone(),
            fields: store
                .fields
                .iter()
                .filter(|f| f.kind != StoreFieldKind::Derive)
                .map(|f| HirStructField {
                    name: f.name.clone(),
                    type_ref: f.type_ref.clone(),
                    optional: false,
                })
                .collect(),
        });
    }

    let read_only = bind_executables(&stores, executables)?;

    for exec in executables.iter_mut() {
        let is_read_only = read_only.contains(&exec.name);
        let store = match &exec.store {
            Some(name) => stores.iter().find(|s| &s.name == name),
            None => None,
        };
        for pipeline in &mut exec.pipelines {
            for step in &mut pipeline.steps {
                match store {
                    Some(store) => lower_store_step(&exec.name, store, is_read_only, step)?,
                    None => lower_plain_step(&exec.name, step)?,
                }
            }
        }
    }
    Ok(stores)
}

/// Store mutations and queries are invoked through a store session, not as the file's entrypoint.
pub(crate) fn is_store_bound(
    program: &Program,
    signature: Option<&crate::ast::ExecutableSignature>,
) -> bool {
    let Some(TypeRef::Named(input, _)) = signature.map(|s| &s.input) else {
        return false;
    };
    program
        .definitions
        .iter()
        .any(|d| matches!(d, Definition::Store(s) if &s.name == input))
}

fn type_error(message: String) -> GraphemeError {
    GraphemeError::TypeError(message)
}

fn lower_store(
    store: &StoreDef,
    struct_defs: &[HirStructDef],
    enum_defs: &[HirEnumDef],
    state_machines: &[HirStateMachineDef],
) -> Result<HirStore, GraphemeError> {
    if let Some(lifetime) = &store.lifetime {
        return Err(type_error(format!(
            "state '{}': '{lifetime} state' lifetimes are not supported yet",
            store.name
        )));
    }
    if let Some(owner) = &store.owner {
        return Err(type_error(format!(
            "state '{}': 'on {owner}' needs views, which are not supported yet",
            store.name
        )));
    }
    if struct_defs.iter().any(|s| s.name == store.name)
        || enum_defs.iter().any(|e| e.name == store.name)
    {
        return Err(type_error(format!(
            "state '{}' has the same name as a struct or enum",
            store.name
        )));
    }

    let mut seen = HashSet::new();
    let mut fields = Vec::new();
    for field in &store.fields {
        if !seen.insert(field.name.as_str()) {
            return Err(type_error(format!(
                "state '{}': field '{}' is declared more than once",
                store.name, field.name
            )));
        }
        let value = value_to_json(&field.value);
        if field.kind != StoreFieldKind::Derive && reads_values(&value) {
            return Err(type_error(format!(
                "state '{}': default of field '{}' may not read fields or variables",
                store.name, field.name
            )));
        }
        if let (
            TypeRef::Named(type_name, _),
            StoreFieldKind::Plain | StoreFieldKind::Const | StoreFieldKind::Transient,
        ) = (&field.type_ref, field.kind)
        {
            let member = match &value {
                JsonValue::String(s) => Some(s.as_str()),
                JsonValue::Object(map) => map.get("$symbol").and_then(JsonValue::as_str),
                _ => None,
            };
            let def = enum_defs.iter().find(|e| &e.name == type_name);
            if let (Some(def), Some(member)) = (def, member)
                && !def.members.iter().any(|m| m == member)
            {
                return Err(type_error(format!(
                    "state '{}': default '{member}' of field '{}' is not a member of enum '{type_name}'",
                    store.name, field.name
                )));
            }
        }
        let lifecycle = match &field.type_ref {
            TypeRef::Named(type_name, _) if field.kind != StoreFieldKind::Derive => {
                let machines = state_machines
                    .iter()
                    .filter(|m| &m.enum_name == type_name)
                    .collect::<Vec<_>>();
                match machines.as_slice() {
                    [] => None,
                    [machine] => Some(machine.name.clone()),
                    _ => {
                        return Err(type_error(format!(
                            "state '{}': field '{}' has several state machines on '{type_name}'",
                            store.name, field.name
                        )));
                    }
                }
            }
            _ => None,
        };
        if lifecycle.is_some() && field.kind == StoreFieldKind::Const {
            return Err(type_error(format!(
                "state '{}': lifecycle field '{}' cannot be const",
                store.name, field.name
            )));
        }
        fields.push(HirStoreField {
            name: field.name.clone(),
            kind: field.kind,
            type_ref: field.type_ref.clone(),
            value,
            lifecycle,
        });
    }

    let derives = fields
        .iter()
        .filter(|f| f.kind == StoreFieldKind::Derive)
        .map(|f| (f.name.clone(), f.value.clone()))
        .collect::<HashMap<_, _>>();
    let mut resolved = HashMap::new();
    for name in derives.keys() {
        resolve_derive(
            &store.name,
            &fields,
            &derives,
            name,
            &mut Vec::new(),
            &mut resolved,
        )?;
    }
    for field in &mut fields {
        if let Some(value) = resolved.remove(&field.name) {
            field.value = value;
        }
    }

    Ok(HirStore {
        name: store.name.clone(),
        fields,
    })
}

fn resolve_derive(
    store: &str,
    fields: &[HirStoreField],
    derives: &HashMap<String, JsonValue>,
    name: &str,
    stack: &mut Vec<String>,
    resolved: &mut HashMap<String, JsonValue>,
) -> Result<JsonValue, GraphemeError> {
    if let Some(value) = resolved.get(name) {
        return Ok(value.clone());
    }
    if stack.iter().any(|s| s == name) {
        stack.push(name.to_string());
        return Err(type_error(format!(
            "state '{store}': derive cycle {}",
            stack.join(" -> ")
        )));
    }
    stack.push(name.to_string());
    let mut value = derives[name].clone();
    let mut error = None;
    rewrite_vars(&mut value, &mut |path| {
        let (head, rest) = split_path(path);
        match fields.iter().find(|f| f.name == head) {
            Some(f) if f.kind == StoreFieldKind::Derive => {
                if rest.is_some() {
                    error.get_or_insert_with(|| {
                        format!("state '{store}': derive '{name}' reads '${path}'; member access on a derive is not supported yet")
                    });
                    return None;
                }
                match resolve_derive(store, fields, derives, head, stack, resolved) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        error.get_or_insert_with(|| e.to_string());
                        None
                    }
                }
            }
            Some(_) => Some(state_var(path)),
            None => {
                error.get_or_insert_with(|| {
                    format!("state '{store}': derive '{name}' reads '${path}', which is not a field of the store")
                });
                None
            }
        }
    });
    if let Some(message) = error {
        return Err(type_error(
            message.trim_start_matches("type error: ").to_string(),
        ));
    }
    if contains_interpolation(&value) {
        return Err(type_error(format!(
            "state '{store}': derive '{name}' may not use string interpolation yet"
        )));
    }
    stack.pop();
    resolved.insert(name.to_string(), value.clone());
    Ok(value)
}

fn store_input_name(exec: &HirExecutable) -> Option<&str> {
    match exec.input_type.as_ref()? {
        TypeRef::Named(name, _) => Some(name),
        _ => None,
    }
}

fn is_synthetic(exec: &HirExecutable) -> bool {
    matches!(exec.kind, HirExecutableKind::Fragment)
        && matches!(
            exec.input_type,
            Some(TypeRef::Scalar(crate::ast::ScalarKind::Any, _))
        )
        && exec.name.starts_with("__")
}

/// Binds executables declared `on` a store and returns the read-only ones
/// (store queries and the fragments they call).
fn bind_executables(
    stores: &[HirStore],
    executables: &mut [HirExecutable],
) -> Result<HashSet<String>, GraphemeError> {
    let mut queue = Vec::new();
    let mut read_only = HashSet::new();
    for (idx, exec) in executables.iter_mut().enumerate() {
        let Some(store) =
            store_input_name(exec).and_then(|name| stores.iter().find(|s| s.name == name))
        else {
            continue;
        };
        match exec.kind {
            HirExecutableKind::Mutation | HirExecutableKind::Fragment => {}
            HirExecutableKind::Query => {
                read_only.insert(exec.name.clone());
            }
            _ => {
                return Err(type_error(format!(
                    "definition '{}': only queries and mutations can be declared on state '{}' so far",
                    exec.name, store.name
                )));
            }
        }
        for param in &exec.params {
            if store.fields.iter().any(|f| f.name == param.name) {
                return Err(type_error(format!(
                    "definition '{}': parameter '${}' has the same name as a field of state '{}'",
                    exec.name, param.name, store.name
                )));
            }
        }
        exec.store = Some(store.name.clone());
        if matches!(exec.kind, HirExecutableKind::Mutation) && exec.output_type.is_none() {
            exec.output_type = Some(TypeRef::Named(store.name.clone(), true));
        }
        queue.push(idx);
    }

    let names = executables
        .iter()
        .enumerate()
        .map(|(i, e)| (e.name.clone(), i))
        .collect::<HashMap<_, _>>();
    while let Some(idx) = queue.pop() {
        let store = executables[idx].store.clone().unwrap_or_default();
        let caller = executables[idx].name.clone();
        let caller_read_only = read_only.contains(&caller);
        let mut targets = Vec::new();
        for step in executables[idx]
            .pipelines
            .iter()
            .flat_map(|p| p.steps.iter())
        {
            match step.module.as_deref() {
                Some("call") => targets.push(step.op.clone()),
                Some("flow") => collect_strings(&step.args, &mut targets),
                _ => {}
            }
        }
        for target in targets {
            let Some(&t) = names.get(&target) else {
                continue;
            };
            let callee = &mut executables[t];
            if caller_read_only
                && !is_synthetic(callee)
                && !matches!(callee.kind, HirExecutableKind::Query)
            {
                return Err(type_error(format!(
                    "query '{caller}' on state '{store}' calls '{target}'; store queries are read-only and can only call queries"
                )));
            }
            if caller_read_only && is_synthetic(callee) && read_only.insert(target.clone()) {
                queue.push(t);
            }
            match &callee.store {
                Some(s) if *s == store => {}
                Some(s) => {
                    return Err(type_error(format!(
                        "definition '{caller}' on state '{store}' calls '{target}' on state '{s}'; a transaction writes one store"
                    )));
                }
                None if is_synthetic(callee) => {
                    callee.store = Some(store.clone());
                    queue.push(t);
                }
                None => {
                    return Err(type_error(format!(
                        "definition '{caller}' on state '{store}' calls '{target}', which is not declared on '{store}'"
                    )));
                }
            }
        }
    }
    Ok(read_only)
}

fn lower_store_step(
    exec: &str,
    store: &HirStore,
    read_only: bool,
    step: &mut HirStep,
) -> Result<(), GraphemeError> {
    let field = |name: &str| find_field(store, name);
    let context = format!("definition '{exec}' on state '{}'", store.name);
    if read_only
        && step.module.as_deref() == Some("core")
        && matches!(step.op.as_str(), STORE_APPLY_OP | TRANSITION_OP)
    {
        return Err(type_error(format!(
            "{context}: store queries are read-only; write fields from a mutation"
        )));
    }

    let mut error = None;
    rewrite_vars(&mut step.args, &mut |path| {
        let (head, rest) = split_path(path);
        match field(head) {
            Some(f) if f.kind == StoreFieldKind::Derive => {
                if rest.is_some() {
                    error.get_or_insert_with(|| {
                        format!("{context}: member access on derive '{head}' is not supported yet")
                    });
                    None
                } else {
                    Some(f.value.clone())
                }
            }
            Some(_) => Some(state_var(path)),
            None => None,
        }
    });
    if let Some(message) = error {
        return Err(type_error(message));
    }
    rewrite_strings(&mut step.args, &mut |s| {
        rewrite_bare(s, &context, store).and_then(|s| rewrite_interpolation(&s, &context, store))
    })?;

    match (step.module.as_deref(), step.op.as_str()) {
        (Some("core"), STORE_APPLY_OP) => {
            for name in written_fields(step) {
                match field(&name) {
                    None => {
                        return Err(type_error(format!(
                            "{context}: apply writes '{name}', which is not a field of the store"
                        )));
                    }
                    Some(f) if f.kind == StoreFieldKind::Const => {
                        return Err(type_error(format!(
                            "{context}: apply cannot write const field '{name}'"
                        )));
                    }
                    Some(f) if f.kind == StoreFieldKind::Derive => {
                        return Err(type_error(format!(
                            "{context}: apply cannot write derived field '{name}'"
                        )));
                    }
                    Some(f) if f.lifecycle.is_some() => {
                        return Err(type_error(format!(
                            "{context}: '{name}' is a lifecycle field; write it with transition"
                        )));
                    }
                    Some(_) => {}
                }
            }
            lower_to_set_fields(step);
        }
        (Some("core"), TRANSITION_OP) => {
            for name in written_fields(step).into_iter().take(1) {
                match field(&name) {
                    Some(f) if f.lifecycle.is_some() => {}
                    Some(_) => {
                        return Err(type_error(format!(
                            "{context}: transition needs a lifecycle field; '{name}' has no state machine"
                        )));
                    }
                    None => {
                        return Err(type_error(format!(
                            "{context}: transition writes '{name}', which is not a field of the store"
                        )));
                    }
                }
            }
            lower_to_set_fields(step);
        }
        (Some("core"), _) if read_only => {}
        (Some("core"), op) => {
            return Err(type_error(format!(
                "{context}: 'core.{op}' cannot run in a store mutation; write fields with apply {{ ... }} or transition"
            )));
        }
        (Some("call"), _) | (Some("flow"), _) => {}
        (module, op) => {
            return Err(type_error(format!(
                "{context}: '{}.{op}' cannot run in a store mutation yet; store mutations only apply, transition, branch and call",
                module.unwrap_or("?")
            )));
        }
    }
    Ok(())
}

fn lower_plain_step(exec: &str, step: &mut HirStep) -> Result<(), GraphemeError> {
    if step.module.as_deref() != Some("core") {
        reject_bare(exec, &step.args)?;
        return Ok(());
    }
    match step.op.as_str() {
        STORE_APPLY_OP => Err(type_error(format!(
            "definition '{exec}': apply {{ ... }} needs an executable declared on a state store (mutation {exec} on Store); use apply state {{ ... }} for the state lane"
        ))),
        TRANSITION_OP => {
            reject_bare(exec, &step.args)?;
            lower_to_set_fields(step);
            Ok(())
        }
        _ => reject_bare(exec, &step.args),
    }
}

fn lower_to_set_fields(step: &mut HirStep) {
    step.op = "set_fields".to_string();
    step.capability = grapheme_artifact::Capability::from_module_op("core", "set_fields");
}

fn reject_bare(exec: &str, args: &JsonValue) -> Result<(), GraphemeError> {
    let mut found = None;
    visit_strings(args, &mut |s| {
        if let Some(name) = s.strip_prefix(BARE_FIELD_PREFIX) {
            found.get_or_insert_with(|| name.to_string());
        }
    });
    match found {
        Some(name) => Err(GraphemeError::ParseError(format!(
            "definition '{exec}': '${name}' must reference $state.<field> or $item.<field> outside a store executable"
        ))),
        None => Ok(()),
    }
}

fn written_fields(step: &HirStep) -> Vec<String> {
    step.args
        .get("fields")
        .and_then(JsonValue::as_object)
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

fn rewrite_bare(s: &str, context: &str, store: &HirStore) -> Result<String, GraphemeError> {
    let Some(path) = s.strip_prefix(BARE_FIELD_PREFIX) else {
        return Ok(s.to_string());
    };
    let (head, _) = split_path(path);
    match find_field(store, head) {
        Some(f) if f.kind == StoreFieldKind::Derive => Err(type_error(format!(
            "{context}: derived field '{head}' cannot be used here yet"
        ))),
        Some(_) => Ok(path.to_string()),
        None => Err(type_error(format!(
            "{context}: '${path}' is not a field of the store"
        ))),
    }
}

/// Rewrites `{$field...}` interpolations to `{$state.field...}`.
fn rewrite_interpolation(
    s: &str,
    context: &str,
    store: &HirStore,
) -> Result<String, GraphemeError> {
    if !s.contains("{$") {
        return Ok(s.to_string());
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find("{$") {
        out.push_str(&rest[..pos + 2]);
        rest = &rest[pos + 2..];
        let end = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        match find_field(store, &rest[..end]) {
            Some(f) if f.kind == StoreFieldKind::Derive => {
                return Err(type_error(format!(
                    "{context}: derived field '{}' cannot be interpolated into a string yet",
                    f.name
                )));
            }
            Some(_) => out.push_str("state."),
            None => {}
        }
    }
    out.push_str(rest);
    Ok(out)
}

pub(crate) fn find_field<'a>(store: &'a HirStore, name: &str) -> Option<&'a HirStoreField> {
    store.fields.iter().find(|f| f.name == name)
}

pub(crate) fn split_path(path: &str) -> (&str, Option<&str>) {
    match path.split_once('.') {
        Some((head, rest)) => (head, Some(rest)),
        None => (path, None),
    }
}

fn state_var(path: &str) -> JsonValue {
    let mut object = Map::new();
    object.insert("$var".to_string(), JsonValue::from(format!("state.{path}")));
    JsonValue::Object(object)
}

fn var_path(value: &JsonValue) -> Option<&str> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object.get("$var")?.as_str()
}

/// Replaces every `{"$var": path}` for which `f` returns a value.
pub(crate) fn rewrite_vars(value: &mut JsonValue, f: &mut dyn FnMut(&str) -> Option<JsonValue>) {
    if let Some(path) = var_path(value).map(str::to_string) {
        if let Some(replacement) = f(&path) {
            *value = replacement;
        }
        return;
    }
    match value {
        JsonValue::Array(items) => items.iter_mut().for_each(|v| rewrite_vars(v, f)),
        JsonValue::Object(map) => map.values_mut().for_each(|v| rewrite_vars(v, f)),
        _ => {}
    }
}

/// Rewrites every string value and object key.
pub(crate) fn rewrite_strings(
    value: &mut JsonValue,
    f: &mut dyn FnMut(&str) -> Result<String, GraphemeError>,
) -> Result<(), GraphemeError> {
    match value {
        JsonValue::String(s) => *s = f(s)?,
        JsonValue::Array(items) => {
            for item in items {
                rewrite_strings(item, f)?;
            }
        }
        JsonValue::Object(map) => {
            let entries = std::mem::take(map);
            for (key, mut v) in entries {
                rewrite_strings(&mut v, f)?;
                map.insert(f(&key)?, v);
            }
        }
        _ => {}
    }
    Ok(())
}

fn visit_strings(value: &JsonValue, f: &mut dyn FnMut(&str)) {
    match value {
        JsonValue::String(s) => f(s),
        JsonValue::Array(items) => items.iter().for_each(|v| visit_strings(v, f)),
        JsonValue::Object(map) => {
            for (k, v) in map {
                f(k);
                visit_strings(v, f);
            }
        }
        _ => {}
    }
}

fn collect_strings(value: &JsonValue, out: &mut Vec<String>) {
    visit_strings(value, &mut |s| out.push(s.to_string()));
}

fn reads_values(value: &JsonValue) -> bool {
    if var_path(value).is_some() {
        return true;
    }
    let mut interpolates = false;
    visit_strings(value, &mut |s| interpolates |= s.contains("{$"));
    interpolates
        || match value {
            JsonValue::Array(items) => items.iter().any(reads_values),
            JsonValue::Object(map) => map.values().any(reads_values),
            _ => false,
        }
}

fn contains_interpolation(value: &JsonValue) -> bool {
    let mut found = false;
    visit_strings(value, &mut |s| found |= s.contains("{$"));
    found
}
