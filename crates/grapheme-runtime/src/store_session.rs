//! Headless state-store session (RFC-0007 Part A.3).
//!
//! A store is a runtime object separate from the pipeline [`AgentState`](crate::AgentState).
//! Each invoked mutation runs against the store's latest committed fields and
//! commits as one transaction: one revision, with a complete runtime diff and a
//! history diff that omits `transient` fields. A failed mutation commits nothing.
//! A task commits each of its own `apply` steps immediately and keeps those
//! revisions if a later step fails. A mutation the task calls is still one
//! transaction.

use std::collections::{BTreeMap, BTreeSet};

use grapheme_artifact::expr::{evaluate, expr_parts, first_error, select_path};
use grapheme_artifact::{
    ArtifactEnvelope, ExecutionOutcome, MirFunctionKind, MirStore, MirStoreField,
    MirStoreFieldKind, MirUiGlyph, MirView, MirViewNode, VIEW_IR_VERSION,
};
use grapheme_signatures::{SignatureEffect, find_op_spec};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};

use crate::error::RuntimeError;
use crate::host::{CapabilityCall, CapabilityHost, HostCallError};
use crate::runtime::{RuntimeEngine, RuntimeOptions};

/// One committed transaction on one store instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreRevision {
    pub store: String,
    pub revision: u64,
    /// The mutation whose run produced this revision.
    pub cause: String,
    /// Every changed field as `[old, new]`, transient fields included.
    pub diff: Map<String, JsonValue>,
    /// `diff` filtered by history policy: transient fields removed.
    pub history_diff: Map<String, JsonValue>,
}

/// Why a task made a capability call (RFC-0007 F.4). `generation` is unset in
/// this slice: concurrency fencing is not implemented yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectCause {
    /// Action id when the task was started from a view or `expose` action.
    pub action: Option<String>,
    pub task: String,
    pub generation: Option<u64>,
}

/// One capability call a store task made (RFC-0007 F.4).
///
/// Pure core helpers (`core.value`, `core.echo`, `core.set_fields`) are not
/// receipts. A call refused before dispatch is `failed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectReceipt {
    pub effect: u64,
    pub op: String,
    pub args: JsonValue,
    pub cause: EffectCause,
    pub store: String,
    pub revision: u64,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Receipts carried by [`RuntimeError::TaskFailed`], if `err` is that failure.
pub fn task_failure_effects(err: &RuntimeError) -> Option<Vec<EffectReceipt>> {
    let RuntimeError::TaskFailed { effects, .. } = err else {
        return None;
    };
    serde_json::from_value(effects.clone()).ok()
}

/// The result of invoking a store mutation or task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreCommit {
    pub store: String,
    /// The store's revision after the invoke. Unchanged when nothing changed.
    pub revision: u64,
    /// `None` when the run changed no field, so no revision was committed.
    /// For tasks, this is the *last* committed revision of the run.
    pub committed: Option<StoreRevision>,
    /// Capability-call receipts from a task (RFC-0007 F.4). Empty for mutations
    /// and when the task only called pure core helpers. A failed task returns
    /// the same list on [`RuntimeError::TaskFailed`] instead of here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EffectReceipt>,
}

#[derive(Debug, Clone)]
struct StoreInstance {
    schema: MirStore,
    fields: Map<String, JsonValue>,
    revision: u64,
}

/// One event bound in a rendered view (RFC-0007 I.2). `id` is opaque and is
/// valid only while the binding's epoch is current.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewAction {
    pub id: String,
    pub key: String,
    pub event: String,
    pub target: String,
    pub bound_args: Map<String, JsonValue>,
    pub payload_params: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A view evaluated against its store's latest revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewSnapshot {
    pub view: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    /// The evaluated tree: `{ "$ir": "grapheme-ui/1", "root": node }`.
    pub ir: JsonValue,
    pub actions: Vec<ViewAction>,
}

/// A UI glyph evaluated as an app session snapshot (RFC-0007 I.2 / Part H).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub glyph: String,
    pub store: String,
    pub revision: u64,
    /// The evaluated tree: `{ "$ir": "grapheme-ui/1", "root": node }`.
    pub ir: JsonValue,
    /// Event bindings in the tree, plus `expose`d mutations as `expose:<Name>` actions.
    pub actions: Vec<ViewAction>,
}

/// A node event binding and its epoch (RFC-0007 I.3).
#[derive(Debug, Clone)]
struct Binding {
    view: String,
    target: String,
    args: Map<String, JsonValue>,
    payload_params: Vec<String>,
    epoch: u64,
    mounted: bool,
}

/// An event binding found while evaluating a view, before it has an epoch.
struct BoundEvent {
    key: String,
    event: String,
    target: String,
    args: Map<String, JsonValue>,
    payload_params: Vec<String>,
    role: Option<String>,
    label: Option<String>,
}

const MAX_VIEW_DEPTH: usize = 64;

/// Owns one instance of every store declared by an artifact.
#[derive(Debug, Clone)]
pub struct StoreSession {
    artifact: ArtifactEnvelope,
    stores: BTreeMap<String, StoreInstance>,
    log: Vec<StoreRevision>,
    bindings: BTreeMap<(String, String), Binding>,
    rendered: BTreeSet<String>,
    /// Currently mounted UI glyph, if any.
    mounted: Option<String>,
}

fn session_error(message: impl Into<String>) -> RuntimeError {
    RuntimeError::RuntimeError(message.into())
}

impl StoreSession {
    /// Creates revision 0 of every store from its field defaults.
    pub fn new(artifact: ArtifactEnvelope) -> Result<Self, RuntimeError> {
        Self::with_consts(artifact, &Map::new())
    }

    /// Like [`StoreSession::new`], with host-supplied values for `const` fields,
    /// keyed `{ "Store": { "field": value } }`.
    pub fn with_consts(
        artifact: ArtifactEnvelope,
        consts: &Map<String, JsonValue>,
    ) -> Result<Self, RuntimeError> {
        let mut stores = BTreeMap::new();
        for schema in &artifact.payload.mir.stores {
            let seeds = consts.get(&schema.name).and_then(JsonValue::as_object);
            let mut fields = Map::new();
            for field in stored_fields(schema) {
                let value = match seeds.and_then(|s| s.get(&field.name)) {
                    Some(seed) if field.kind == MirStoreFieldKind::Const => seed.clone(),
                    Some(_) => {
                        return Err(session_error(format!(
                            "state '{}': only const fields can be seeded, not '{}'",
                            schema.name, field.name
                        )));
                    }
                    None => eval_store_value(&field.value, &Map::new()).map_err(|e| {
                        session_error(format!(
                            "state '{}': default of '{}': {e}",
                            schema.name, field.name
                        ))
                    })?,
                };
                fields.insert(field.name.clone(), value);
            }
            if let Some(unknown) = seeds.and_then(|s| s.keys().find(|k| !fields.contains_key(*k))) {
                return Err(session_error(format!(
                    "state '{}' has no const field '{unknown}'",
                    schema.name
                )));
            }
            stores.insert(
                schema.name.clone(),
                StoreInstance {
                    schema: schema.clone(),
                    fields,
                    revision: 0,
                },
            );
        }
        if let Some(unknown) = consts.keys().find(|k| !stores.contains_key(*k)) {
            return Err(session_error(format!("no state named '{unknown}'")));
        }
        Ok(Self {
            artifact,
            stores,
            log: Vec::new(),
            bindings: BTreeMap::new(),
            rendered: BTreeSet::new(),
            mounted: None,
        })
    }

