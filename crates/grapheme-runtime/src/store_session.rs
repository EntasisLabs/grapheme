//! Headless state-store session (RFC-0007 Part A.3).
//!
//! A store is a runtime object separate from the pipeline [`AgentState`](crate::AgentState).
//! Each invoked mutation runs against the store's latest committed fields and
//! commits as one transaction: one revision, with a complete runtime diff and a
//! history diff that omits `transient` fields. A failed run commits nothing.

use std::collections::{BTreeMap, BTreeSet};

use grapheme_artifact::expr::{evaluate, expr_parts, first_error, select_path};
use grapheme_artifact::{
    ArtifactEnvelope, ExecutionOutcome, MirFunctionKind, MirStore, MirStoreField, MirStoreFieldKind,
    MirView, MirViewNode, VIEW_IR_VERSION,
};
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

/// The result of invoking a store mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreCommit {
    pub store: String,
    /// The store's revision after the invoke. Unchanged when nothing changed.
    pub revision: u64,
    /// `None` when the run changed no field, so no revision was committed.
    pub committed: Option<StoreRevision>,
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

/// A node event binding and its epoch (RFC-0007 I.3).
#[derive(Debug, Clone)]
struct Binding {
    view: String,
    target: String,
    args: Map<String, JsonValue>,
    epoch: u64,
    mounted: bool,
}

