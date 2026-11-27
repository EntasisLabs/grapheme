//! Lowers `view` declarations (RFC-0007 Parts C, D) to View IR nodes.
//!
//! A view is data: elements, props and event bindings, never steps. Inside
//! `view V on Store`, `$field` reads the store's current revision (derives
//! included) and lowers to `$state.field`. Every event binding is checked
//! against its target mutation's parameters.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Map, Value as JsonValue};

use crate::ast::{
    Definition, GlyphDef, Program, ScalarKind, TypeRef, Value, ViewDef, ViewElement,
};
use crate::error::GraphemeError;
use crate::hir::{HirExecutable, HirExecutableKind, HirStore, value_to_json};
use crate::stores::{find_field, rewrite_strings, rewrite_vars, split_path};
use crate::verifier::LintWarning;
use grapheme_artifact::{MirUiGlyph, MirView, MirViewBinding, MirViewFor, MirViewNode};

#[derive(Clone, Copy, PartialEq)]
enum PropType {
    /// Any scalar; rendered as text.
    Text,
    String,
    Int,
    Bool,
}

/// Catalog event: name plus payload params the host/widget supplies at invoke.
struct CatalogEvent {
    name: &'static str,
    /// Params the event supplies, as `(name, type)`. TextField `change` is
    /// `("value", "String")`.
    payload: &'static [(&'static str, &'static str)],
}

/// A built-in primitive of the GraphemeUI catalog.
struct Primitive {
    name: &'static str,
    role: &'static str,
    /// Accepts positional content, stored as the `content` prop.
    content: bool,
    props: &'static [(&'static str, PropType)],
    events: &'static [CatalogEvent],
    children: bool,
}

impl Primitive {
    fn interactive(&self) -> bool {
        !self.events.is_empty()
    }

    fn event(&self, name: &str) -> Option<&'static CatalogEvent> {
        self.events.iter().find(|e| e.name == name)
    }
}

const CATALOG: &[Primitive] = &[
    Primitive {
        name: "Column",
        role: "group",
        content: false,
        props: &[("gap", PropType::Int), ("label", PropType::String)],
        events: &[],
        children: true,
    },
    Primitive {
        name: "Row",
        role: "group",
        content: false,
        props: &[("gap", PropType::Int), ("label", PropType::String)],
        events: &[],
        children: true,
    },
    Primitive {
        name: "Text",
        role: "text",
        content: true,
        props: &[("content", PropType::Text)],
        events: &[],
        children: false,
    },
    Primitive {
        name: "Button",
        role: "button",
        content: false,
        props: &[("label", PropType::String), ("enabled", PropType::Bool)],
        events: &[CatalogEvent {
            name: "click",
            payload: &[],
        }],
        children: true,
    },
    Primitive {
        name: "TextField",
        role: "textbox",
        content: false,
        props: &[
            ("value", PropType::Text),
            ("placeholder", PropType::String),
            ("label", PropType::String),
        ],
        events: &[CatalogEvent {
            name: "change",
            payload: &[("value", "String")],
        }],
        children: false,
    },
];

fn primitive(name: &str) -> Option<&'static Primitive> {
    CATALOG.iter().find(|p| p.name == name)
}

/// Warns for every interactive node with no accessible name: no `label`
/// prop and no `Text` content beneath it (RFC-0007 I.1 rule 4).
pub(crate) fn accessibility_lints(view: &MirView, out: &mut Vec<LintWarning>) {
    fn has_text(node: &MirViewNode) -> bool {
        (node.prim.as_deref() == Some("Text") && node.props.contains_key("content"))
            || node.children.iter().any(has_text)
    }
    fn walk(view: &str, node: &MirViewNode, out: &mut Vec<LintWarning>) {
        let interactive = node
            .prim
            .as_deref()
            .and_then(primitive)
            .is_some_and(Primitive::interactive);
        if interactive && !node.props.contains_key("label") && !has_text(node) {
            out.push(LintWarning {
                code: "view-accessibility".to_string(),
                message: format!(
                    "view '{view}', element '{}': interactive '{}' has no accessible name; give it a label or Text content",
                    node.key,
                    node.prim.as_deref().unwrap_or_default()
                ),
                definition: view.to_string(),
                pipeline: 0,
                step: 0,
            });
        }
        for child in &node.children {
            walk(view, child, out);
        }
    }
    walk(&view.name, &view.root, out);
}