    pub fn store_names(&self) -> impl Iterator<Item = &str> {
        self.stores.keys().map(String::as_str)
    }

    /// UI glyphs declared by the program (`glyph Name on Store { … }`).
    pub fn ui_glyphs(&self) -> &[MirUiGlyph] {
        &self.artifact.payload.mir.ui_glyphs
    }

    /// Currently mounted UI glyph, if any.
    pub fn mounted_glyph(&self) -> Option<&str> {
        self.mounted.as_deref()
    }

    pub fn revision(&self, store: &str) -> Option<u64> {
        self.stores.get(store).map(|s| s.revision)
    }

    /// Every committed revision, oldest first.
    pub fn revisions(&self) -> &[StoreRevision] {
        &self.log
    }

    /// The store's stored fields plus its derives, evaluated at the latest revision.
    pub fn snapshot(&self, store: &str) -> Result<JsonValue, RuntimeError> {
        let instance = self
            .stores
            .get(store)
            .ok_or_else(|| session_error(format!("no state named '{store}'")))?;
        let mut out = instance.fields.clone();
        let scope = JsonValue::Object(instance.fields.clone());
        for field in instance
            .schema
            .fields
            .iter()
            .filter(|f| f.kind == MirStoreFieldKind::Derive)
        {
            let value = eval_store_value(&field.value, scope.as_object().unwrap())
                .map_err(|e| session_error(format!("derive '{}': {e}", field.name)))?;
            out.insert(field.name.clone(), value);
        }
        Ok(JsonValue::Object(out))
    }

    /// Runs a store query against the store's latest fields and returns its
    /// result. Queries are read-only: nothing is committed.
    pub fn query(
        &self,
        query: &str,
        args: Option<JsonValue>,
        options: RuntimeOptions,
        host: &mut dyn CapabilityHost,
    ) -> Result<JsonValue, RuntimeError> {
        let (store, kind) = self.bound_store(query)?;
        if !matches!(kind, MirFunctionKind::Query) {
            return Err(session_error(format!(
                "'{query}' is a mutation; use invoke to run it"
            )));
        }
        let (state, _) = self.run(query, &store, args, options, host)?;
        Ok(state.current)
    }

    /// Evaluates `view` against its store's latest revision and returns the
    /// tree plus its action list. Rendering is read-only for stores; it only
    /// advances binding epochs whose node was remounted or rebound.
    pub fn render(&mut self, view: &str) -> Result<ViewSnapshot, RuntimeError> {
        let (mut snapshot, events) = self.evaluate_view(view)?;
        self.rendered.insert(view.to_string());
        snapshot.actions = self.bind(view, events);
        attach_action_ids(&mut snapshot.ir["root"], &snapshot.actions);
        Ok(snapshot)
    }

    /// Mounts a UI glyph and returns its app snapshot (RFC-0007 Part H / I.2).
    /// When `glyph` is `None`, mounts the program's sole UI glyph.
    pub fn mount(&mut self, glyph: Option<&str>) -> Result<AppSnapshot, RuntimeError> {
        let selected = self.resolve_ui_glyph(glyph)?.name.clone();
        self.mounted = Some(selected);
        self.app_snapshot()
    }

    /// Re-evaluates the mounted UI glyph: View IR, tree actions and `expose` actions.
    pub fn app_snapshot(&mut self) -> Result<AppSnapshot, RuntimeError> {
        let glyph_name = self
            .mounted
            .clone()
            .ok_or_else(|| session_error("no UI glyph is mounted; call mount first"))?;
        let glyph = self.find_ui_glyph(&glyph_name)?.clone();
        let mut view = self.render(&glyph.root)?;
        let mut actions = view.actions;
        actions.extend(self.expose_actions(&glyph)?);
        // Keep `view.actions` aligned for callers that read the ViewSnapshot fields.
        view.actions = actions.clone();
        let revision = self.revision(&glyph.store).ok_or_else(|| {
            session_error(format!(
                "glyph '{}': no state named '{}'",
                glyph.name, glyph.store
            ))
        })?;
        Ok(AppSnapshot {
            glyph: glyph.name,
            store: glyph.store,
            revision,
            ir: view.ir,
            actions,
        })
    }

