//! Template resolution for Stage B container MIR args (`$var`, `$state`, `$current`).

use serde_json::{Map, Value as JsonValue};

#[derive(Debug, Clone, Copy)]
pub struct TemplateScope<'a> {
    pub current: &'a JsonValue,
    pub state: &'a JsonValue,
    pub locals: &'a Map<String, JsonValue>,
}

pub fn args_with_pipeline_input(
    args: &JsonValue,
    input: &JsonValue,
    scope: &TemplateScope<'_>,
) -> JsonValue {
    let mut merged = match resolve_templates(args, scope) {
        JsonValue::Object(map) => map,
        _ => Map::new(),
    };
    merged.insert("__input".to_string(), input.clone());
    JsonValue::Object(merged)
}

pub fn resolve_templates(value: &JsonValue, scope: &TemplateScope<'_>) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            if let Some(var_ref) = variable_ref_from_object(map) {
                return resolve_variable_reference(var_ref, scope);
            }
            let mapped = map
                .iter()
                .map(|(k, v)| (k.clone(), resolve_templates(v, scope)))
                .collect::<Map<String, JsonValue>>();
            JsonValue::Object(mapped)
        }
        JsonValue::Array(items) => JsonValue::Array(
            items
                .iter()
                .map(|item| resolve_templates(item, scope))
                .collect(),
        ),
        JsonValue::String(s) => resolve_string_template(s, scope),
        _ => value.clone(),
    }
}

fn variable_ref_from_object(map: &Map<String, JsonValue>) -> Option<&str> {
    if map.len() != 1 {
        return None;
    }
    map.get("$var")?.as_str()
}

fn resolve_local_reference(reference: &str, scope: &TemplateScope<'_>) -> Option<JsonValue> {
    if reference == "args" {
        return Some(JsonValue::Object(scope.locals.clone()));
    }
    if let Some(path) = reference.strip_prefix("args.") {
        return Some(
            select_json_path(&JsonValue::Object(scope.locals.clone()), path)
                .cloned()
                .unwrap_or(JsonValue::Null),
        );
    }
    if let Some(value) = scope.locals.get(reference) {
        return Some(value.clone());
    }
    if let Some((name, path)) = reference.split_once('.') {
        if let Some(value) = scope.locals.get(name) {
            return Some(
                select_json_path(value, path)
                    .cloned()
                    .unwrap_or(JsonValue::Null),
            );
        }
    }
    None
}

fn resolve_variable_reference(reference: &str, scope: &TemplateScope<'_>) -> JsonValue {
    if let Some(value) = resolve_local_reference(reference, scope) {
        return value;
    }
    if reference == "state" {
        return scope.state.clone();
    }
    if reference == "current" {
        return scope.current.clone();
    }
    if let Some(path) = reference.strip_prefix("state.") {
        return select_json_path(scope.state, path)
            .cloned()
            .unwrap_or(JsonValue::Null);
    }
    if let Some(path) = reference.strip_prefix("current.") {
        return select_json_path(scope.current, path)
            .cloned()
            .unwrap_or(JsonValue::Null);
    }
    JsonValue::String(format!("${reference}"))
}

fn resolve_string_template(template: &str, scope: &TemplateScope<'_>) -> JsonValue {
    if let Some(reference) = template.strip_prefix('$') {
        if reference.chars().all(|c| is_selector_char(c) || c == '.') {
            if let Some(value) = resolve_local_reference(reference, scope) {
                return value;
            }
        }
    }
    if template == "$state" {
        return scope.state.clone();
    }
    if template == "$current" {
        return scope.current.clone();
    }
    if let Some(path) = template.strip_prefix("$state.") {
        if path.chars().all(is_selector_char) {
            return select_json_path(scope.state, path)
                .cloned()
                .unwrap_or(JsonValue::Null);
        }
    }
    if let Some(path) = template.strip_prefix("$current.") {
        if path.chars().all(is_selector_char) {
            return select_json_path(scope.current, path)
                .cloned()
                .unwrap_or(JsonValue::Null);
        }
    }
    JsonValue::String(template.to_string())
}

fn is_selector_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn select_json_path<'a>(root: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    let mut current = root;
    for part in path.split('.') {
        if part.is_empty() {
            return None;
        }
        current = current.as_object()?.get(part)?;
    }
    Some(current)
}
