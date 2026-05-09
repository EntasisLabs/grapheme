/// ─────────────────────────────────────────────────────────────
///  AgentQL  —  AgentState
///  The universal state object that threads through every pipeline step.
///  Every |> step receives the previous AgentState and returns a new one.
///  Immutable by convention — each step produces a fresh snapshot.
/// ─────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::ast::ModuleProposal;

// ── Step Result ───────────────────────────────────────────────

/// The outcome of a single pipeline step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// The step index in the pipeline (0-based)
    pub index: usize,
    /// Name of the operation that ran: "Database.query"
    pub op: String,
    /// What this step returned
    pub output: JsonValue,
    /// Whether this step succeeded
    pub ok: bool,
    /// Any error message from this step
    pub error: Option<String>,
}

// ── Agent Error ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentError {
    pub step: usize,
    pub code: String,
    pub message: String,
}

// ── AgentState ────────────────────────────────────────────────

/// Threads through every |> step in a pipeline.
/// Queries only produce new state; mutations may also have side effects.
///
/// The AI can inspect this at any point via:
///   state { current diff errors pipeline proposed }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// The current output value — result of the last completed step
    pub current: JsonValue,

    /// Diff between the previous step's output and this step's output
    /// (null if this is the first step or nothing changed)
    pub diff: Option<JsonValue>,

    /// All errors encountered so far in this pipeline run
    pub errors: Vec<AgentError>,

    /// The full history of step results in this pipeline
    pub pipeline: Vec<StepResult>,

    /// Modules the AI has proposed but which haven't been approved yet
    pub proposed: Vec<ProposedModule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedModule {
    pub proposal: ModuleProposal,
    pub status: ProposalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected(String),
}

impl AgentState {
    /// Create a fresh empty state at the start of a pipeline
    pub fn new() -> Self {
        AgentState {
            current:  JsonValue::Null,
            diff:     None,
            errors:   vec![],
            pipeline: vec![],
            proposed: vec![],
        }
    }

    /// Advance state after a successful step
    pub fn advance(&self, index: usize, op: String, output: JsonValue) -> Self {
        let diff = compute_diff(&self.current, &output);

        let mut pipeline = self.pipeline.clone();
        pipeline.push(StepResult {
            index,
            op,
            output: output.clone(),
            ok: true,
            error: None,
        });

        AgentState {
            current: output,
            diff,
            errors: self.errors.clone(),
            pipeline,
            proposed: self.proposed.clone(),
        }
    }

    /// Advance state after a failed step (errors accumulate, current unchanged)
    pub fn fail(&self, index: usize, op: String, code: String, message: String) -> Self {
        let mut errors = self.errors.clone();
        errors.push(AgentError { step: index, code, message: message.clone() });

        let mut pipeline = self.pipeline.clone();
        pipeline.push(StepResult {
            index,
            op,
            output: JsonValue::Null,
            ok: false,
            error: Some(message),
        });

        AgentState {
            current: self.current.clone(),
            diff: None,
            errors,
            pipeline,
            proposed: self.proposed.clone(),
        }
    }

    /// Register a module proposal from the AI
    pub fn propose(&self, proposal: ModuleProposal) -> Self {
        let mut proposed = self.proposed.clone();
        proposed.push(ProposedModule {
            proposal,
            status: ProposalStatus::Pending,
        });
        AgentState {
            proposed,
            ..self.clone()
        }
    }

    /// Returns true if there are any errors in the current run
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Serialize the state to JSON for returning to the AI agent
    pub fn to_json(&self) -> JsonValue {
        serde_json::to_value(self).unwrap_or(JsonValue::Null)
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Diff Helper ───────────────────────────────────────────────

/// Compute a simple structural diff between two JSON values.
/// Returns None if they are identical.
fn compute_diff(prev: &JsonValue, next: &JsonValue) -> Option<JsonValue> {
    if prev == next {
        return None;
    }

    match (prev, next) {
        (JsonValue::Object(p), JsonValue::Object(n)) => {
            let mut diff = serde_json::Map::new();

            // Keys added or changed
            for (k, v) in n {
                match p.get(k) {
                    None       => { diff.insert(format!("+{k}"), v.clone()); }
                    Some(pv) if pv != v => { diff.insert(format!("~{k}"), v.clone()); }
                    _ => {}
                }
            }
            // Keys removed
            for k in p.keys() {
                if !n.contains_key(k) {
                    diff.insert(format!("-{k}"), JsonValue::Null);
                }
            }

            if diff.is_empty() { None } else { Some(JsonValue::Object(diff)) }
        }
        // For non-objects just show before/after
        _ => Some(serde_json::json!({ "from": prev, "to": next })),
    }
}