    /// Invokes the action `id` from a rendered view's current action list, or
    /// an `expose:<Name>` action from a mounted UI glyph. The rendered views
    /// (and the mounted glyph, if any) are re-evaluated first, so an id whose
    /// binding changed since it was handed out is rejected with `stale_action`.
    ///
    /// `payload` supplies arguments listed in the action's `payload_params`
    /// (e.g. TextField `change` → `value`). Bound args from the view win only
    /// for names not in the payload object; payload keys overlay when present.
    pub fn invoke_action(
        &mut self,
        id: &str,
        payload: Option<JsonValue>,
        options: RuntimeOptions,
        host: &mut dyn CapabilityHost,
    ) -> Result<StoreCommit, RuntimeError> {
        if self.mounted.is_some() {
            let _ = self.app_snapshot()?;
        } else {
            for view in self.rendered.clone() {
                let (_, events) = self.evaluate_view(&view)?;
                self.bind(&view, events);
            }
        }
        if let Some(name) = id.strip_prefix("expose:") {
            let glyph_name = self.mounted.clone().ok_or_else(|| {
                session_error(format!(
                    "stale_action: '{id}' is an expose action but no UI glyph is mounted"
                ))
            })?;
            let glyph = self.find_ui_glyph(&glyph_name)?.clone();
            if !glyph.expose.iter().any(|e| e == name) {
                return Err(session_error(format!(
                    "stale_action: '{id}' is not in the current action list"
                )));
            }
            let (store, kind) = self.bound_store(name)?;
            if store != glyph.store {
                return Err(session_error(format!(
                    "expose '{name}' is not on glyph store '{}'",
                    glyph.store
                )));
            }
            if matches!(kind, MirFunctionKind::Query) {
                return Err(session_error(format!(
                    "'{name}' is a query; use query() to read exposed queries"
                )));
            }
            let required = self
                .artifact
                .payload
                .mir
                .functions
                .iter()
                .find(|f| f.name == name)
                .map(|f| {
                    f.params
                        .iter()
                        .filter(|p| p.required)
                        .map(|p| p.name.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let args = merge_action_args(
                &self.artifact,
                name,
                Map::new(),
                &required,
                payload.as_ref(),
                id,
            )?;
            return self.invoke_calling(name, args, options, host, Some(id));
        }
        let Some(binding) = self
            .bindings
            .iter()
            .find(|((key, event), b)| b.mounted && action_id(key, event, b.epoch) == id)
            .map(|(_, b)| b.clone())
        else {
            return Err(session_error(format!(
                "stale_action: '{id}' is not in the current action list"
            )));
        };
        let args = merge_action_args(
            &self.artifact,
            &binding.target,
            binding.args.clone(),
            &binding.payload_params,
            payload.as_ref(),
            id,
        )?;
        self.invoke_calling(&binding.target, args, options, host, Some(id))
    }

    fn resolve_ui_glyph(&self, glyph: Option<&str>) -> Result<&MirUiGlyph, RuntimeError> {
        match glyph {
            Some(name) => self.find_ui_glyph(name),
            None => {
                let glyphs = &self.artifact.payload.mir.ui_glyphs;
                match glyphs.len() {
                    0 => Err(session_error(
                        "program has no UI glyph (`glyph Name on Store {{ … }}`)",
                    )),
                    1 => Ok(&glyphs[0]),
                    _ => Err(session_error(format!(
                        "program has multiple UI glyphs ({}); pass a name to mount",
                        glyphs
                            .iter()
                            .map(|g| g.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))),
                }
            }
        }
    }

    fn find_ui_glyph(&self, name: &str) -> Result<&MirUiGlyph, RuntimeError> {
        self.artifact
            .payload
            .mir
            .ui_glyphs
            .iter()
            .find(|g| g.name == name)
            .ok_or_else(|| session_error(format!("no UI glyph named '{name}'")))
    }

    fn expose_actions(&self, glyph: &MirUiGlyph) -> Result<Vec<ViewAction>, RuntimeError> {
        let mut actions = Vec::new();
        for name in &glyph.expose {
            if let Some(func) = self
                .artifact
                .payload
                .mir
                .functions
                .iter()
                .find(|f| &f.name == name)
            {
                if matches!(func.kind, MirFunctionKind::Query) {
                    // Exposed queries are readable via query(); they are not actions.
                    continue;
                }
                let payload_params = func
                    .params
                    .iter()
                    .filter(|p| p.required)
                    .map(|p| p.name.clone())
                    .collect();
                actions.push(ViewAction {
                    id: format!("expose:{name}"),
                    key: format!("expose/{name}"),
                    event: "invoke".to_string(),
                    target: name.clone(),
                    bound_args: Map::new(),
                    payload_params,
                    role: None,
                    label: Some(name.clone()),
                });
            }
            // Exposed views are mountable in a host slot; they are not actions.
        }
        Ok(actions)
    }

    fn evaluate_view(&self, name: &str) -> Result<(ViewSnapshot, Vec<BoundEvent>), RuntimeError> {
        let view = self.find_view(name)?;
        let (scope, revision) = match &view.store {
            Some(store) => (self.snapshot(store)?, self.revision(store)),
            None => (JsonValue::Object(Map::new()), None),
        };
        let mut events = Vec::new();
        let root = self.evaluate_node(
            view,
            &view.root,
            &Place {
                key: &view.root.key,
                scope: &scope,
                depth: 0,
            },
            &mut events,
        )?;
        let ir = serde_json::json!({ "$ir": VIEW_IR_VERSION, "root": root });
        Ok((
            ViewSnapshot {
                view: name.to_string(),
                store: view.store.clone(),
                revision,
                ir,
                actions: Vec::new(),
            },
            events,
        ))
    }

    fn find_view(&self, name: &str) -> Result<&MirView, RuntimeError> {
        self.artifact
            .payload
            .mir
            .views
            .iter()
            .find(|v| v.name == name)
            .ok_or_else(|| session_error(format!("no view named '{name}'")))
    }

    /// Expands `if` / `for` children into realized nodes (RFC-0007 C.7).
    fn evaluate_children(
        &self,
        view: &MirView,
        node: &MirViewNode,
        place: &Place<'_>,
        events: &mut Vec<BoundEvent>,
        out: &mut Vec<JsonValue>,
    ) -> Result<(), RuntimeError> {
        self.expand_list(view, &node.key, &node.children, place, events, out)
    }

    /// Splices `nodes` into `out`. `struct_key` is the compiler key of their
    /// parent; `place.key` is that parent's key in this render.
    fn expand_list(
        &self,
        view: &MirView,
        struct_key: &str,
        nodes: &[MirViewNode],
        place: &Place<'_>,
        events: &mut Vec<BoundEvent>,
        out: &mut Vec<JsonValue>,
    ) -> Result<(), RuntimeError> {
        for child in nodes {
            let suffix = child.key.strip_prefix(struct_key).unwrap_or(&child.key);
            let child_key = format!("{}{suffix}", place.key);
            if child.when.is_some() {
                let branch = self.if_branch(view, child, &child_key, place.scope)?;
                let branch_place = Place {
                    key: &child_key,
                    scope: place.scope,
                    depth: place.depth,
                };
                self.expand_list(view, &child.key, branch, &branch_place, events, out)?;
                continue;
            }
            if child.repeat.is_some() {
                self.expand_for(view, child, &child_key, place, events, out)?;
                continue;
            }
            out.push(self.evaluate_node(
                view,
                child,
                &Place {
                    key: &child_key,
                    scope: place.scope,
                    depth: place.depth,
                },
                events,
            )?);
        }
        Ok(())
    }

    fn expand_for(
        &self,
        view: &MirView,
        node: &MirViewNode,
        child_key: &str,
        place: &Place<'_>,
        events: &mut Vec<BoundEvent>,
        out: &mut Vec<JsonValue>,
    ) -> Result<(), RuntimeError> {
        let Some(repeat) = &node.repeat else {
            return Ok(());
        };
        let fail = |message: String| {
            session_error(format!("view '{}', element '{child_key}': {message}", view.name))
        };
        let source = eval_view_value(&repeat.source, place.scope)
            .map_err(|e| fail(format!("for source: {e}")))?;
        let items = source.as_array().ok_or_else(|| {
            fail(format!(
                "for source must be a list, found {}",
                type_name(&source)
            ))
        })?;
        let mut seen_keys = BTreeSet::new();
        for (index, item_value) in items.iter().enumerate() {
            let mut item_scope = place.scope.as_object().cloned().unwrap_or_default();
            item_scope.insert(repeat.item.clone(), item_value.clone());
            if let Some(index_name) = &repeat.index {
                item_scope.insert(index_name.clone(), JsonValue::from(index as u64));
            }
            let item_scope = JsonValue::Object(item_scope);
            let item_key = eval_view_value(&repeat.key, &item_scope)
                .map_err(|e| fail(format!("for key: {e}")))?;
            let item_key = match row_key(&item_key) {
                Ok(key) => key,
                Err(message) => return Err(fail(message)),
            };
            if !seen_keys.insert(item_key.clone()) {
                return Err(fail(format!("duplicate for key '{item_key}'")));
            }
            let row_key = format!("{child_key}/{item_key}");
            let row = Place {
                key: &row_key,
                scope: &item_scope,
                depth: place.depth,
            };
            self.evaluate_children(view, node, &row, events, out)?;
        }
        Ok(())
    }

    /// The `if` branch to expand. The condition must be `Bool`; `0` is not true.
    fn if_branch<'a>(
        &self,
        view: &MirView,
        node: &'a MirViewNode,
        key: &str,
        scope: &JsonValue,
    ) -> Result<&'a [MirViewNode], RuntimeError> {
        let fail = |message: String| {
            session_error(format!("view '{}', element '{key}': {message}", view.name))
        };
        let cond = node
            .when
            .as_ref()
            .ok_or_else(|| fail("if is missing a condition".to_string()))?;
        let value = eval_view_value(cond, scope).map_err(|e| fail(format!("if condition: {e}")))?;
        match value {
            JsonValue::Bool(true) => Ok(&node.children),
            JsonValue::Bool(false) => Ok(&node.else_children),
            other => Err(fail(format!(
                "if condition must be Bool, found {}",
                type_name(&other)
            ))),
        }
    }

    /// Evaluates `node` with its key rebased from the view's root key onto `key`,
    /// expanding view references in place.
    fn evaluate_node(
        &self,
        view: &MirView,
        node: &MirViewNode,
        place: &Place<'_>,
        events: &mut Vec<BoundEvent>,
    ) -> Result<JsonValue, RuntimeError> {
        let fail = |message: String| {
            session_error(format!(
                "view '{}', element '{}': {message}",
                view.name, place.key
            ))
        };
        if let Some(name) = &node.view {
            if place.depth >= MAX_VIEW_DEPTH {
                return Err(fail(format!(
                    "view references nest deeper than {MAX_VIEW_DEPTH}"
                )));
            }
            let referenced = self.find_view(name)?;
            let nested = Place {
                key: place.key,
                scope: place.scope,
                depth: place.depth + 1,
            };
            let mut out = self.evaluate_node(referenced, &referenced.root, &nested, events)?;
            out["view"] = JsonValue::from(name.clone());
            return Ok(out);
        }

        let mut out = Map::new();
        if let Some(prim) = &node.prim {
            out.insert("$prim".to_string(), JsonValue::from(prim.clone()));
        }
        out.insert("key".to_string(), JsonValue::from(place.key));
        if let Some(role) = &node.role {
            out.insert("role".to_string(), JsonValue::from(role.clone()));
        }
        let mut props = Map::new();
        for (name, value) in &node.props {
            let value = eval_view_value(value, place.scope)
                .map_err(|e| fail(format!("prop '{name}': {e}")))?;
            props.insert(name.clone(), value);
        }
        let mut children = Vec::new();
        self.evaluate_children(view, node, place, events, &mut children)?;
        let label = props
            .get("label")
            .and_then(JsonValue::as_str)
            .map(str::to_string)
            .or_else(|| first_text(&children));
        let mut bound = Map::new();
        for (event, binding) in &node.events {
            let mut args = Map::new();
            for (name, value) in &binding.args {
                let value = eval_view_value(value, place.scope)
                    .map_err(|e| fail(format!("argument '{name}' of '{}': {e}", binding.target)))?;
                args.insert(name.clone(), value);
            }
            let mut event_ir = serde_json::Map::new();
            event_ir.insert("$action".to_string(), JsonValue::from(binding.target.clone()));
            event_ir.insert("args".to_string(), JsonValue::Object(args.clone()));
            if !binding.payload_params.is_empty() {
                event_ir.insert(
                    "payload_params".to_string(),
                    JsonValue::from(binding.payload_params.clone()),
                );
            }
            bound.insert(event.clone(), JsonValue::Object(event_ir));
            events.push(BoundEvent {
                key: place.key.to_string(),
                event: event.clone(),
                target: binding.target.clone(),
                args,
                payload_params: binding.payload_params.clone(),
                role: node.role.clone(),
                label: label.clone(),
            });
        }
        if !bound.is_empty() {
            out.insert("events".to_string(), JsonValue::Object(bound));
        }
        if !props.is_empty() {
            out.insert("props".to_string(), JsonValue::Object(props));
        }
        if !children.is_empty() {
            out.insert("children".to_string(), JsonValue::Array(children));
        }
        Ok(JsonValue::Object(out))
    }

    /// Assigns epochs: a binding keeps its epoch while its node stays mounted
    /// with the same target, arguments and payload params; otherwise the epoch
    /// advances.
    fn bind(&mut self, view: &str, events: Vec<BoundEvent>) -> Vec<ViewAction> {
        let mut seen = BTreeSet::new();
        let mut actions = Vec::new();
        for event in events {
            let slot = (event.key.clone(), event.event.clone());
            let binding = self
                .bindings
                .entry(slot.clone())
                .or_insert_with(|| Binding {
                    view: view.to_string(),
                    target: event.target.clone(),
                    args: event.args.clone(),
                    payload_params: event.payload_params.clone(),
                    epoch: 0,
                    mounted: false,
                });
            if !binding.mounted
                || binding.view != view
                || binding.target != event.target
                || binding.args != event.args
                || binding.payload_params != event.payload_params
            {
                binding.epoch += 1;
            }
            binding.view = view.to_string();
            binding.target = event.target.clone();
            binding.args = event.args.clone();
            binding.payload_params = event.payload_params.clone();
            binding.mounted = true;
            actions.push(ViewAction {
                id: action_id(&event.key, &event.event, binding.epoch),
                key: event.key,
                event: event.event,
                target: event.target,
                bound_args: event.args,
                payload_params: event.payload_params,
                role: event.role,
                label: event.label,
            });
            seen.insert(slot);
        }
        for (slot, binding) in &mut self.bindings {
            if binding.view == view && !seen.contains(slot) {
                binding.mounted = false;
            }
        }
        actions
    }

    fn bound_store(&self, name: &str) -> Result<(String, MirFunctionKind), RuntimeError> {
        self.artifact
            .payload
            .mir
            .functions
            .iter()
            .find(|f| f.name == name)
            .and_then(|f| f.store.clone().map(|s| (s, f.kind.clone())))
            .ok_or_else(|| session_error(format!("'{name}' is not declared on a state store")))
    }

    fn run(
        &self,
        name: &str,
        store: &str,
        args: Option<JsonValue>,
        mut options: RuntimeOptions,
        host: &mut dyn CapabilityHost,
    ) -> Result<(crate::AgentState, Vec<FieldWrite>), RuntimeError> {
        let instance = self
            .stores
            .get(store)
            .ok_or_else(|| session_error(format!("no state named '{store}'")))?;
        let mut artifact = self.artifact.clone();
        artifact.entrypoint = name.to_string();
        let fields = JsonValue::Object(instance.fields.clone());
        options.initial_state_current = Some(fields.clone());
        options.initial_state_lane = Some(fields);
        options.entrypoint_args = args;
        let mut recorder = WriteRecorder {
            inner: host,
            writes: Vec::new(),
        };
        let (state, result) =
            RuntimeEngine::new(options).execute_artifact(&artifact, &mut recorder)?;
        if !matches!(result.outcome, ExecutionOutcome::Succeeded) {
            return Err(session_error(format!(
                "'{name}' failed; nothing committed: {}",
                result.message.unwrap_or_default()
            )));
        }
        Ok((state, recorder.writes))
    }

    /// Runs a mutation or task (declared `on` a store) against the store's
    /// latest fields. Mutations commit one revision for the whole run. Tasks
    /// commit one revision per `apply`/`transition` and may call capabilities.
    /// A mutation called from a task commits once, when that call returns.
    pub fn invoke(
        &mut self,
        mutation: &str,
        args: Option<JsonValue>,
        options: RuntimeOptions,
        host: &mut dyn CapabilityHost,
    ) -> Result<StoreCommit, RuntimeError> {
        self.invoke_calling(mutation, args, options, host, None)
    }

    fn invoke_calling(
        &mut self,
        mutation: &str,
        args: Option<JsonValue>,
        options: RuntimeOptions,
        host: &mut dyn CapabilityHost,
        action: Option<&str>,
    ) -> Result<StoreCommit, RuntimeError> {
        let (store_name, kind) = self.bound_store(mutation)?;
        if matches!(kind, MirFunctionKind::Query) {
            return Err(session_error(format!(
                "'{mutation}' is a query; queries are read-only"
            )));
        }
        if matches!(kind, MirFunctionKind::Task) {
            return self.invoke_task(mutation, &store_name, args, options, host, action);
        }
        let (state, writes) = self.run(mutation, &store_name, args, options, host)?;
        let next = state
            .state_lane
            .as_ref()
            .or(Some(&state.current))
            .and_then(JsonValue::as_object)
            .cloned()
            .unwrap_or_default();
        self.commit_fields(&store_name, mutation, &next, &writes)
    }

    fn invoke_task(
        &mut self,
        task: &str,
        store_name: &str,
        args: Option<JsonValue>,
        mut options: RuntimeOptions,
        host: &mut dyn CapabilityHost,
        action: Option<&str>,
    ) -> Result<StoreCommit, RuntimeError> {
        let (fields, mut sink) = {
            let instance = self
                .stores
                .get(store_name)
                .ok_or_else(|| session_error(format!("no state named '{store_name}'")))?;
            (
                JsonValue::Object(instance.fields.clone()),
                TaskCommitSink {
                    store: store_name.to_string(),
                    schema: instance.schema.clone(),
                    fields: instance.fields.clone(),
                    revision: instance.revision,
                    log: Vec::new(),
                    last_committed: None,
                },
            )
        };
        let mut artifact = self.artifact.clone();
        artifact.entrypoint = task.to_string();
        options.initial_state_current = Some(fields.clone());
        options.initial_state_lane = Some(fields);
        options.entrypoint_args = args;

        let mut effects = Vec::new();
        let mut effect_seq = 0u64;
        let executed = {
            let mut task_host = TaskHost {
                sink: &mut sink,
                cause: task.to_string(),
                action: action.map(str::to_string),
                inner: host,
                effects: &mut effects,
                effect_seq: &mut effect_seq,
                txns: Vec::new(),
            };
            RuntimeEngine::new(options).execute_artifact(&artifact, &mut task_host)
        };

        // Task applies commit as they run; keep them even if a later step fails.
        let commits = sink.log.len();
        let last_committed = sink.last_committed.clone();
        let revision = sink.revision;
        let Some(instance) = self.stores.get_mut(store_name) else {
            return Err(session_error(format!("no state named '{store_name}'")));
        };
        instance.fields = sink.fields;
        instance.revision = sink.revision;
        self.log.extend(sink.log);

        let failure = match executed {
            Ok((_, result)) if matches!(result.outcome, ExecutionOutcome::Succeeded) => None,
            Ok((_, result)) => Some(format!(
                "'{task}' failed after {commits} commit(s): outcome={:?} message={} trace={:?}",
                result.outcome,
                result.message.unwrap_or_default(),
                result.trace_summary
            )),
            Err(err) => Some(format!("'{task}' failed after {commits} commit(s): {err}")),
        };
        if let Some(message) = failure {
            let effects_json =
                serde_json::to_value(&effects).unwrap_or_else(|_| JsonValue::Array(Vec::new()));
            return Err(RuntimeError::TaskFailed {
                message,
                effects: effects_json,
            });
        }
        Ok(StoreCommit {
            store: store_name.to_string(),
            revision,
            committed: last_committed,
            effects,
        })
    }

    fn commit_fields(
        &mut self,
        store_name: &str,
        cause: &str,
        next: &Map<String, JsonValue>,
        writes: &[FieldWrite],
    ) -> Result<StoreCommit, RuntimeError> {
        let instance = &self.stores[store_name];
        let mut transitioned = Vec::new();
        for (name, from, to) in writes {
            let Some(field) = instance.schema.fields.iter().find(|f| &f.name == name) else {
                continue;
            };
            match field.kind {
                MirStoreFieldKind::Const | MirStoreFieldKind::Derive => {
                    return Err(session_error(format!(
                        "'{cause}' wrote read-only field '{name}'; nothing committed"
                    )));
                }
                _ if field.lifecycle.is_some() => {
                    check_lifecycle(field, from, to)?;
                    transitioned.push(name.as_str());
                }
                _ => {}
            }
        }

        let mut diff = Map::new();
        let mut history_diff = Map::new();
        for field in stored_fields(&instance.schema) {
            let old = instance
                .fields
                .get(&field.name)
                .cloned()
                .unwrap_or(JsonValue::Null);
            let Some(new) = next.get(&field.name) else {
                continue;
            };
            if *new == old && !transitioned.contains(&field.name.as_str()) {
                continue;
            }
            if field.kind == MirStoreFieldKind::Const {
                return Err(session_error(format!(
                    "'{cause}' wrote read-only field '{}'; nothing committed",
                    field.name
                )));
            }
            let change = JsonValue::Array(vec![old, new.clone()]);
            if field.kind != MirStoreFieldKind::Transient {
                history_diff.insert(field.name.clone(), change.clone());
            }
            diff.insert(field.name.clone(), change);
        }

        let Some(instance) = self.stores.get_mut(store_name) else {
            return Err(session_error(format!("no state named '{store_name}'")));
        };
        if diff.is_empty() {
            return Ok(StoreCommit {
                store: store_name.to_string(),
                revision: instance.revision,
                committed: None,
                effects: Vec::new(),
            });
        }
        for (name, change) in &diff {
            instance.fields.insert(name.clone(), change[1].clone());
        }
        instance.revision += 1;
        let revision = StoreRevision {
            store: store_name.to_string(),
            revision: instance.revision,
            cause: cause.to_string(),
            diff,
            history_diff,
        };
        self.log.push(revision.clone());
        Ok(StoreCommit {
            store: store_name.to_string(),
            revision: instance.revision,
            committed: Some(revision),
            effects: Vec::new(),
        })
    }
}

/// Records every field write a store mutation makes, in order, so lifecycle
/// edges are validated per transition (including same-value self-edges) rather
/// than from the net diff.
struct WriteRecorder<'a> {
    inner: &'a mut dyn CapabilityHost,
    writes: Vec<FieldWrite>,
}

/// One field write: `(field, old, new)`.
type FieldWrite = (String, JsonValue, JsonValue);

impl CapabilityHost for WriteRecorder<'_> {
    fn call(&mut self, call: &CapabilityCall) -> Result<JsonValue, HostCallError> {
        if call.capability == "core.set_fields" {
            let input = call
                .args
                .get("input")
                .or_else(|| call.args.get("__input"))
                .and_then(JsonValue::as_object);
            if let Some(fields) = call.args.get("fields").and_then(JsonValue::as_object) {
                for (name, to) in fields {
                    let from = input
                        .and_then(|i| i.get(name))
                        .cloned()
                        .unwrap_or(JsonValue::Null);
                    self.writes.push((name.clone(), from, to.clone()));
                }
            }
        }
        self.inner.call(call)
    }
}