fn type_error(message: String) -> GraphemeError {
    GraphemeError::TypeError(message)
}

pub(crate) fn lower_views(
    program: &Program,
    stores: &[HirStore],
    executables: &[HirExecutable],
) -> Result<Vec<MirView>, GraphemeError> {
    let defs: Vec<&ViewDef> = program
        .definitions
        .iter()
        .filter_map(|d| match d {
            Definition::View(v) => Some(v),
            _ => None,
        })
        .collect();
    let mut names = HashMap::new();
    for def in &defs {
        if names
            .insert(def.name.as_str(), def.store.as_deref())
            .is_some()
        {
            return Err(type_error(format!(
                "view '{}' is declared more than once",
                def.name
            )));
        }
        if primitive(&def.name).is_some() {
            return Err(type_error(format!(
                "view '{}' has the name of a built-in primitive",
                def.name
            )));
        }
        if executables.iter().any(|e| e.name == def.name)
            || stores.iter().any(|s| s.name == def.name)
        {
            return Err(type_error(format!(
                "view '{}' has the same name as another definition",
                def.name
            )));
        }
    }
    defs.iter()
        .map(|def| {
            let store = match &def.store {
                Some(name) => Some(stores.iter().find(|s| &s.name == name).ok_or_else(|| {
                    type_error(format!(
                        "view '{}' is on '{name}', which is not a declared state",
                        def.name
                    ))
                })?),
                None => None,
            };
            let ctx = Ctx {
                view: def,
                store,
                views: &names,
                executables,
            };
            Ok(MirView {
                name: def.name.clone(),
                store: def.store.clone(),
                root: ctx.lower_element(&def.root, def.name.clone(), &[])?,
            })
        })
        .collect()
}

fn is_bare_view_ref(element: &ViewElement, views: &HashMap<&str, Option<&str>>) -> bool {
    element.child_name.is_none()
        && element.content.is_none()
        && element.props.is_empty()
        && element.events.is_empty()
        && element.children.is_empty()
        && views.contains_key(element.kind.as_str())
}