/// An event binding found while evaluating a view, before it has an epoch.
struct BoundEvent {
    key: String,
    event: String,
    target: String,
    args: Map<String, JsonValue>,
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
        })
    }

    pub fn store_names(&self) -> impl Iterator<Item = &str> {
        self.stores.keys().map(String::as_str)
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
        Ok(snapshot)
    }

    /// Invokes the action `id` from a rendered view's current action list. The
    /// rendered views are re-evaluated first, so an id whose binding changed
    /// since it was handed out is rejected with `stale_action`.
    pub fn invoke_action(
        &mut self,
        id: &str,
        options: RuntimeOptions,
        host: &mut dyn CapabilityHost,
    ) -> Result<StoreCommit, RuntimeError> {
        for view in self.rendered.clone() {
            let (_, events) = self.evaluate_view(&view)?;
            self.bind(&view, events);
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
        self.invoke(
            &binding.target,
            Some(JsonValue::Object(binding.args)),
            options,
            host,
        )
    }

    fn evaluate_view(&self, name: &str) -> Result<(ViewSnapshot, Vec<BoundEvent>), RuntimeError> {
        let view = self.find_view(name)?;
        let (scope, revision) = match &view.store {
            Some(store) => (self.snapshot(store)?, self.revision(store)),
            None => (JsonValue::Object(Map::new()), None),
        };
        let mut events = Vec::new();
        let root = self.evaluate_node(view, &view.root, &view.root.key, &scope, 0, &mut events)?;
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

    /// Evaluates `node` with its key rebased from the view's root key onto `key`,
    /// expanding view references in place.
    fn evaluate_node(
        &self,
        view: &MirView,
        node: &MirViewNode,
        key: &str,
        scope: &JsonValue,
        depth: usize,
        events: &mut Vec<BoundEvent>,
    ) -> Result<JsonValue, RuntimeError> {
        let fail = |message: String| session_error(format!("view '{}', element '{key}': {message}", view.name));
        if let Some(name) = &node.view {
            if depth >= MAX_VIEW_DEPTH {
                return Err(fail(format!("view references nest deeper than {MAX_VIEW_DEPTH}")));
            }
            let referenced = self.find_view(name)?;
            let mut out = self.evaluate_node(referenced, &referenced.root, key, scope, depth + 1, events)?;
            out["view"] = JsonValue::from(name.clone());
            return Ok(out);
        }

        let mut out = Map::new();
        if let Some(prim) = &node.prim {
            out.insert("$prim".to_string(), JsonValue::from(prim.clone()));
        }
        out.insert("key".to_string(), JsonValue::from(key));
        if let Some(role) = &node.role {
            out.insert("role".to_string(), JsonValue::from(role.clone()));
        }
        let mut props = Map::new();
        for (name, value) in &node.props {
            let value = eval_view_value(value, scope).map_err(|e| fail(format!("prop '{name}': {e}")))?;
            props.insert(name.clone(), value);
        }
        let mut children = Vec::new();
        for child in &node.children {
            let suffix = child.key.strip_prefix(&node.key).unwrap_or(&child.key);
            children.push(self.evaluate_node(view, child, &format!("{key}{suffix}"), scope, depth, events)?);
        }
        let label = props
            .get("label")
            .and_then(JsonValue::as_str)
            .map(str::to_string)
            .or_else(|| first_text(&children));
        for (event, binding) in &node.events {
            let mut args = Map::new();
            for (name, value) in &binding.args {
                let value = eval_view_value(value, scope)
                    .map_err(|e| fail(format!("argument '{name}' of '{}': {e}", binding.target)))?;
                args.insert(name.clone(), value);
            }
            events.push(BoundEvent {
                key: key.to_string(),
                event: event.clone(),
                target: binding.target.clone(),
                args,
                role: node.role.clone(),
                label: label.clone(),
            });
        }
        if !node.events.is_empty() {
            let names: Vec<JsonValue> = node.events.keys().cloned().map(JsonValue::from).collect();
            out.insert("events".to_string(), JsonValue::Array(names));
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
    /// with the same target and arguments; otherwise the epoch advances.
    fn bind(&mut self, view: &str, events: Vec<BoundEvent>) -> Vec<ViewAction> {
        let mut seen = BTreeSet::new();
        let mut actions = Vec::new();
        for event in events {
            let slot = (event.key.clone(), event.event.clone());
            let binding = self.bindings.entry(slot.clone()).or_insert_with(|| Binding {
                view: view.to_string(),
                target: event.target.clone(),
                args: event.args.clone(),
                epoch: 0,
                mounted: false,
            });
            if !binding.mounted
                || binding.view != view
                || binding.target != event.target
                || binding.args != event.args
            {
                binding.epoch += 1;
            }
            binding.view = view.to_string();
            binding.target = event.target.clone();
            binding.args = event.args.clone();
            binding.mounted = true;
            actions.push(ViewAction {
                id: action_id(&event.key, &event.event, binding.epoch),
                key: event.key,
                event: event.event,
                target: event.target,
                bound_args: event.args,
                payload_params: Vec::new(),
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
        options.initial_state_current = Some(JsonValue::Object(instance.fields.clone()));
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

    /// Runs `mutation` (declared `on` a store) against the store's latest
    /// fields and commits its writes as one revision.
    pub fn invoke(
        &mut self,
        mutation: &str,
        args: Option<JsonValue>,
        options: RuntimeOptions,
        host: &mut dyn CapabilityHost,
    ) -> Result<StoreCommit, RuntimeError> {
        let (store_name, kind) = self.bound_store(mutation)?;
        if matches!(kind, MirFunctionKind::Query) {
            return Err(session_error(format!(
                "'{mutation}' is a query; queries are read-only"
            )));
        }
        let (state, writes) = self.run(mutation, &store_name, args, options, host)?;
        let instance = &self.stores[&store_name];
        let mut transitioned = Vec::new();
        for (name, from, to) in &writes {
            let Some(field) = instance.schema.fields.iter().find(|f| &f.name == name) else {
                continue;
            };
            match field.kind {
                MirStoreFieldKind::Const | MirStoreFieldKind::Derive => {
                    return Err(session_error(format!(
                        "mutation '{mutation}' wrote read-only field '{name}'; nothing committed"
                    )));
                }
                _ if field.lifecycle.is_some() => {
                    check_lifecycle(field, from, to)?;
                    transitioned.push(name.as_str());
                }
                _ => {}
            }
        }

        let next = state.current.as_object().cloned().unwrap_or_default();
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
            // A transition along a declared self-edge is still a change.
            if *new == old && !transitioned.contains(&field.name.as_str()) {
                continue;
            }
            if field.kind == MirStoreFieldKind::Const {
                return Err(session_error(format!(
                    "mutation '{mutation}' wrote read-only field '{}'; nothing committed",
                    field.name
                )));
            }
            let change = JsonValue::Array(vec![old, new.clone()]);
            if field.kind != MirStoreFieldKind::Transient {
                history_diff.insert(field.name.clone(), change.clone());
            }
            diff.insert(field.name.clone(), change);
        }

        let instance = self.stores.get_mut(&store_name).unwrap();
        if diff.is_empty() {
            return Ok(StoreCommit {
                store: store_name,
                revision: instance.revision,
                committed: None,
            });
        }
        for (name, change) in &diff {
            instance.fields.insert(name.clone(), change[1].clone());
        }
        instance.revision += 1;
        let revision = StoreRevision {
            store: store_name.clone(),
            revision: instance.revision,
            cause: mutation.to_string(),
            diff,
            history_diff,
        };
        self.log.push(revision.clone());
        Ok(StoreCommit {
            store: store_name,
            revision: instance.revision,
            committed: Some(revision),
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

fn action_id(key: &str, event: &str, epoch: u64) -> String {
    format!("{key}#{event}@{epoch}")
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
                out.push_str(&display(&select_path(scope, path).unwrap_or(JsonValue::Null)));
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