/// In-flight store state for a task run; merged back into the session after.
struct TaskCommitSink {
    store: String,
    schema: MirStore,
    fields: Map<String, JsonValue>,
    revision: u64,
    log: Vec<StoreRevision>,
    last_committed: Option<StoreRevision>,
}

impl TaskCommitSink {
    fn commit(
        &mut self,
        cause: &str,
        next: &Map<String, JsonValue>,
        writes: &[FieldWrite],
    ) -> Result<(), String> {
        let mut transitioned = Vec::new();
        for (name, from, to) in writes {
            let Some(field) = self.schema.fields.iter().find(|f| &f.name == name) else {
                continue;
            };
            match field.kind {
                MirStoreFieldKind::Const | MirStoreFieldKind::Derive => {
                    return Err(format!(
                        "'{cause}' wrote read-only field '{name}'; nothing committed"
                    ));
                }
                _ if field.lifecycle.is_some() => {
                    check_lifecycle(field, from, to).map_err(|e| e.to_string())?;
                    transitioned.push(name.as_str());
                }
                _ => {}
            }
        }

        let mut diff = Map::new();
        let mut history_diff = Map::new();
        for field in stored_fields(&self.schema) {
            let old = self
                .fields
                .get(&field.name)
                .cloned()
                .unwrap_or(JsonValue::Null);
            let Some(new) = next.get(&field.name) else {
                continue;
            };
            if *new == old && !transitioned.contains(&field.name.as_str()) {
                continue;
            }
            if field.kind == MirStoreFieldKind::Const {
                return Err(format!(
                    "'{cause}' wrote read-only field '{}'; nothing committed",
                    field.name
                ));
            }
            let change = JsonValue::Array(vec![old, new.clone()]);
            if field.kind != MirStoreFieldKind::Transient {
                history_diff.insert(field.name.clone(), change.clone());
            }
            diff.insert(field.name.clone(), change);
        }
        if diff.is_empty() {
            return Ok(());
        }
        for (name, change) in &diff {
            self.fields.insert(name.clone(), change[1].clone());
        }
        self.revision += 1;
        let revision = StoreRevision {
            store: self.store.clone(),
            revision: self.revision,
            cause: cause.to_string(),
            diff,
            history_diff,
        };
        self.last_committed = Some(revision.clone());
        self.log.push(revision);
        Ok(())
    }
}

