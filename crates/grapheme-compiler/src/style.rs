//! `style { … }` on a view element (RFC-0007 C.5).
//!
//! Style is renderer-free data. A property's expected type decides what a bare
//! identifier means: `fill` is a size in `width`, and a theme role in `fill`.
//! The compiler records the choice (`"fill"`, `{ "$role": "bg" }`) and does
//! not know how a renderer paints it.
//!
//! Properties: `width`, `height` (`fill`, `hug`, or a length in px; `full` is
//! `fill`), `min`, `max` (`{ width, height }` in px), `padding` (a length, or
//! `{ x, y, top, right, bottom, left }`), `gap`, `align` (`start`, `center`,
//! `end`, `stretch`), `justify` (`start`, `center`, `end`, `space_between`,
//! `space_around`, `space_evenly`), `fill` and `color` (a theme role or
//! `#rgb` / `#rrggbb` / `#rrggbbaa`), `size` (font size in px) and `weight`
//! (`regular`, `medium`, `semibold`, `bold`, or an integer from 100 to 900).
//!
//! Theme roles: `bg`, `text`, `text_secondary`, `accent`, `separator`,
//! `selection`, `transparent`, `control_off`, `hover`, `press`, `pressed`,
//! `disabled`. Interaction states `hover`, `pressed`, `focused` and `disabled`
//! are nested blocks, not properties.

use serde_json::{Map, Value as JsonValue};

use crate::ast::{Value, ValueExpr, ViewStyle, ViewStyleItem, ViewStyleState};
use crate::error::GraphemeError;

/// A style failure. [`StyleError::Value`] is already a view error from
/// lowering an expression. [`StyleError::Style`] is a style message the
/// caller prefixes with the element.
pub(crate) enum StyleError {
    Style(String),
    Value(GraphemeError),
}

#[derive(Clone, Copy)]
enum Slot {
    Size,
    Length,
    Padding,
    Edges,
    Align,
    Justify,
    Color,
    Weight,
}

const PROPERTIES: &[&str] = &[
    "width", "height", "min", "max", "padding", "gap", "align", "justify", "fill", "color", "size",
    "weight",
];

const ROLES: &[&str] = &[
    "bg",
    "text",
    "text_secondary",
    "accent",
    "separator",
    "selection",
    "transparent",
    "control_off",
    "hover",
    "press",
    "pressed",
    "disabled",
];

const ALIGN: &[&str] = &["start", "center", "end", "stretch"];
const JUSTIFY: &[&str] = &[
    "start",
    "center",
    "end",
    "space_between",
    "space_around",
    "space_evenly",
];
const WEIGHTS: &[&str] = &["regular", "medium", "semibold", "bold"];