/// Lowers `glyph Name on Store { expose …; Element }` (RFC-0007 Part H).
///
/// A bare view reference reuses that view as the root. Any other element is
/// lowered as a synthetic view named after the glyph. `expose` names must be
/// mutations or queries on the glyph's store, or views on that store.
pub(crate) fn lower_ui_glyphs(
    program: &Program,
    stores: &[HirStore],
    executables: &[HirExecutable],
    views: &mut Vec<MirView>,
) -> Result<Vec<MirUiGlyph>, GraphemeError> {
    let glyphs: Vec<&GlyphDef> = program
        .definitions
        .iter()
        .filter_map(|d| match d {
            Definition::Glyph(g) if g.store.is_some() => Some(g),
            _ => None,
        })
        .collect();
    if glyphs.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for glyph in glyphs {
        let store_name = glyph.store.as_deref().expect("UI glyph has store");
        let store = stores.iter().find(|s| s.name == store_name).ok_or_else(|| {
            type_error(format!(
                "glyph '{}' is on '{store_name}', which is not a declared state",
                glyph.name
            ))
        })?;

        let view_stores: HashMap<&str, Option<&str>> = views
            .iter()
            .map(|v| (v.name.as_str(), v.store.as_deref()))
            .collect();

        let root_el = glyph.root.as_ref().ok_or_else(|| {
            type_error(format!("glyph '{}' is missing its root element", glyph.name))
        })?;

        let root_view = if is_bare_view_ref(root_el, &view_stores) {
            let referred = root_el.kind.as_str();
            match view_stores.get(referred).copied().flatten() {
                Some(other) if other != store_name => {
                    return Err(type_error(format!(
                        "glyph '{}' on {store_name} roots '{referred}', which is on {other}",
                        glyph.name
                    )));
                }
                _ => {}
            }
            referred.to_string()
        } else {
            if views.iter().any(|v| v.name == glyph.name)
                || executables.iter().any(|e| e.name == glyph.name)
                || stores.iter().any(|s| s.name == glyph.name)
                || primitive(&glyph.name).is_some()
            {
                return Err(type_error(format!(
                    "glyph '{}' has the same name as another definition",
                    glyph.name
                )));
            }
            let synthetic = ViewDef {
                name: glyph.name.clone(),
                store: Some(store_name.to_string()),
                root: root_el.clone(),
            };
            let ctx = Ctx {
                view: &synthetic,
                store: Some(store),
                views: &view_stores,
                executables,
            };
            let mir = MirView {
                name: glyph.name.clone(),
                store: Some(store_name.to_string()),
                root: ctx.lower_element(&synthetic.root, glyph.name.clone(), &[])?,
            };
            let name = mir.name.clone();
            views.push(mir);
            name
        };

        let view_stores: HashMap<&str, Option<&str>> = views
            .iter()
            .map(|v| (v.name.as_str(), v.store.as_deref()))
            .collect();

        let mut expose = Vec::new();
        let mut seen = HashSet::new();
        for name in &glyph.expose {
            if !seen.insert(name.as_str()) {
                return Err(type_error(format!(
                    "glyph '{}': '{name}' is exposed more than once",
                    glyph.name
                )));
            }
            if let Some(exec) = executables.iter().find(|e| &e.name == name) {
                match &exec.store {
                    Some(s) if s == store_name => {}
                    Some(other) => {
                        return Err(type_error(format!(
                            "glyph '{}' exposes '{name}' on {other}, not on {store_name}",
                            glyph.name
                        )));
                    }
                    None => {
                        return Err(type_error(format!(
                            "glyph '{}' exposes '{name}', which is not on a state store",
                            glyph.name
                        )));
                    }
                }
                if !matches!(
                    exec.kind,
                    HirExecutableKind::Mutation
                        | HirExecutableKind::Task
                        | HirExecutableKind::Query
                ) {
                    return Err(type_error(format!(
                        "glyph '{}' exposes '{name}', which is not a mutation, task or query",
                        glyph.name
                    )));
                }
            } else if let Some(store_of) = view_stores.get(name.as_str()) {
                if let Some(other) = *store_of
                    && other != store_name
                {
                    return Err(type_error(format!(
                        "glyph '{}' exposes view '{name}' on {other}, not on {store_name}",
                        glyph.name
                    )));
                }
            } else {
                return Err(type_error(format!(
                    "glyph '{}' exposes '{name}', which is not a mutation, task, query or view on {store_name}",
                    glyph.name
                )));
            }
            expose.push(name.clone());
        }

        out.push(MirUiGlyph {
            name: glyph.name.clone(),
            store: store_name.to_string(),
            root: root_view,
            expose,
        });
    }
    Ok(out)
}

struct Ctx<'a> {
    view: &'a ViewDef,
    store: Option<&'a HirStore>,
    views: &'a HashMap<&'a str, Option<&'a str>>,
    executables: &'a [HirExecutable],
}