/// Writes buffered while a task is inside `call` of a mutation. Committed
/// once, when the outermost mutation call returns (RFC-0007 F.3).
struct TxnFrame {
    name: String,
    writes: Vec<FieldWrite>,
    next: Option<Map<String, JsonValue>>,
}

/// Host wrapper for store tasks. The task's own `set_fields` commits
/// immediately. Writes inside a called mutation stay in [`TxnFrame`] until
/// that call returns. Capability ops (not pure core helpers) become receipts.
struct TaskHost<'a> {
    sink: &'a mut TaskCommitSink,
    cause: String,
    action: Option<String>,
    inner: &'a mut dyn CapabilityHost,
    effects: &'a mut Vec<EffectReceipt>,
    effect_seq: &'a mut u64,
    txns: Vec<TxnFrame>,
}

impl CapabilityHost for TaskHost<'_> {
    fn enter_call(&mut self, name: &str, kind: &MirFunctionKind) {
        if matches!(kind, MirFunctionKind::Mutation) {
            self.txns.push(TxnFrame {
                name: name.to_string(),
                writes: Vec::new(),
                next: None,
            });
        }
    }

    fn leave_call(&mut self, name: &str, failed: bool) -> Result<(), HostCallError> {
        let top_is_call = self.txns.last().is_some_and(|frame| frame.name == name);
        if !top_is_call {
            return Ok(());
        }
        let Some(frame) = self.txns.pop() else {
            return Ok(());
        };
        if failed {
            return Ok(());
        }
        if let Some(parent) = self.txns.last_mut() {
            parent.writes.extend(frame.writes);
            if frame.next.is_some() {
                parent.next = frame.next;
            }
            return Ok(());
        }
        if let Some(next) = frame.next {
            self.sink
                .commit(&self.cause, &next, &frame.writes)
                .map_err(HostCallError::Fatal)?;
        }
        Ok(())
    }

    fn refuse_call(&mut self, module: Option<&str>, op: &str, args: &JsonValue, message: &str) {
        self.push_receipt(module, op, args, "failed", Some(message.to_string()));
    }

    fn call(&mut self, call: &CapabilityCall) -> Result<JsonValue, HostCallError> {
        let is_apply = call.capability == "core.set_fields";
        if is_apply {
            let writes = field_writes_of(call);
            let output = self.inner.call(call)?;
            let next = output.as_object().cloned().unwrap_or_default();
            if let Some(frame) = self.txns.last_mut() {
                // Validate now so a bad transition fails this step. The
                // revision is committed in `leave_call` if the mutation returns.
                validate_writes(&self.sink.schema, &self.cause, &writes)
                    .map_err(HostCallError::Fatal)?;
                frame.writes.extend(writes);
                frame.next = Some(next);
            } else {
                self.sink
                    .commit(&self.cause, &next, &writes)
                    .map_err(HostCallError::Fatal)?;
            }
            return Ok(output);
        }

        let module = call.module.as_deref();
        match self.inner.call(call) {
            Ok(output) => {
                self.push_receipt(module, &call.op, &call.args, "acknowledged", None);
                Ok(output)
            }
            Err(err) => {
                let message = match &err {
                    HostCallError::Retryable(m) | HostCallError::Fatal(m) => m.clone(),
                };
                self.push_receipt(module, &call.op, &call.args, "failed", Some(message));
                Err(err)
            }
        }
    }
}

