//! Headless state-store session (RFC-0007 Part A.3).
//!
//! A store is a runtime object separate from the pipeline [`AgentState`](crate::AgentState).
//! Each invoked mutation runs against the store's latest committed fields and
//! commits as one transaction: one revision, with a complete runtime diff and a
//! history diff that omits `transient` fields. A failed run commits nothing.

use std::collections::BTreeMap;

use grapheme_artifact::expr::{evaluate, expr_parts, first_error, select_path};
use grapheme_artifact::{
    ArtifactEnvelope, ExecutionOutcome, MirFunctionKind, MirStore, MirStoreField, MirStoreFieldKind,
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

/// Owns one instance of every store declared by an artifact.
#[derive(Debug, Clone)]
pub struct StoreSession {
    artifact: ArtifactEnvelope,
    stores: BTreeMap<String, StoreInstance>,
    log: Vec<StoreRevision>,
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