impl Ctx<'_> {
    fn error(&self, key: &str, message: String) -> GraphemeError {
        type_error(format!(
            "view '{}', element '{key}': {message}",
            self.view.name
        ))
    }

    fn lower_element(
        &self,
        element: &ViewElement,
        key: String,
        items: &[String],
    ) -> Result<MirViewNode, GraphemeError> {
        if element.kind == "if" {
            return self.lower_if(element, key, items);
        }
        if element.kind == "for" {
            return self.lower_for(element, key, items);
        }
        let Some(prim) = primitive(&element.kind) else {
            if self.views.contains_key(element.kind.as_str()) {
                return self.lower_view_ref(element, key);
            }
            return Err(self.error(
                &key,
                format!(
                    "unknown element '{}'; expected a view, if/for, or one of: {}",
                    element.kind,
                    CATALOG
                        .iter()
                        .map(|p| p.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        };

        let mut props = Map::new();
        if let Some(content) = &element.content {
            if !prim.content {
                return Err(self.error(
                    &key,
                    format!(
                        "'{}' takes no positional content; use named props",
                        prim.name
                    ),
                ));
            }
            props.insert(
                "content".to_string(),
                self.lower_prop(&key, "content", PropType::Text, content, items)?,
            );
        }
        for (name, value) in &element.props {
            let Some((_, ty)) = prim.props.iter().find(|(p, _)| p == name) else {
                return Err(self.error(
                    &key,
                    format!(
                        "'{}' has no prop '{name}'; its props are: {}",
                        prim.name,
                        prim.props
                            .iter()
                            .map(|(p, _)| *p)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            };
            if props.contains_key(name) {
                return Err(self.error(&key, format!("prop '{name}' is given more than once")));
            }
            props.insert(
                name.clone(),
                self.lower_prop(&key, name, *ty, value, items)?,
            );
        }

        let mut events = BTreeMap::new();
        for event in &element.events {
            let Some(catalog_event) = prim.event(&event.event) else {
                return Err(self.error(
                    &key,
                    format!("'{}' has no event '{}'", prim.name, event.event),
                ));
            };
            if events.contains_key(&event.event) {
                return Err(self.error(
                    &key,
                    format!("event '{}' is bound more than once", event.event),
                ));
            }
            let binding = self.lower_binding(
                &key,
                &event.target,
                &event.args,
                catalog_event.payload,
                items,
            )?;
            events.insert(event.event.clone(), binding);
        }

        if !prim.children && !element.children.is_empty() {
            return Err(self.error(&key, format!("'{}' cannot have children", prim.name)));
        }
        let children = self.lower_children(&element.children, &key, items)?;

        Ok(MirViewNode {
            prim: Some(prim.name.to_string()),
            view: None,
            when: None,
            else_children: Vec::new(),
            repeat: None,
            key,
            role: Some(prim.role.to_string()),
            props,
            events,
            children,
        })
    }

    fn lower_if(
        &self,
        element: &ViewElement,
        key: String,
        items: &[String],
    ) -> Result<MirViewNode, GraphemeError> {
        let Some(cond) = &element.condition else {
            return Err(self.error(&key, "if is missing a condition".to_string()));
        };
        if !element.events.is_empty() || !element.props.is_empty() || element.content.is_some() {
            return Err(self.error(
                &key,
                "if cannot have props, content or event bindings".to_string(),
            ));
        }
        let when = self.lower_value(&key, cond, items)?;
        if let Some(actual) = self.condition_type(&when)
            && actual != "Bool"
        {
            return Err(self.error(
                &key,
                format!("if condition must be Bool, found {actual}"),
            ));
        }
        let children = self.lower_children(&element.children, &key, items)?;
        let else_children = self.lower_children(&element.else_children, &key, items)?;
        Ok(MirViewNode {
            prim: None,
            view: None,
            when: Some(when),
            else_children,
            repeat: None,
            key,
            role: None,
            props: Map::new(),
            events: BTreeMap::new(),
            children,
        })
    }

    fn lower_for(
        &self,
        element: &ViewElement,
        key: String,
        items: &[String],
    ) -> Result<MirViewNode, GraphemeError> {
        let Some(repeat) = &element.repeat else {
            return Err(self.error(&key, "for is missing its bindings".to_string()));
        };
        if !element.events.is_empty() || !element.props.is_empty() || element.content.is_some() {
            return Err(self.error(
                &key,
                "for cannot have props, content or event bindings".to_string(),
            ));
        }
        self.reject_for_name(&key, &repeat.item, items, "item")?;
        if let Some(index) = &repeat.index {
            if index == &repeat.item {
                return Err(self.error(
                    &key,
                    format!("for index '${index}' has the same name as the item"),
                ));
            }
            self.reject_for_name(&key, index, items, "index")?;
        }
        let source = self.lower_value(&key, &repeat.source, items)?;
        let mut inner = items.to_vec();
        inner.push(repeat.item.clone());
        if let Some(index) = &repeat.index {
            inner.push(index.clone());
        }
        let key_expr = self.lower_value(&key, &repeat.key, &inner)?;
        let children = self.lower_children(&element.children, &key, &inner)?;
        Ok(MirViewNode {
            prim: None,
            view: None,
            when: None,
            else_children: Vec::new(),
            repeat: Some(MirViewFor {
                item: repeat.item.clone(),
                index: repeat.index.clone(),
                source,
                key: key_expr,
            }),
            key,
            role: None,
            props: Map::new(),
            events: BTreeMap::new(),
            children,
        })
    }

    /// `$if` / `$for` cannot be written as a child name (`$` is not an ident),
    /// so an unnamed conditional never collides with `if: Button` or `for: Text`.
    fn control_segment(kind: &str, count: usize) -> String {
        if count == 0 {
            format!("${kind}")
        } else {
            format!("${kind}{count}")
        }
    }

    fn reject_for_name(
        &self,
        key: &str,
        name: &str,
        items: &[String],
        role: &str,
    ) -> Result<(), GraphemeError> {
        if self
            .store
            .is_some_and(|s| s.fields.iter().any(|f| f.name == name))
        {
            return Err(self.error(
                key,
                format!("for {role} '${name}' has the same name as a field of the store"),
            ));
        }
        if items.iter().any(|item| item == name) {
            return Err(self.error(
                key,
                format!("for {role} '${name}' shadows an outer for binding"),
            ));
        }
        Ok(())
    }

    fn lower_children(
        &self,
        elements: &[ViewElement],
        key: &str,
        items: &[String],
    ) -> Result<Vec<MirViewNode>, GraphemeError> {
        let mut children = Vec::new();
        let mut named = HashSet::new();
        let mut ordinal = 0usize;
        let mut if_count = 0usize;
        let mut for_count = 0usize;
        for child in elements {
            let segment = match &child.child_name {
                Some(name) => {
                    if !named.insert(name.as_str()) {
                        return Err(self.error(key, format!("two children are named '{name}'")));
                    }
                    name.clone()
                }
                None if child.kind == "if" => {
                    let segment = Self::control_segment("if", if_count);
                    if_count += 1;
                    segment
                }
                None if child.kind == "for" => {
                    let segment = Self::control_segment("for", for_count);
                    for_count += 1;
                    segment
                }
                None => {
                    let segment = ordinal.to_string();
                    ordinal += 1;
                    segment
                }
            };
            children.push(self.lower_element(child, format!("{key}/{segment}"), items)?);
        }
        Ok(children)
    }

    fn lower_view_ref(
        &self,
        element: &ViewElement,
        key: String,
    ) -> Result<MirViewNode, GraphemeError> {
        if element.content.is_some() || !element.props.is_empty() {
            return Err(self.error(
                &key,
                format!(
                    "view '{}' takes no arguments yet (view params are not supported)",
                    element.kind
                ),
            ));
        }
        if !element.events.is_empty() || !element.children.is_empty() {
            return Err(self.error(
                &key,
                format!(
                    "view '{}' takes no body yet (slots and event params are not supported)",
                    element.kind
                ),
            ));
        }
        if let Some(Some(other)) = self.views.get(element.kind.as_str())
            && self.view.store.as_deref() != Some(*other)
        {
            return Err(self.error(
                &key,
                format!(
                    "view '{}' is on state '{other}'; it can only be used from a view on the same state",
                    element.kind
                ),
            ));
        }
        Ok(MirViewNode {
            prim: None,
            view: Some(element.kind.clone()),
            when: None,
            else_children: Vec::new(),
            repeat: None,
            key,
            role: None,
            props: Map::new(),
            events: BTreeMap::new(),
            children: Vec::new(),
        })
    }

    fn lower_prop(
        &self,
        key: &str,
        name: &str,
        ty: PropType,
        value: &Value,
        items: &[String],
    ) -> Result<JsonValue, GraphemeError> {
        let lowered = self.lower_value(key, value, items)?;
        if let Some(actual) = self.static_type(&lowered) {
            let expected = match ty {
                PropType::Text => return Ok(lowered),
                PropType::String => "String",
                PropType::Int => "Int",
                PropType::Bool => "Bool",
            };
            if actual != expected {
                return Err(self.error(
                    key,
                    format!("prop '{name}' expects {expected}, found {actual}"),
                ));
            }
        }
        Ok(lowered)
    }

    fn lower_binding(
        &self,
        key: &str,
        target: &str,
        args: &[(String, Value)],
        event_payload: &[(&str, &str)],
        items: &[String],
    ) -> Result<MirViewBinding, GraphemeError> {
        let Some(exec) = self.executables.iter().find(|e| e.name == target) else {
            return Err(self.error(key, format!("binding target '{target}' is not declared")));
        };
        if !matches!(
            exec.kind,
            HirExecutableKind::Mutation | HirExecutableKind::Task
        ) {
            return Err(self.error(
                key,
                format!(
                    "binding target '{target}' is not a mutation or task; events can only invoke mutations and tasks"
                ),
            ));
        }
        match (&self.store, &exec.store) {
            (Some(store), Some(target_store)) if &store.name == target_store => {}
            (Some(store), _) => {
                return Err(self.error(
                    key,
                    format!(
                        "'{target}' is not a mutation or task on state '{}'",
                        store.name
                    ),
                ));
            }
            (None, _) => {
                return Err(self.error(
                    key,
                    format!(
                        "'{target}' cannot be bound here: view '{}' has no `on` state",
                        self.view.name
                    ),
                ));
            }
        }

        let mut bound = Map::new();
        for (name, value) in args {
            let Some(param) = exec.params.iter().find(|p| &p.name == name) else {
                return Err(self.error(key, format!("'{target}' has no parameter '{name}'")));
            };
            if bound.contains_key(name) {
                return Err(self.error(key, format!("argument '{name}' is given more than once")));
            }
            if event_payload.iter().any(|(payload_name, _)| payload_name == name) {
                return Err(self.error(
                    key,
                    format!(
                        "argument '{name}' of '{target}' is supplied by the binding and by the event payload"
                    ),
                ));
            }
            let lowered = self.lower_value(key, value, items)?;
            if let (Some(actual), Some(expected)) =
                (self.static_type(&lowered), scalar_name(&param.type_ref))
                && !(actual == expected || (expected == "Float" && actual == "Int"))
            {
                return Err(self.error(
                    key,
                    format!("argument '{name}' of '{target}' expects {expected}, found {actual}"),
                ));
            }
            bound.insert(name.clone(), lowered);
        }

        let mut payload_params = Vec::new();
        for param in &exec.params {
            if !param.required || param.default.is_some() || bound.contains_key(&param.name) {
                continue;
            }
            if let Some((_, payload_ty)) = event_payload
                .iter()
                .find(|(payload_name, _)| *payload_name == param.name)
            {
                let expected = scalar_name(&param.type_ref).unwrap_or("a non-scalar type");
                if !payload_type_compatible(expected, payload_ty) {
                    return Err(self.error(
                        key,
                        format!(
                            "parameter '{}' of '{target}' has type {expected}, but the event payload supplies {payload_ty}",
                            param.name
                        ),
                    ));
                }
                payload_params.push(param.name.clone());
                continue;
            }
            return Err(self.error(
                key,
                format!(
                    "'{target}' needs argument '{}', which the binding does not supply",
                    param.name
                ),
            ));
        }
        Ok(MirViewBinding {
            target: target.to_string(),
            args: bound,
            payload_params,
        })
    }

    /// Lowers a prop or argument, resolving `$field` and `{$field}` against the view's store.
    /// Inside a `for $item`, `$item` / `$item.x` stay as `$var` paths for runtime.
    fn lower_value(
        &self,
        key: &str,
        value: &Value,
        items: &[String],
    ) -> Result<JsonValue, GraphemeError> {
        let mut json = value_to_json(value);
        let mut error = None;
        rewrite_vars(&mut json, &mut |path| {
            let path = path.strip_prefix("state.").unwrap_or(path);
            if item_in_scope(items, path) {
                return Some(plain_var(path));
            }
            match self.field_path(path) {
                Ok(path) => Some(state_var(&path)),
                Err(message) => {
                    error.get_or_insert(message);
                    None
                }
            }
        });
        if let Some(message) = error {
            return Err(self.error(key, message));
        }
        rewrite_strings(&mut json, &mut |s| {
            self.rewrite_interpolation(s, items)
                .map_err(|m| self.error(key, m))
        })?;
        Ok(json)
    }

    fn field_path(&self, path: &str) -> Result<String, String> {
        let (head, _) = split_path(path);
        match self.store {
            Some(store) if find_field(store, head).is_some() => Ok(path.to_string()),
            Some(store) => Err(format!(
                "'${path}' is not a field of state '{}'",
                store.name
            )),
            None => Err(format!(
                "'${path}' cannot be read: view '{}' has no `on` state",
                self.view.name
            )),
        }
    }

    fn rewrite_interpolation(&self, s: &str, items: &[String]) -> Result<String, String> {
        if !s.contains("{$") {
            return Ok(s.to_string());
        }
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        while let Some(pos) = rest.find("{$") {
            out.push_str(&rest[..pos + 2]);
            rest = &rest[pos + 2..];
            let end = rest.find('}').unwrap_or(rest.len());
            let path = rest[..end].strip_prefix("state.").unwrap_or(&rest[..end]);
            if item_in_scope(items, path) {
                out.push_str(path);
            } else {
                out.push_str("state.");
                out.push_str(&self.field_path(path)?);
            }
            rest = &rest[end..];
        }
        out.push_str(rest);
        Ok(out)
    }

    /// Bool for a view `if`, when the condition's type is known.
    fn condition_type(&self, value: &JsonValue) -> Option<&'static str> {
        if let Some(ty) = self.static_type(value) {
            return Some(ty);
        }
        let (op, _) = value.as_object().and_then(grapheme_artifact::expr::expr_parts)?;
        match op {
            "==" | "!=" | "<" | "<=" | ">" | ">=" | "&&" | "||" | "!" => Some("Bool"),
            _ => None,
        }
    }

    /// The type of a literal or a direct field read, when it is known statically.
    fn static_type(&self, value: &JsonValue) -> Option<&'static str> {
        match value {
            JsonValue::String(_) => Some("String"),
            JsonValue::Bool(_) => Some("Bool"),
            JsonValue::Number(n) if n.is_i64() || n.is_u64() => Some("Int"),
            JsonValue::Number(_) => Some("Float"),
            JsonValue::Object(map) if map.len() == 1 => {
                let path = map.get("$var")?.as_str()?.strip_prefix("state.")?;
                if path.contains('.') {
                    return None;
                }
                scalar_name(&find_field(self.store?, path)?.type_ref)
            }
            _ => None,
        }
    }
}

fn payload_type_compatible(expected: &str, payload_ty: &str) -> bool {
    expected == payload_ty || (expected == "Float" && payload_ty == "Int")
}

fn scalar_name(type_ref: &TypeRef) -> Option<&'static str> {
    match type_ref {
        TypeRef::Scalar(ScalarKind::String, _) => Some("String"),
        TypeRef::Scalar(ScalarKind::Int, _) => Some("Int"),
        TypeRef::Scalar(ScalarKind::Float, _) => Some("Float"),
        TypeRef::Scalar(ScalarKind::Bool, _) => Some("Bool"),
        TypeRef::Named(name, _) => match name.as_str() {
            "String" => Some("String"),
            "Int" => Some("Int"),
            "Float" => Some("Float"),
            "Bool" => Some("Bool"),
            _ => None,
        },
        _ => None,
    }
}

fn item_in_scope(items: &[String], path: &str) -> bool {
    items
        .iter()
        .any(|item| path == item || path.starts_with(&format!("{item}.")))
}

fn state_var(path: &str) -> JsonValue {
    plain_var(&format!("state.{path}"))
}

fn plain_var(path: &str) -> JsonValue {
    let mut object = Map::new();
    object.insert("$var".to_string(), JsonValue::from(path));
    JsonValue::Object(object)
}