impl TaskHost<'_> {
    fn push_receipt(
        &mut self,
        module: Option<&str>,
        op: &str,
        args: &JsonValue,
        status: &str,
        error: Option<String>,
    ) {
        let module_name = module.unwrap_or("core");
        if !is_capability_effect(module_name, op) {
            return;
        }
        *self.effect_seq += 1;
        let mut args = args.clone();
        if let Some(obj) = args.as_object_mut() {
            obj.remove("__input");
        }
        self.effects.push(EffectReceipt {
            effect: *self.effect_seq,
            op: format!("{module_name}.{op}"),
            args,
            cause: EffectCause {
                action: self.action.clone(),
                task: self.cause.clone(),
                generation: None,
            },
            store: self.sink.store.clone(),
            revision: self.sink.revision,
            status: status.to_string(),
            error,
        });
    }
}

fn field_writes_of(call: &CapabilityCall) -> Vec<FieldWrite> {
    let mut writes = Vec::new();
    let input = call
        .args
        .get("input")
        .or_else(|| call.args.get("__input"))
        .and_then(JsonValue::as_object);
    if let Some(fields) = call.args.get("fields").and_then(JsonValue::as_object) {
        for (name, to) in fields {
            let from = input
                .and_then(|i| i.get(name))
                .cloned()
                .unwrap_or(JsonValue::Null);
            writes.push((name.clone(), from, to.clone()));
        }
    }
    writes
}