pub(crate) fn lower_style_block(
    style: &ViewStyle,
    mut lower_value: impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
    mut static_type: impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<JsonValue, StyleError> {
    let mut out = Map::new();
    let mut seen_props = Vec::new();
    let mut seen_states = Vec::new();
    for item in &style.items {
        match item {
            ViewStyleItem::Prop(name, value) => {
                insert_prop(
                    &mut out,
                    &mut seen_props,
                    name,
                    value,
                    false,
                    &mut lower_value,
                    &mut static_type,
                )?;
            }
            ViewStyleItem::State(state) => {
                insert_state(
                    &mut out,
                    &mut seen_states,
                    state,
                    &mut lower_value,
                    &mut static_type,
                )?;
            }
        }
    }
    Ok(JsonValue::Object(out))
}

fn insert_state(
    out: &mut Map<String, JsonValue>,
    seen: &mut Vec<String>,
    state: &ViewStyleState,
    lower_value: &mut impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
    static_type: &mut impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<(), StyleError> {
    if !is_state(&state.name) {
        return Err(style(format!(
            "style has no interaction state '{}'; states are hover, pressed, focused, disabled",
            state.name
        )));
    }
    if seen.iter().any(|name| name == &state.name) {
        return Err(style(format!(
            "interaction state '{}' is given more than once",
            state.name
        )));
    }
    seen.push(state.name.clone());
    let mut body = Map::new();
    let mut seen_props = Vec::new();
    for (name, value) in &state.props {
        insert_prop(
            &mut body,
            &mut seen_props,
            name,
            value,
            true,
            lower_value,
            static_type,
        )?;
    }
    if body.is_empty() {
        return Err(style(format!(
            "interaction state '{}' has no properties",
            state.name
        )));
    }
    out.insert(state.name.clone(), JsonValue::Object(body));
    Ok(())
}

fn insert_prop(
    out: &mut Map<String, JsonValue>,
    seen: &mut Vec<String>,
    name: &str,
    value: &Value,
    in_state: bool,
    lower_value: &mut impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
    static_type: &mut impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<(), StyleError> {
    if in_state && is_state(name) {
        return Err(style(format!(
            "interaction state '{name}' cannot be nested; write `{name} {{ … }}` on the element's style"
        )));
    }
    let Some(slot) = slot_of(name) else {
        if is_state(name) {
            return Err(style(format!(
                "'{name}' is an interaction state; write `{name} {{ … }}`, not `{name}: …`"
            )));
        }
        return Err(style(format!(
            "style has no property '{name}'; properties are: {}",
            PROPERTIES.join(", ")
        )));
    };
    if seen.iter().any(|seen| seen == name) {
        return Err(style(format!(
            "style property '{name}' is given more than once"
        )));
    }
    seen.push(name.to_string());
    let lowered = lower_slot(name, slot, value, lower_value, static_type)?;
    out.insert(name.to_string(), lowered);
    Ok(())
}

fn slot_of(name: &str) -> Option<Slot> {
    Some(match name {
        "width" | "height" => Slot::Size,
        "gap" | "size" => Slot::Length,
        "padding" => Slot::Padding,
        "min" | "max" => Slot::Edges,
        "align" => Slot::Align,
        "justify" => Slot::Justify,
        "fill" | "color" => Slot::Color,
        "weight" => Slot::Weight,
        _ => return None,
    })
}

fn is_state(name: &str) -> bool {
    matches!(name, "hover" | "pressed" | "focused" | "disabled")
}

fn lower_slot(
    prop: &str,
    slot: Slot,
    value: &Value,
    lower_value: &mut impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
    static_type: &mut impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<JsonValue, StyleError> {
    match value {
        Value::Symbol(name) => resolve_symbol(prop, slot, name),
        Value::Int(n) => accept_number(prop, slot, JsonValue::from(*n)),
        Value::Float(n) => accept_number(prop, slot, JsonValue::from(*n)),
        Value::String(text) => accept_string(prop, slot, text),
        Value::Object(fields) => lower_object(prop, slot, fields, lower_value, static_type),
        Value::Expr(expr) => match expr.as_ref() {
            ValueExpr::If(cond, then_value, else_value) => lower_if(
                prop,
                slot,
                cond,
                then_value,
                else_value,
                lower_value,
                static_type,
            ),
            _ => accept_dynamic(prop, slot, &lower_raw(value, lower_value)?, static_type),
        },
        Value::Variable(_) => {
            accept_dynamic(prop, slot, &lower_raw(value, lower_value)?, static_type)
        }
        other => Err(style(format!(
            "style property '{prop}' expects {}, found {}",
            expects(slot),
            value_kind(other)
        ))),
    }
}

fn lower_if(
    prop: &str,
    slot: Slot,
    cond: &Value,
    then_value: &Value,
    else_value: &Value,
    lower_value: &mut impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
    static_type: &mut impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<JsonValue, StyleError> {
    let cond = lower_raw(cond, lower_value)?;
    let then_value = lower_slot(prop, slot, then_value, lower_value, static_type)?;
    let else_value = lower_slot(prop, slot, else_value, lower_value, static_type)?;
    Ok(serde_json::json!({
        "$expr": { "op": "if", "args": [cond, then_value, else_value] }
    }))
}

fn lower_raw(
    value: &Value,
    lower_value: &mut impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
) -> Result<JsonValue, StyleError> {
    lower_value(value).map_err(StyleError::Value)
}

fn lower_object(
    prop: &str,
    slot: Slot,
    fields: &[(String, Value)],
    lower_value: &mut impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
    static_type: &mut impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<JsonValue, StyleError> {
    match slot {
        Slot::Padding => lower_padding(prop, fields, lower_value, static_type),
        Slot::Edges => lower_edges(prop, fields, lower_value, static_type),
        _ => Err(style(format!(
            "style property '{prop}' expects {}, found an object",
            expects(slot)
        ))),
    }
}

fn lower_padding(
    prop: &str,
    fields: &[(String, Value)],
    lower_value: &mut impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
    static_type: &mut impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<JsonValue, StyleError> {
    if fields.is_empty() {
        return Err(style(format!(
            "style property '{prop}' needs an inset; use a length or {{ x, y, top, right, bottom, left }}"
        )));
    }
    let mut raw: Vec<(String, JsonValue)> = Vec::new();
    for (name, value) in fields {
        if !matches!(
            name.as_str(),
            "x" | "y" | "top" | "right" | "bottom" | "left"
        ) {
            return Err(style(format!(
                "style property '{prop}' has no edge '{name}'; edges are x, y, top, right, bottom, left"
            )));
        }
        if raw.iter().any(|(seen, _)| seen == name) {
            return Err(style(format!(
                "style property '{prop}.{name}' is given more than once"
            )));
        }
        let length = lower_slot(
            &format!("{prop}.{name}"),
            Slot::Length,
            value,
            lower_value,
            static_type,
        )?;
        raw.push((name.clone(), length));
    }
    let mut edges: Vec<(&str, JsonValue)> = Vec::new();
    let take = |raw: &[(String, JsonValue)], name: &str| {
        raw.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    };
    if let Some(x) = take(&raw, "x") {
        edges.push(("left", x.clone()));
        edges.push(("right", x));
    }
    if let Some(y) = take(&raw, "y") {
        edges.push(("top", y.clone()));
        edges.push(("bottom", y));
    }
    for side in ["top", "right", "bottom", "left"] {
        if let Some(value) = take(&raw, side) {
            edges.retain(|(name, _)| *name != side);
            edges.push((side, value));
        }
    }
    let mut out = Map::new();
    for side in ["top", "right", "bottom", "left"] {
        if let Some((_, value)) = edges.iter().find(|(name, _)| *name == side) {
            out.insert((*side).to_string(), value.clone());
        }
    }
    Ok(JsonValue::Object(out))
}

fn lower_edges(
    prop: &str,
    fields: &[(String, Value)],
    lower_value: &mut impl FnMut(&Value) -> Result<JsonValue, GraphemeError>,
    static_type: &mut impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<JsonValue, StyleError> {
    if fields.is_empty() {
        return Err(style(format!(
            "style property '{prop}' needs width, height, or both"
        )));
    }
    let mut out = Map::new();
    for (name, value) in fields {
        if !matches!(name.as_str(), "width" | "height") {
            return Err(style(format!(
                "style property '{prop}' has no axis '{name}'; axes are width, height"
            )));
        }
        if out.contains_key(name) {
            return Err(style(format!(
                "style property '{prop}.{name}' is given more than once"
            )));
        }
        let length = lower_slot(
            &format!("{prop}.{name}"),
            Slot::Length,
            value,
            lower_value,
            static_type,
        )?;
        out.insert(name.clone(), length);
    }
    Ok(JsonValue::Object(out))
}

fn accept_dynamic(
    prop: &str,
    slot: Slot,
    value: &JsonValue,
    static_type: &mut impl FnMut(&JsonValue) -> Option<&'static str>,
) -> Result<JsonValue, StyleError> {
    if let Some(name) = value
        .as_object()
        .filter(|map| map.len() == 1)
        .and_then(|map| map.get("$symbol"))
        .and_then(JsonValue::as_str)
    {
        return resolve_symbol(prop, slot, name);
    }
    if let Some((op, args)) = value
        .as_object()
        .and_then(grapheme_artifact::expr::expr_parts)
    {
        if op == "if" && args.len() == 3 {
            let then_value = accept_dynamic(prop, slot, &args[1], static_type)?;
            let else_value = accept_dynamic(prop, slot, &args[2], static_type)?;
            return Ok(serde_json::json!({
                "$expr": { "op": "if", "args": [args[0].clone(), then_value, else_value] }
            }));
        }
        return accept_expr(prop, slot, value.clone());
    }
    match value {
        JsonValue::Number(_) => accept_number(prop, slot, value.clone()),
        JsonValue::String(text) => accept_string(prop, slot, text),
        JsonValue::Object(_) => match static_type(value) {
            Some("Int" | "Float") if matches!(slot, Slot::Size | Slot::Length | Slot::Padding) => {
                Ok(value.clone())
            }
            Some("Int") if matches!(slot, Slot::Weight) => Ok(value.clone()),
            Some("String") if matches!(slot, Slot::Color) => Ok(value.clone()),
            Some(found) => Err(style(format!(
                "style property '{prop}' expects {}, found {found}",
                expects(slot)
            ))),
            None => accept_expr(prop, slot, value.clone()),
        },
        _ => Err(style(format!(
            "style property '{prop}' expects {}, found {}",
            expects(slot),
            json_kind(value)
        ))),
    }
}

fn accept_expr(prop: &str, slot: Slot, value: JsonValue) -> Result<JsonValue, StyleError> {
    match slot {
        Slot::Size | Slot::Length | Slot::Padding | Slot::Color | Slot::Weight => Ok(value),
        Slot::Edges | Slot::Align | Slot::Justify => Err(style(format!(
            "style property '{prop}' expects {}, found an expression",
            expects(slot)
        ))),
    }
}

fn accept_number(prop: &str, slot: Slot, value: JsonValue) -> Result<JsonValue, StyleError> {
    let Some(number) = value.as_f64() else {
        return Err(style(format!(
            "style property '{prop}' expects {}, found a number",
            expects(slot)
        )));
    };
    if !number.is_finite() {
        return Err(style(format!(
            "style property '{prop}' expects a finite length"
        )));
    }
    match slot {
        Slot::Size | Slot::Length => {
            if number < 0.0 {
                return Err(style(format!(
                    "style property '{prop}' expects a length >= 0, found {number}"
                )));
            }
            Ok(value)
        }
        Slot::Padding => {
            if number < 0.0 {
                return Err(style(format!(
                    "style property '{prop}' expects a length >= 0, found {number}"
                )));
            }
            Ok(serde_json::json!({
                "top": value,
                "right": value,
                "bottom": value,
                "left": value,
            }))
        }
        Slot::Weight => match value.as_i64() {
            Some(int) if (100..=900).contains(&int) => Ok(value),
            _ => Err(style(format!(
                "style property '{prop}' expects regular, medium, semibold, bold, or an integer from 100 to 900, found {number}"
            ))),
        },
        _ => Err(style(format!(
            "style property '{prop}' expects {}, found a number",
            expects(slot)
        ))),
    }
}

fn accept_string(prop: &str, slot: Slot, text: &str) -> Result<JsonValue, StyleError> {
    match slot {
        Slot::Size => match text {
            "fill" | "hug" => Ok(JsonValue::from(text)),
            "full" => Ok(JsonValue::from("fill")),
            _ => Err(misplaced(prop, slot, text)),
        },
        Slot::Align if ALIGN.contains(&text) => Ok(JsonValue::from(text)),
        Slot::Justify if JUSTIFY.contains(&text) => Ok(JsonValue::from(text)),
        Slot::Weight if WEIGHTS.contains(&text) => Ok(JsonValue::from(text)),
        Slot::Color if is_hex(text) => Ok(JsonValue::from(text)),
        Slot::Color if ROLES.contains(&text) => role(text),
        _ => Err(misplaced(prop, slot, text)),
    }
}

fn resolve_symbol(prop: &str, slot: Slot, name: &str) -> Result<JsonValue, StyleError> {
    match slot {
        Slot::Size if matches!(name, "fill" | "hug") => Ok(JsonValue::from(name)),
        Slot::Size if name == "full" => Ok(JsonValue::from("fill")),
        Slot::Align if ALIGN.contains(&name) => Ok(JsonValue::from(name)),
        Slot::Justify if JUSTIFY.contains(&name) => Ok(JsonValue::from(name)),
        Slot::Weight if WEIGHTS.contains(&name) => Ok(JsonValue::from(name)),
        Slot::Color if ROLES.contains(&name) => role(name),
        _ => Err(misplaced(prop, slot, name)),
    }
}

fn role(name: &str) -> Result<JsonValue, StyleError> {
    Ok(serde_json::json!({ "$role": name }))
}

fn misplaced(prop: &str, slot: Slot, name: &str) -> StyleError {
    style(format!(
        "style property '{prop}' expects {}, found {}",
        expects(slot),
        found_word(name)
    ))
}

fn found_word(name: &str) -> String {
    if matches!(name, "fill" | "hug" | "full") {
        return format!("size '{name}'");
    }
    if ALIGN.contains(&name) && !JUSTIFY.contains(&name) {
        return format!("alignment '{name}'");
    }
    if JUSTIFY.contains(&name) && !ALIGN.contains(&name) {
        return format!("justification '{name}'");
    }
    if ALIGN.contains(&name) {
        return format!("alignment '{name}'");
    }
    if WEIGHTS.contains(&name) {
        return format!("font weight '{name}'");
    }
    if ROLES.contains(&name) {
        return format!("theme role '{name}'");
    }
    format!("'{name}'")
}

fn expects(slot: Slot) -> &'static str {
    match slot {
        Slot::Size => "fill, hug, or a length in px",
        Slot::Length => "a length in px",
        Slot::Padding => "a length, or { x, y, top, right, bottom, left }",
        Slot::Edges => "{ width, height } in px",
        Slot::Align => "start, center, end, or stretch",
        Slot::Justify => "start, center, end, space_between, space_around, or space_evenly",
        Slot::Color => "a theme role (bg, text, text_secondary, accent, …) or a #rrggbb color",
        Slot::Weight => "regular, medium, semibold, bold, or an integer from 100 to 900",
    }
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Int(_) | Value::Float(_) => "a number",
        Value::Bool(_) => "Bool",
        Value::Null => "null",
        Value::String(_) => "a string",
        Value::List(_) => "a list",
        Value::Object(_) => "an object",
        Value::Variable(_) => "a variable",
        Value::Symbol(_) => "an identifier",
        Value::Expr(_) => "an expression",
    }
}

fn json_kind(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "Bool",
        JsonValue::Number(_) => "a number",
        JsonValue::String(_) => "a string",
        JsonValue::Array(_) => "a list",
        JsonValue::Object(_) => "an object",
    }
}

fn is_hex(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('#') else {
        return false;
    };
    matches!(rest.len(), 3 | 6 | 8) && rest.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn style(message: String) -> StyleError {
    StyleError::Style(message)
}