/// Lifecycle and read-only checks shared with [`TaskCommitSink::commit`], so a
/// bad write fails the step before the mutation's transaction commits.
fn validate_writes(schema: &MirStore, cause: &str, writes: &[FieldWrite]) -> Result<(), String> {
    for (name, from, to) in writes {
        let Some(field) = schema.fields.iter().find(|f| &f.name == name) else {
            continue;
        };
        match field.kind {
            MirStoreFieldKind::Const | MirStoreFieldKind::Derive => {
                return Err(format!(
                    "'{cause}' wrote read-only field '{name}'; nothing committed"
                ));
            }
            _ if field.lifecycle.is_some() => {
                check_lifecycle(field, from, to).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Receipts are for capability ops (network, io, secrets, state modules).
/// Pure helpers such as `core.value` and `core.echo` are not effects.
fn is_capability_effect(module: &str, op: &str) -> bool {
    match find_op_spec(module, op) {
        Some(spec) => matches!(
            spec.effect,
            SignatureEffect::Network
                | SignatureEffect::Io
                | SignatureEffect::Secrets
                | SignatureEffect::State
        ),
        None => !matches!(module, "core" | "call" | "flow" | "runtime"),
    }
}

fn stored_fields(schema: &MirStore) -> impl Iterator<Item = &MirStoreField> {
    schema
        .fields
        .iter()
        .filter(|f| f.kind != MirStoreFieldKind::Derive)
}

fn check_lifecycle(
    field: &MirStoreField,
    old: &JsonValue,
    new: &JsonValue,
) -> Result<(), RuntimeError> {
    let Some(lifecycle) = &field.lifecycle else {
        return Ok(());
    };
    let (from, to) = (
        old.as_str().unwrap_or_default(),
        new.as_str().unwrap_or_default(),
    );
    if lifecycle.terminals.iter().any(|t| t == from) {
        return Err(session_error(format!(
            "invalid_transition: '{}' is terminal in state machine '{}'; nothing committed",
            from, lifecycle.machine
        )));
    }
    if !lifecycle
        .transitions
        .iter()
        .any(|(f, t)| f == from && t == to)
    {
        return Err(session_error(format!(
            "invalid_transition: state machine '{}' has no edge {from} -> {to} for field '{}'; nothing committed",
            lifecycle.machine, field.name
        )));
    }
    Ok(())
}

/// Writes each action's id into its node's `events.<event>.id`.
fn attach_action_ids(node: &mut JsonValue, actions: &[ViewAction]) {
    let key = node
        .get("key")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_string();
    if let Some(events) = node.get_mut("events").and_then(JsonValue::as_object_mut) {
        for (event, binding) in events.iter_mut() {
            // Last match: if two rows ever shared a key, the later epoch is the live id.
            if let Some(action) = actions.iter().rev().find(|a| a.key == key && &a.event == event)
            {
                binding["id"] = JsonValue::from(action.id.clone());
            }
        }
    }
    if let Some(children) = node.get_mut("children").and_then(JsonValue::as_array_mut) {
        for child in children {
            attach_action_ids(child, actions);
        }
    }
}

fn action_id(key: &str, event: &str, epoch: u64) -> String {
    format!("{key}#{event}@{epoch}")
}

/// Merges view-bound args with an invoke-time payload. Every name in
/// `payload_params` must appear in `payload`; unknown payload keys are rejected.
/// JSON values are checked against the target parameter's declared type.
fn merge_action_args(
    artifact: &ArtifactEnvelope,
    target: &str,
    mut args: Map<String, JsonValue>,
    payload_params: &[String],
    payload: Option<&JsonValue>,
    action_id: &str,
) -> Result<Option<JsonValue>, RuntimeError> {
    let payload_map = match payload {
        None => None,
        Some(JsonValue::Object(map)) => Some(map),
        Some(_) => {
            return Err(session_error(format!(
                "action '{action_id}': payload must be a JSON object"
            )));
        }
    };
    if let Some(map) = payload_map {
        for key in map.keys() {
            if !payload_params.iter().any(|p| p == key) {
                return Err(session_error(format!(
                    "action '{action_id}': unknown payload param '{key}'"
                )));
            }
        }
        for (key, value) in map {
            if let Some(expected) = param_type_name(artifact, target, key)
                && !json_matches_type(value, expected)
            {
                return Err(session_error(format!(
                    "action '{action_id}': payload param '{key}' expects {expected}, found {}",
                    json_type_name(value)
                )));
            }
            args.insert(key.clone(), value.clone());
        }
    }
    for name in payload_params {
        if !args.contains_key(name) {
            return Err(session_error(format!(
                "action '{action_id}' requires payload param '{name}'"
            )));
        }
    }
    if args.is_empty() {
        Ok(None)
    } else {
        Ok(Some(JsonValue::Object(args)))
    }
}

fn param_type_name<'a>(
    artifact: &'a ArtifactEnvelope,
    target: &str,
    name: &str,
) -> Option<&'a str> {
    artifact
        .payload
        .mir
        .functions
        .iter()
        .find(|f| f.name == target)
        .and_then(|f| f.params.iter().find(|p| p.name == name))
        .and_then(|p| p.type_name.as_deref())
}

fn json_matches_type(value: &JsonValue, type_name: &str) -> bool {
    let type_name = type_name.strip_suffix('!').unwrap_or(type_name);
    match type_name {
        "String" => value.is_string(),
        "Int" => value.as_i64().is_some() || value.as_u64().is_some(),
        "Float" => value.is_number(),
        "Bool" => value.is_boolean(),
        _ => true,
    }
}

fn json_type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "Bool",
        JsonValue::Number(n) if n.is_i64() || n.is_u64() => "Int",
        JsonValue::Number(_) => "Float",
        JsonValue::String(_) => "String",
        JsonValue::Array(_) => "List",
        JsonValue::Object(_) => "Object",
    }
}

/// The content of the first `Text` in `nodes`, depth first: a node's accessible name
/// when it has no `label`.
fn first_text(nodes: &[JsonValue]) -> Option<String> {
    nodes.iter().find_map(|node| {
        if node.get("$prim").and_then(JsonValue::as_str) == Some("Text")
            && let Some(content) = node.pointer("/props/content")
        {
            return Some(display(content));
        }
        first_text(node.get("children")?.as_array()?)
    })
}

fn display(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Evaluates a view prop or binding argument against the store snapshot,
/// including `{$state.path}` interpolation in strings.
fn eval_view_value(value: &JsonValue, scope: &JsonValue) -> Result<JsonValue, String> {
    let resolved = interpolate(&resolve(value, scope), value, scope);
    match first_error(&resolved) {
        Some(message) => Err(message.to_string()),
        None => Ok(resolved),
    }
}

/// Interpolates the strings of `resolved` that were literal strings in `original`.
fn interpolate(resolved: &JsonValue, original: &JsonValue, scope: &JsonValue) -> JsonValue {
    match (resolved, original) {
        (JsonValue::String(s), JsonValue::String(_)) if s.contains("{$") => {
            let mut out = String::with_capacity(s.len());
            let mut rest = s.as_str();
            while let Some(pos) = rest.find("{$") {
                out.push_str(&rest[..pos]);
                rest = &rest[pos + 2..];
                let end = rest.find('}').unwrap_or(rest.len());
                let path = rest[..end].strip_prefix("state.").unwrap_or(&rest[..end]);
                out.push_str(&display(
                    &select_path(scope, path).unwrap_or(JsonValue::Null),
                ));
                rest = rest.get(end + 1..).unwrap_or("");
            }
            out.push_str(rest);
            JsonValue::from(out)
        }
        (JsonValue::Array(items), JsonValue::Array(originals)) => JsonValue::Array(
            items
                .iter()
                .zip(originals)
                .map(|(v, o)| interpolate(v, o, scope))
                .collect(),
        ),
        (JsonValue::Object(map), JsonValue::Object(originals)) => JsonValue::Object(
            map.iter()
                .map(|(k, v)| {
                    let value = match originals.get(k) {
                        Some(o) => interpolate(v, o, scope),
                        None => v.clone(),
                    };
                    (k.clone(), value)
                })
                .collect(),
        ),
        (other, _) => other.clone(),
    }
}

/// Evaluates a lowered store value (default or derive) against `fields`.
fn eval_store_value(
    value: &JsonValue,
    fields: &Map<String, JsonValue>,
) -> Result<JsonValue, String> {
    let root = JsonValue::Object(fields.clone());
    let resolved = resolve(value, &root);
    match first_error(&resolved) {
        Some(message) => Err(message.to_string()),
        None => Ok(resolved),
    }
}

fn resolve(value: &JsonValue, root: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            if map.len() == 1 {
                if let Some(path) = map.get("$var").and_then(JsonValue::as_str) {
                    let path = path.strip_prefix("state.").unwrap_or(path);
                    return select_path(root, path).unwrap_or(JsonValue::Null);
                }
                if let Some(symbol) = map.get("$symbol").and_then(JsonValue::as_str) {
                    return JsonValue::from(symbol);
                }
            }
            if let Some((op, args)) = expr_parts(map) {
                return evaluate(op, args, |arg| resolve(arg, root));
            }
            JsonValue::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), resolve(v, root)))
                    .collect(),
            )
        }
        JsonValue::Array(items) => {
            JsonValue::Array(items.iter().map(|v| resolve(v, root)).collect())
        }
        other => other.clone(),
    }
}

fn type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "Bool",
        JsonValue::Number(_) => "Number",
        JsonValue::String(_) => "String",
        JsonValue::Array(_) => "List",
        JsonValue::Object(_) => "Object",
    }
}

/// Where a node is being evaluated: its runtime key, the scope, and the view depth.
struct Place<'a> {
    key: &'a str,
    scope: &'a JsonValue,
    depth: usize,
}

/// A `for` row key. Empty strings, nulls, bools and objects are not identity.
/// `/` and `#` are escaped so they cannot splice into a node path or an action id.
fn row_key(value: &JsonValue) -> Result<String, String> {
    let raw = match value {
        JsonValue::String(s) if !s.is_empty() => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(_) => {
            return Err("for key must be a non-empty string or number".to_string());
        }
        other => {
            return Err(format!(
                "for key must be a non-empty string or number, found {}",
                type_name(other)
            ));
        }
    };
    Ok(escape_key_segment(&raw))
}

fn escape_key_segment(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '/' => out.push_str("%2F"),
            '#' => out.push_str("%23"),
            other => out.push(other),
        }
    }
    out
}
