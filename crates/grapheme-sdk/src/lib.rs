//! Embedded Grapheme SDK for in-process compile and execution workflows.
//!
//! The SDK wraps compiler and runtime crates with an ergonomic builder API,
//! structured output formatting, and AOT helper entrypoints.

use grapheme_artifact::{build_stage_b_container_from_aot, AotEnvelope, ArtifactEnvelope, ExecutionResult};
use grapheme_compiler::verifier::LintWarning;
use grapheme_compiler::{CompiledScript, Compiler, CompilerError, CompilerOptions};
use grapheme_runtime::{
    CapabilityCall, CapabilityHost, HostCallError, PolicyGuard, RuntimeEngine, RuntimeError,
    RuntimeOptions, TracePolicy,
};
use grapheme_signatures::{find_op_spec, op_output_object_fields, op_output_type, ArgType};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

type HostFactory = Arc<dyn Fn() -> Box<dyn CapabilityHost + Send> + Send + Sync>;
type CapabilityObserver = Arc<dyn Fn(&CapabilityCall) + Send + Sync>;
type CapabilityInterceptor =
    Arc<dyn Fn(&CapabilityCall) -> Option<Result<JsonValue, HostCallError>> + Send + Sync>;

/// Structured output mode for SDK formatting helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredMode {
    /// YAML output.
    Yaml,
    /// JSON output.
    Json,
}

/// Top-level execute payload returned by SDK execution entrypoints.
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteResultPayload {
    /// Executed artifact id.
    pub artifact_id: String,
    /// Runtime execution result summary.
    pub execution: ExecutionResult,
    /// Final runtime state as JSON.
    pub final_state: JsonValue,
    /// Non-fatal compiler warnings collected during compile path.
    pub lint_warnings: Vec<LintWarning>,
}

/// Errors returned by SDK compile/execute/format operations.
#[derive(Debug, Error)]
pub enum GraphemeSdkError {
    #[error(transparent)]
    Compiler(#[from] CompilerError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("aot contract error: {0}")]
    Contract(String),
    #[error("serialize output: {0}")]
    Serialization(String),
}

/// Builder for configuring `GraphemeEngine` runtime behavior.
pub struct GraphemeEngineBuilder {
    runtime_options: RuntimeOptions,
    compiler_options: CompilerOptions,
    module_bindings: HashMap<String, PathBuf>,
    host_factory: Option<HostFactory>,
    capability_observer: Option<CapabilityObserver>,
    capability_interceptor: Option<CapabilityInterceptor>,
}

impl Default for GraphemeEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphemeEngineBuilder {
    /// Create a new builder with runtime defaults.
    pub fn new() -> Self {
        Self {
            runtime_options: RuntimeOptions::default(),
            compiler_options: CompilerOptions::default(),
            module_bindings: HashMap::new(),
            host_factory: None,
            capability_observer: None,
            capability_interceptor: None,
        }
    }

    /// Set policy guard for runtime admission checks.
    pub fn with_policy_guard(mut self, guard: PolicyGuard) -> Self {
        self.runtime_options.policy_guard = guard;
        self
    }

    /// Set trace policy shaping runtime pipeline history.
    pub fn with_trace_policy(mut self, policy: TracePolicy) -> Self {
        self.runtime_options.trace_policy = policy;
        self
    }

    /// Enable or disable artifact integrity verification.
    pub fn with_verify_integrity(mut self, enabled: bool) -> Self {
        self.runtime_options.verify_integrity = enabled;
        self
    }

    /// Enable or disable streaming plain step output.
    pub fn with_stream_step_output(mut self, enabled: bool) -> Self {
        self.runtime_options.stream_step_output = enabled;
        self
    }

    /// Enable or disable strict Stage B container execution mode.
    pub fn with_strict_stage_b_container_execution(mut self, enabled: bool) -> Self {
        self.runtime_options.strict_stage_b_container_execution = enabled;
        self
    }

    /// Set compiler options used by source-based compile helpers.
    pub fn with_compiler_options(mut self, options: CompilerOptions) -> Self {
        self.compiler_options = options;
        self
    }

    /// Set optional maximum step count.
    pub fn with_max_steps(mut self, max_steps: Option<usize>) -> Self {
        self.runtime_options.max_steps = max_steps;
        self
    }

    /// Set optional maximum nested call depth.
    pub fn with_max_call_depth(mut self, max_call_depth: Option<usize>) -> Self {
        self.runtime_options.max_call_depth = max_call_depth;
        self
    }

    /// Bind a module id to a Wasm path for runtime resolution.
    pub fn with_module_path(mut self, module: &str, path: impl Into<PathBuf>) -> Self {
        self.module_bindings
            .insert(module.to_lowercase(), path.into());
        self
    }

    /// Register an observer called for each capability invocation.
    pub fn with_capability_observer<F>(mut self, observer: F) -> Self
    where
        F: Fn(&CapabilityCall) + Send + Sync + 'static,
    {
        self.capability_observer = Some(Arc::new(observer));
        self
    }

    /// Register an interceptor that can override capability call results.
    pub fn with_capability_interceptor<F>(mut self, interceptor: F) -> Self
    where
        F: Fn(&CapabilityCall) -> Option<Result<JsonValue, HostCallError>> + Send + Sync + 'static,
    {
        self.capability_interceptor = Some(Arc::new(interceptor));
        self
    }

    /// Provide a custom host factory for full capability dispatch control.
    pub fn with_host_factory<F>(mut self, host_factory: F) -> Self
    where
        F: Fn() -> Box<dyn CapabilityHost + Send> + Send + Sync + 'static,
    {
        self.host_factory = Some(Arc::new(host_factory));
        self
    }

    /// Build the configured `GraphemeEngine`.
    pub fn build(self) -> GraphemeEngine {
        GraphemeEngine {
            runtime_options: self.runtime_options,
            compiler_options: self.compiler_options,
            module_bindings: self.module_bindings,
            host_factory: self.host_factory,
            capability_observer: self.capability_observer,
            capability_interceptor: self.capability_interceptor,
        }
    }
}

/// High-level embedded engine for compile/execute and AOT helper flows.
pub struct GraphemeEngine {
    runtime_options: RuntimeOptions,
    compiler_options: CompilerOptions,
    module_bindings: HashMap<String, PathBuf>,
    host_factory: Option<HostFactory>,
    capability_observer: Option<CapabilityObserver>,
    capability_interceptor: Option<CapabilityInterceptor>,
}

impl GraphemeEngine {
    /// Create a builder for `GraphemeEngine`.
    pub fn builder() -> GraphemeEngineBuilder {
        GraphemeEngineBuilder::new()
    }

    /// Compile and execute source in one call.
    pub fn execute_source(&self, source: &str) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        let compiled = Compiler::compile_source(source, self.compiler_options.clone())?;
        self.execute_compiled(&compiled)
    }

    /// Compile source into a Stage A AOT envelope.
    pub fn compile_source_to_aot(&self, source: &str) -> Result<AotEnvelope, GraphemeSdkError> {
        let compiled = Compiler::compile_source_to_aot(source, self.compiler_options.clone())?;
        Ok(compiled.aot)
    }

    /// Compile source into Stage B AOT using provided workflow bytes/imports.
    pub fn compile_source_to_aot_stage_b(
        &self,
        source: &str,
        workflow_wasm: &[u8],
        allowed_imports: &[String],
    ) -> Result<AotEnvelope, GraphemeSdkError> {
        let stage_a = self.compile_source_to_aot(source)?;
        build_stage_b_container_from_aot(&stage_a, workflow_wasm, allowed_imports)
            .map_err(|e| GraphemeSdkError::Contract(e.to_string()))
    }

    /// Execute a prebuilt artifact envelope.
    pub fn execute_artifact(
        &self,
        artifact: &ArtifactEnvelope,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        self.execute_artifact_with_lints(artifact, Vec::new())
    }

    /// Execute a prebuilt AOT envelope.
    pub fn execute_aot(&self, aot: &AotEnvelope) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        self.execute_aot_with_lints(aot, Vec::new())
    }

    /// Execute a compiled script produced by compiler APIs.
    pub fn execute_compiled(
        &self,
        compiled: &CompiledScript,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        self.execute_artifact_with_lints(
            &compiled.artifact,
            compiled.compilation.lint_warnings.clone(),
        )
    }

    /// Format execute results as YAML or JSON.
    pub fn format_result(
        &self,
        result: &ExecuteResultPayload,
        mode: StructuredMode,
    ) -> Result<String, GraphemeSdkError> {
        match mode {
            StructuredMode::Json => serde_json::to_string_pretty(result)
                .map_err(|e| GraphemeSdkError::Serialization(e.to_string())),
            StructuredMode::Yaml => serde_yaml::to_string(result)
                .map_err(|e| GraphemeSdkError::Serialization(e.to_string())),
        }
    }

    /// Format an AOT envelope as YAML or JSON.
    pub fn format_aot(&self, aot: &AotEnvelope, mode: StructuredMode) -> Result<String, GraphemeSdkError> {
        match mode {
            StructuredMode::Json => serde_json::to_string_pretty(aot)
                .map_err(|e| GraphemeSdkError::Serialization(e.to_string())),
            StructuredMode::Yaml => serde_yaml::to_string(aot)
                .map_err(|e| GraphemeSdkError::Serialization(e.to_string())),
        }
    }

    fn execute_artifact_with_lints(
        &self,
        artifact: &ArtifactEnvelope,
        lint_warnings: Vec<LintWarning>,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        let mut options = self.runtime_options.clone();
        for (module, path) in &self.module_bindings {
            options
                .module_registry
                .set_wasm_path(module.as_str(), path.clone());
        }

        let runtime = RuntimeEngine::new(options);
        let (state, execution) = if let Some(factory) = &self.host_factory {
            let mut host = factory();
            runtime.execute_artifact(artifact, host.as_mut())?
        } else {
            let mut host = StdlibHost {
                capability_observer: self.capability_observer.clone(),
                capability_interceptor: self.capability_interceptor.clone(),
            };
            runtime.execute_artifact(artifact, &mut host)?
        };

        Ok(ExecuteResultPayload {
            artifact_id: artifact.artifact_id.clone(),
            execution,
            final_state: state.to_json(),
            lint_warnings,
        })
    }

    fn execute_aot_with_lints(
        &self,
        aot: &AotEnvelope,
        lint_warnings: Vec<LintWarning>,
    ) -> Result<ExecuteResultPayload, GraphemeSdkError> {
        let mut options = self.runtime_options.clone();
        for (module, path) in &self.module_bindings {
            options
                .module_registry
                .set_wasm_path(module.as_str(), path.clone());
        }

        let runtime = RuntimeEngine::new(options);
        let (state, execution) = if let Some(factory) = &self.host_factory {
            let mut host = factory();
            runtime
                .execute_aot(aot, host.as_mut())
                .map_err(map_runtime_aot_error)?
        } else {
            let mut host = StdlibHost {
                capability_observer: self.capability_observer.clone(),
                capability_interceptor: self.capability_interceptor.clone(),
            };
            runtime
                .execute_aot(aot, &mut host)
                .map_err(map_runtime_aot_error)?
        };

        Ok(ExecuteResultPayload {
            artifact_id: aot.base_artifact.artifact_id.clone(),
            execution,
            final_state: state.to_json(),
            lint_warnings,
        })
    }
}

fn map_runtime_aot_error(err: RuntimeError) -> GraphemeSdkError {
    match err {
        RuntimeError::ArtifactCompatibilityError(message) => GraphemeSdkError::Contract(message),
        other => GraphemeSdkError::Runtime(other),
    }
}

/// Detail tier for module search explain payloads.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleSearchDetail {
    /// Full guidance payload with operation and usage hints.
    #[default]
    Full,
    /// Concise payload optimized for ranking and short previews.
    Concise,
}

/// Search options for module discovery payload APIs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleSearchOptions {
    /// Enable explain output shape (when false, returns compact module-id list).
    pub explain: bool,
    /// Explain detail tier when explain mode is active.
    #[serde(default)]
    pub detail: ModuleSearchDetail,
    /// Optional maximum number of ranked results.
    pub top: Option<usize>,
    /// Optional minimum relevance threshold.
    pub min_score: Option<f64>,
}

/// Discovery row returned by SDK example listing/search APIs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExampleDiscoveryItem {
    /// Stable example name.
    pub name: String,
    /// Relative path where the example is typically scaffolded.
    pub path: String,
    /// One-line description.
    pub summary: String,
    /// Guidance on when to choose this example.
    pub use_when: String,
    /// Authoring complexity tier.
    pub complexity: String,
    /// Discovery tags.
    pub tags: Vec<String>,
    /// Whether native modules are required.
    pub requires_native_modules: bool,
    /// Suggested CLI run command.
    pub run: String,
}

struct ExampleCatalogEntry {
    name: &'static str,
    path: &'static str,
    summary: &'static str,
    use_when: &'static str,
    complexity: &'static str,
    tags: &'static [&'static str],
    requires_native_modules: bool,
}

const EXAMPLE_CATALOG: &[ExampleCatalogEntry] = &[
    ExampleCatalogEntry {
        name: "main",
        path: "examples/main.gr",
        summary: "Canonical project entrypoint with glyph-based composition.",
        use_when: "You want the default project root flow and main wiring pattern.",
        complexity: "beginner",
        tags: &["entrypoint", "glyph", "composition"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "hello-world",
        path: "examples/hello-world.gr",
        summary: "Smallest end-to-end query pipeline.",
        use_when: "You need a first successful run to validate install/runtime.",
        complexity: "beginner",
        tags: &["intro", "core", "query"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "core-merge",
        path: "examples/core-merge.gr",
        summary: "Shows object merge semantics in core transforms.",
        use_when: "You need to build/reshape state objects.",
        complexity: "beginner",
        tags: &["core", "transform", "object"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "core-filter",
        path: "examples/core-filter.gr",
        summary: "Filters list items by field equality.",
        use_when: "You need list narrowing before later pipeline steps.",
        complexity: "beginner",
        tags: &["core", "list", "filter"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "core-validate-schema",
        path: "examples/core-validate-schema.gr",
        summary: "Validates required fields in payload-like objects.",
        use_when: "You need fast contract checks before side effects.",
        complexity: "intermediate",
        tags: &["core", "validation", "schema"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "request-transform-output",
        path: "examples/request-transform-output.gr",
        summary: "Transforms request data into a structured output envelope.",
        use_when: "You need to normalize or map inbound payloads.",
        complexity: "intermediate",
        tags: &["transform", "mapping", "output"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "mutation-update-preferences",
        path: "examples/mutation-update-preferences.gr",
        summary: "Mutation flow that updates preference-like state.",
        use_when: "You need write-style workflows with controlled state changes.",
        complexity: "intermediate",
        tags: &["mutation", "state", "core"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "mutation-state-machine-apply",
        path: "examples/mutation-state-machine-apply.gr",
        summary: "State-machine style mutation transitions.",
        use_when: "You need explicit status/lifecycle transitions.",
        complexity: "advanced",
        tags: &["mutation", "state-machine", "transition"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "resilience-composition",
        path: "examples/resilience-composition.gr",
        summary: "Shows retry/timeout/loop resilience composition patterns.",
        use_when: "You need robust flows under transient errors.",
        complexity: "advanced",
        tags: &["resilience", "retry", "timeout"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "subscription-heartbeat-readable",
        path: "examples/subscription-heartbeat-readable.gr",
        summary: "Readable heartbeat subscription-style workflow.",
        use_when: "You need periodic signal/event style patterns.",
        complexity: "intermediate",
        tags: &["subscription", "heartbeat", "loop"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "websearch-report",
        path: "examples/websearch-report.gr",
        summary: "Search -> fetch -> clean -> report pipeline.",
        use_when: "You need source-grounded report generation.",
        complexity: "advanced",
        tags: &["websearch", "report", "research"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "web-provider-catalog",
        path: "examples/web-provider-catalog.gr",
        summary: "Discover web providers and render provider ids.",
        use_when: "You need capability-aware provider discovery in-flow.",
        complexity: "beginner",
        tags: &["web", "providers", "discovery"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "web-provider-routing",
        path: "examples/web-provider-routing.gr",
        summary: "Capability-aware provider routing with fallback behavior.",
        use_when: "You need provider-selection control flow with graceful fallback.",
        complexity: "advanced",
        tags: &["web", "routing", "fallback"],
        requires_native_modules: false,
    },
    ExampleCatalogEntry {
        name: "web-xaviv-planned",
        path: "examples/web-xaviv-planned.gr",
        summary: "Planned-provider handling path for unsupported providers.",
        use_when: "You need explicit unsupported-provider branching patterns.",
        complexity: "intermediate",
        tags: &["web", "provider", "planned"],
        requires_native_modules: false,
    },
];

struct ModuleSearchGuidance {
    summary: &'static str,
    use_when: &'static str,
    avoid_when: &'static str,
}

/// Return runtime module manifests currently known to the core runtime catalog.
pub fn discover_module_manifests() -> Vec<grapheme_runtime::ModuleManifest> {
    grapheme_runtime::core_v1_manifests()
}

/// Find a module manifest by id (case-insensitive).
pub fn module_manifest_by_id(module_id: &str) -> Option<grapheme_runtime::ModuleManifest> {
    discover_module_manifests()
        .into_iter()
        .find(|m| m.module_id.eq_ignore_ascii_case(module_id))
}

/// Return curated example paths for a module id.
pub fn curated_examples_for_module(module_id: &str) -> &'static [&'static str] {
    match module_id.to_lowercase().as_str() {
        "http" => &["examples/http-get.gr"],
        "websearch" => &[
            "examples/websearch-materials.gr",
            "examples/websearch-report.gr",
        ],
        "tcp" => &["examples/tcp-connect.gr"],
        "smtp" => &["examples/smtp-send.gr"],
        "sql" => &[
            "examples/sql-query.gr",
            "examples/sql-query-params.gr",
            "examples/sql-transaction.gr",
            "examples/sql-transaction-rollback.gr",
        ],
        "surreal" => &[
            "examples/surreal-select.gr",
            "examples/surreal-query.gr",
            "examples/surreal-select-filtered.gr",
            "examples/surreal-query-vars.gr",
            "examples/surreal-health.gr",
            "examples/surreal-create.gr",
            "examples/surreal-update.gr",
            "examples/surreal-delete.gr",
        ],
        "io" => &["examples/io-list.gr"],
        "memory" => &["examples/memory-roundtrip.gr"],
        "secrets" => &["examples/secrets-handle.gr", "examples/secrets-sign.gr"],
        "json" | "csv" | "yaml" | "html" => &["examples/request-transform-output.gr"],
        "core" => &[
            "examples/core-merge.gr",
            "examples/core-filter.gr",
            "examples/core-validate-schema.gr",
            "examples/mutation-update-preferences.gr",
        ],
        _ => &[],
    }
}

/// Search modules and return compact or explainable ranked payloads.
pub fn modules_search_payload(query: &str, options: &ModuleSearchOptions) -> JsonValue {
    let q = query.to_lowercase();

    if !options.explain {
        let mut matches = discover_module_manifests()
            .into_iter()
            .filter(|m| {
                m.module_id.to_lowercase().contains(&q)
                    || m.exported_ops
                        .iter()
                        .any(|op| op.op.to_lowercase().contains(&q))
            })
            .map(|m| m.module_id)
            .collect::<Vec<_>>();
        matches.sort();

        return JsonValue::Array(matches.into_iter().map(JsonValue::String).collect());
    }

    let mut matches = Vec::new();

    for manifest in discover_module_manifests() {
        let module_id = manifest.module_id;
        let module_match = module_id.to_lowercase().contains(&q);
        let matching_ops = manifest
            .exported_ops
            .iter()
            .filter_map(|op| {
                if op.op.to_lowercase().contains(&q)
                    || format!("{}.{}", module_id, op.op).to_lowercase().contains(&q)
                {
                    Some(op.op.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !(module_match || !matching_ops.is_empty()) {
            continue;
        }

        let mut why_matched = Vec::new();
        if module_match {
            why_matched.push("module_id");
        }
        if !matching_ops.is_empty() {
            why_matched.push("op_name");
        }

        let relevance_score =
            compute_module_relevance_score(&q, &module_id, &matching_ops, module_match);

        let guidance = module_search_guidance(&module_id);
        let related_examples = curated_examples_for_module(&module_id)
            .iter()
            .map(|path| JsonValue::String((*path).to_string()))
            .collect::<Vec<_>>();

        let row = match options.detail {
            ModuleSearchDetail::Concise => serde_json::json!({
                "module_id": module_id,
                "score": relevance_score,
                "why_matched": why_matched,
                "summary": guidance.summary,
                "related_examples": related_examples,
            }),
            ModuleSearchDetail::Full => serde_json::json!({
                "module_id": module_id,
                "score": relevance_score,
                "why_matched": why_matched,
                "matching_ops": matching_ops,
                "summary": guidance.summary,
                "use_when": guidance.use_when,
                "avoid_when": guidance.avoid_when,
                "related_examples": related_examples,
            }),
        };

        matches.push(row);
    }

    matches.sort_by(|a, b| {
        let a_score = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b_score = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);

        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.get("module_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .cmp(b.get("module_id").and_then(|v| v.as_str()).unwrap_or_default()),
            )
    });

    if let Some(min_score) = options.min_score {
        matches.retain(|row| {
            row.get("score")
                .and_then(|v| v.as_f64())
                .map(|score| score >= min_score)
                .unwrap_or(false)
        });
    }

    if let Some(top) = options.top {
        matches.truncate(top);
    }

    serde_json::json!({
        "query": query,
        "detail": match options.detail {
            ModuleSearchDetail::Full => "full",
            ModuleSearchDetail::Concise => "concise",
        },
        "top": options.top,
        "min_score": options.min_score,
        "count": matches.len(),
        "matches": matches,
    })
}

/// Build `modules ops` payload for a query string.
pub fn modules_ops_payload(query: &str) -> JsonValue {
    let q = query.to_lowercase();
    let mut matches = Vec::new();

    for manifest in discover_module_manifests() {
        let module_id = manifest.module_id;
        let module_match = module_id.to_lowercase().contains(&q);

        for op in manifest.exported_ops {
            let full = format!("{}.{}", module_id, op.op);
            if module_match
                || op.op.to_lowercase().contains(&q)
                || full.to_lowercase().contains(&q)
            {
                matches.push(serde_json::json!({
                    "module_id": module_id,
                    "op": op.op,
                    "effect": op.effect,
                    "input_schema_ref": op.input_schema_ref,
                    "output_schema_ref": op.output_schema_ref,
                }));
            }
        }
    }

    matches.sort_by(|a, b| {
        let a_module = a
            .get("module_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let b_module = b
            .get("module_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let a_op = a.get("op").and_then(|v| v.as_str()).unwrap_or_default();
        let b_op = b.get("op").and_then(|v| v.as_str()).unwrap_or_default();
        a_module.cmp(b_module).then(a_op.cmp(b_op))
    });

    serde_json::json!({
        "query": query,
        "matches": matches,
    })
}

#[derive(Serialize)]
struct CompactModuleOp {
    op: String,
    effect: grapheme_runtime::EffectKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_object_type: Option<OperationObjectType>,
    output_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_object_type: Option<OperationObjectType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_schema_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_schema_ref: Option<String>,
}

#[derive(Serialize)]
struct OperationObjectField {
    ty: String,
    required: bool,
}

#[derive(Serialize)]
struct OperationObjectType {
    kind: String,
    required: Vec<String>,
    properties: BTreeMap<String, OperationObjectField>,
}

#[derive(Serialize)]
struct EffectGroup {
    effect: String,
    ops: Vec<String>,
}

#[derive(Serialize)]
struct ModuleOpSummary {
    total_ops: usize,
    typed_ops: usize,
    untyped_ops: usize,
    input_schema_refs: usize,
    output_schema_refs: usize,
}

#[derive(Serialize)]
struct ModuleInfoPayload {
    module_id: String,
    version: String,
    abi: grapheme_runtime::ModuleAbi,
    entrypoint: String,
    required_capabilities: Vec<String>,
    limits: grapheme_runtime::ResourceLimits,
    op_summary: ModuleOpSummary,
    exported_ops_by_effect: Vec<EffectGroup>,
    exported_ops: Vec<CompactModuleOp>,
}

#[derive(Serialize)]
struct ModuleTypesPayload {
    module_id: String,
    type_summary: ModuleOpSummary,
    types_by_effect: Vec<EffectGroup>,
    types: Vec<CompactModuleOp>,
}

fn effect_name(effect: &grapheme_runtime::EffectKind) -> &'static str {
    match effect {
        grapheme_runtime::EffectKind::Pure => "pure",
        grapheme_runtime::EffectKind::Network => "network",
        grapheme_runtime::EffectKind::Io => "io",
        grapheme_runtime::EffectKind::State => "state",
        grapheme_runtime::EffectKind::Secrets => "secrets",
        grapheme_runtime::EffectKind::Control => "control",
    }
}

fn arg_type_label(ty: ArgType) -> &'static str {
    match ty {
        ArgType::String => "string",
        ArgType::Number => "number",
        ArgType::Boolean => "boolean",
        ArgType::Object => "object",
        ArgType::Array => "array",
        ArgType::Any => "any",
    }
}

fn output_type_label(
    module_id: &str,
    op_name: &str,
    output_schema_ref: Option<&str>,
) -> String {
    if let Some(shape) = output_schema_ref {
        return shape.to_string();
    }

    arg_type_label(op_output_type(module_id, op_name)).to_string()
}

fn op_input_object_type(module_id: &str, op_name: &str) -> Option<OperationObjectType> {
    let spec = find_op_spec(module_id, op_name)?;
    if spec.args.is_empty() {
        return None;
    }

    let mut required = Vec::new();
    let mut properties = BTreeMap::new();

    for arg in spec.args {
        if arg.required {
            required.push(arg.name.to_string());
        }

        properties.insert(
            arg.name.to_string(),
            OperationObjectField {
                ty: arg_type_label(arg.ty).to_string(),
                required: arg.required,
            },
        );
    }

    Some(OperationObjectType {
        kind: "object".to_string(),
        required,
        properties,
    })
}

fn op_output_object_type(module_id: &str, op_name: &str) -> Option<OperationObjectType> {
    let fields = op_output_object_fields(module_id, op_name)?;

    let mut required = Vec::new();
    let mut properties = BTreeMap::new();

    for field in fields {
        if field.required {
            required.push(field.name.to_string());
        }

        properties.insert(
            field.name.to_string(),
            OperationObjectField {
                ty: arg_type_label(field.ty).to_string(),
                required: field.required,
            },
        );
    }

    Some(OperationObjectType {
        kind: "object".to_string(),
        required,
        properties,
    })
}

fn compact_module_ops(module_id: &str, ops: &[grapheme_runtime::ExportedOp]) -> Vec<CompactModuleOp> {
    ops.iter()
        .map(|op| CompactModuleOp {
            op: op.op.clone(),
            effect: op.effect.clone(),
            input_object_type: op_input_object_type(module_id, &op.op),
            output_type: output_type_label(module_id, &op.op, op.output_schema_ref.as_deref()),
            output_object_type: op_output_object_type(module_id, &op.op),
            input_schema_ref: op.input_schema_ref.clone(),
            output_schema_ref: op.output_schema_ref.clone(),
        })
        .collect()
}

fn module_op_summary(ops: &[grapheme_runtime::ExportedOp]) -> ModuleOpSummary {
    let total_ops = ops.len();
    let input_schema_refs = ops
        .iter()
        .filter(|op| op.input_schema_ref.is_some())
        .count();
    let output_schema_refs = ops
        .iter()
        .filter(|op| op.output_schema_ref.is_some())
        .count();
    let typed_ops = ops
        .iter()
        .filter(|op| op.input_schema_ref.is_some() || op.output_schema_ref.is_some())
        .count();

    ModuleOpSummary {
        total_ops,
        typed_ops,
        untyped_ops: total_ops.saturating_sub(typed_ops),
        input_schema_refs,
        output_schema_refs,
    }
}

fn grouped_module_ops(ops: &[grapheme_runtime::ExportedOp]) -> Vec<EffectGroup> {
    let mut groups: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();

    for op in ops {
        groups
            .entry(effect_name(&op.effect))
            .or_default()
            .push(op.op.clone());
    }

    groups
        .into_iter()
        .map(|(effect, ops)| EffectGroup {
            effect: effect.to_string(),
            ops,
        })
        .collect()
}

/// Build `modules info` payload for a module id.
pub fn modules_info_payload(module_id: &str) -> Option<JsonValue> {
    let manifest = module_manifest_by_id(module_id)?;
    let op_summary = module_op_summary(&manifest.exported_ops);
    let module_id = manifest.module_id.clone();
    let payload = ModuleInfoPayload {
        module_id: module_id.clone(),
        version: manifest.version,
        abi: manifest.abi,
        entrypoint: manifest.entrypoint,
        required_capabilities: manifest.required_capabilities,
        limits: manifest.limits,
        op_summary,
        exported_ops_by_effect: grouped_module_ops(&manifest.exported_ops),
        exported_ops: compact_module_ops(&module_id, &manifest.exported_ops),
    };

    serde_json::to_value(payload).ok()
}

/// Build `modules types` payload for a module id.
pub fn modules_types_payload(module_id: &str) -> Option<JsonValue> {
    let manifest = module_manifest_by_id(module_id)?;
    let module_id = manifest.module_id.clone();
    let payload = ModuleTypesPayload {
        module_id: module_id.clone(),
        type_summary: module_op_summary(&manifest.exported_ops),
        types_by_effect: grouped_module_ops(&manifest.exported_ops),
        types: compact_module_ops(&module_id, &manifest.exported_ops),
    };

    serde_json::to_value(payload).ok()
}

/// Build `modules examples` payload for a module id.
pub fn modules_examples_payload(module_id: &str) -> Option<JsonValue> {
    let normalized = module_id.to_lowercase();
    let examples = curated_examples_for_module(&normalized);
    if examples.is_empty() {
        return None;
    }

    Some(serde_json::json!({
        "module_id": normalized,
        "examples": examples,
    }))
}

fn module_search_guidance(module_id: &str) -> ModuleSearchGuidance {
    match module_id.to_lowercase().as_str() {
        "core" => ModuleSearchGuidance {
            summary: "General-purpose transforms, branching helpers, and state shaping.",
            use_when: "You need data reshaping, list operations, path access, or debug helpers.",
            avoid_when: "You need external network/database side effects.",
        },
        "web" | "websearch" => ModuleSearchGuidance {
            summary: "Search and research primitives over web providers.",
            use_when: "You need source discovery, provider routing, or report/material generation.",
            avoid_when: "You already have trusted local content and do not need web fetch/search.",
        },
        "http" | "tcp" | "smtp" => ModuleSearchGuidance {
            summary: "Network side-effect modules for transport and external I/O.",
            use_when: "You need outbound calls, socket interactions, or email delivery.",
            avoid_when: "You can complete the workflow with local transforms only.",
        },
        "sql" | "surreal" => ModuleSearchGuidance {
            summary: "Database capability modules for read/write and transactional patterns.",
            use_when: "You need persistent state queries and durable updates.",
            avoid_when: "You only need ephemeral in-memory state.",
        },
        "memory" => ModuleSearchGuidance {
            summary: "In-memory storage/roundtrip examples and lightweight persistence patterns.",
            use_when: "You need temporary memory interactions within bounded runtime scope.",
            avoid_when: "You need durable relational/document persistence.",
        },
        "io" | "docs" | "json" | "csv" | "yaml" | "html" => ModuleSearchGuidance {
            summary: "Document and content transformation helpers.",
            use_when: "You need file/text conversion or structured format transforms.",
            avoid_when: "You need network search or database transactions.",
        },
        "secrets" => ModuleSearchGuidance {
            summary: "Secret handling and signing-oriented capability flows.",
            use_when: "You need governed secret retrieval or signing operations.",
            avoid_when: "Your workflow does not require secret material.",
        },
        _ => ModuleSearchGuidance {
            summary: "Runtime module capability surface.",
            use_when: "You need operation-level capabilities for workflow execution.",
            avoid_when: "No matching capabilities are needed for your task.",
        },
    }
}

fn compute_module_relevance_score(
    query_lower: &str,
    module_id: &str,
    matching_ops: &[String],
    module_match: bool,
) -> f64 {
    let mut score = 0.0;

    if module_id.eq_ignore_ascii_case(query_lower) {
        score += 100.0;
    } else if module_match {
        score += 55.0;
    }

    score += matching_ops.len() as f64 * 7.5;

    if matching_ops
        .iter()
        .any(|op| op.eq_ignore_ascii_case(query_lower))
    {
        score += 35.0;
    }

    score
}

fn example_catalog_row(entry: &ExampleCatalogEntry) -> ExampleDiscoveryItem {
    let mut run = format!("grapheme run {}", entry.path);
    if entry.requires_native_modules {
        run.push_str(" --native-modules");
    }

    ExampleDiscoveryItem {
        name: entry.name.to_string(),
        path: entry.path.to_string(),
        summary: entry.summary.to_string(),
        use_when: entry.use_when.to_string(),
        complexity: entry.complexity.to_string(),
        tags: entry.tags.iter().map(|t| (*t).to_string()).collect(),
        requires_native_modules: entry.requires_native_modules,
        run,
    }
}

/// Discover examples with optional filtering by query/tag/complexity/native requirement.
pub fn discover_examples(
    query: Option<&str>,
    tag: Option<&str>,
    complexity: Option<&str>,
    native_only: bool,
) -> Vec<ExampleDiscoveryItem> {
    let query = query.map(|q| q.to_lowercase());
    let tag = tag.map(|t| t.to_lowercase());
    let complexity = complexity.map(|c| c.to_lowercase());

    EXAMPLE_CATALOG
        .iter()
        .map(example_catalog_row)
        .filter(|item| {
            if native_only && !item.requires_native_modules {
                return false;
            }

            if let Some(ref c) = complexity {
                if item.complexity.to_lowercase() != *c {
                    return false;
                }
            }

            if let Some(ref t) = tag {
                if !item.tags.iter().any(|entry| entry.eq_ignore_ascii_case(t)) {
                    return false;
                }
            }

            if let Some(ref q) = query {
                let tags = item.tags.join(" ").to_lowercase();
                if !(item.name.to_lowercase().contains(q)
                    || item.summary.to_lowercase().contains(q)
                    || item.use_when.to_lowercase().contains(q)
                    || tags.contains(q))
                {
                    return false;
                }
            }

            true
        })
        .collect()
}

/// Get one example discovery row by stable name.
pub fn example_by_name(name: &str) -> Option<ExampleDiscoveryItem> {
    EXAMPLE_CATALOG
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name))
        .map(example_catalog_row)
}

struct StdlibHost {
    capability_observer: Option<CapabilityObserver>,
    capability_interceptor: Option<CapabilityInterceptor>,
}

impl StdlibHost {
    fn resolve_module(call: &CapabilityCall) -> String {
        call.module
            .as_deref()
            .map(|m| m.to_lowercase())
            .or_else(|| call.capability.split('.').next().map(|m| m.to_lowercase()))
            .unwrap_or_default()
    }
}

impl CapabilityHost for StdlibHost {
    fn call(&mut self, call: &CapabilityCall) -> Result<JsonValue, HostCallError> {
        if let Some(observer) = &self.capability_observer {
            observer(call);
        }

        if let Some(interceptor) = &self.capability_interceptor {
            if let Some(outcome) = interceptor(call) {
                return outcome;
            }
        }

        let module = Self::resolve_module(call);
        if let Some(out) = grapheme_stdlib::registry::dispatch(&module, &call.op, &call.args) {
            return Ok(out);
        }

        Err(HostCallError::Fatal(format!(
            "unsupported capability dispatch: module='{}' op='{}'",
            module, call.op
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grapheme_artifact::ExecutionOutcome;
    use grapheme_runtime::PolicyGuard;
    use std::sync::{Arc, Mutex};

    #[test]
    fn modules_search_payload_concise_returns_ranked_matches() {
        let payload = modules_search_payload(
            "web",
            &ModuleSearchOptions {
                explain: true,
                detail: ModuleSearchDetail::Concise,
                top: Some(1),
                min_score: Some(100.0),
            },
        );

        assert_eq!(payload.get("count").and_then(|v| v.as_u64()), Some(1));
        let first = payload
            .get("matches")
            .and_then(|v| v.as_array())
            .and_then(|items| items.first())
            .expect("first ranked match");
        assert_eq!(
            first.get("module_id").and_then(|v| v.as_str()),
            Some("web")
        );
    }

    #[test]
    fn modules_ops_payload_includes_core_ops_for_core_query() {
        let payload = modules_ops_payload("core");
        let matches = payload
            .get("matches")
            .and_then(|v| v.as_array())
            .expect("matches array");

        assert!(matches.iter().any(|row| {
            row.get("module_id").and_then(|v| v.as_str()) == Some("core")
        }));
    }

    #[test]
    fn modules_info_payload_groups_ops_and_compacts_null_schema_refs() {
        let payload = modules_info_payload("web").expect("web module payload");

        assert_eq!(payload.get("module_id").and_then(|v| v.as_str()), Some("web"));
        assert_eq!(payload.get("op_summary").and_then(|v| v.get("total_ops")).and_then(|v| v.as_u64()), Some(5));

        let groups = payload
            .get("exported_ops_by_effect")
            .and_then(|v| v.as_array())
            .expect("effect groups");
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].get("effect").and_then(|v| v.as_str()), Some("control"));
        assert_eq!(groups[1].get("effect").and_then(|v| v.as_str()), Some("network"));

        let exported_ops = payload
            .get("exported_ops")
            .and_then(|v| v.as_array())
            .expect("exported ops");
        let first = exported_ops.first().expect("first exported op");
        assert_eq!(first.get("op").and_then(|v| v.as_str()), Some("duckduckgo"));
        assert_eq!(first.get("output_type").and_then(|v| v.as_str()), Some("object"));
        assert_eq!(
            first
                .get("output_object_type")
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("results"))
                .and_then(|v| v.get("ty"))
                .and_then(|v| v.as_str()),
            Some("array")
        );
        assert_eq!(
            first
                .get("input_object_type")
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("query"))
                .and_then(|v| v.get("ty"))
                .and_then(|v| v.as_str()),
            Some("string")
        );
        assert!(first.get("input_schema_ref").is_none());
        assert!(first.get("output_schema_ref").is_none());
    }

    #[test]
    fn modules_types_payload_reports_type_summary_and_compacts_null_schema_refs() {
        let payload = modules_types_payload("web").expect("web module payload");

        assert_eq!(payload.get("module_id").and_then(|v| v.as_str()), Some("web"));
        assert_eq!(payload.get("type_summary").and_then(|v| v.get("typed_ops")).and_then(|v| v.as_u64()), Some(0));

        let types = payload
            .get("types")
            .and_then(|v| v.as_array())
            .expect("types array");
        assert_eq!(types.len(), 5);
        assert!(types.iter().all(|row| row.get("output_type").and_then(|v| v.as_str()) == Some("object")));
        let duckduckgo = types
            .iter()
            .find(|row| row.get("op").and_then(|v| v.as_str()) == Some("duckduckgo"))
            .expect("duckduckgo op row");
        assert!(duckduckgo.get("input_object_type").is_some());

        let providers = types
            .iter()
            .find(|row| row.get("op").and_then(|v| v.as_str()) == Some("providers"))
            .expect("providers op row");
        assert!(providers.get("input_object_type").is_none());
        assert_eq!(
            providers
                .get("output_object_type")
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("providers"))
                .and_then(|v| v.get("ty"))
                .and_then(|v| v.as_str()),
            Some("array")
        );
        assert!(types.iter().all(|row| row.get("input_schema_ref").is_none()));
        assert!(types.iter().all(|row| row.get("output_schema_ref").is_none()));
    }

    #[test]
    fn modules_examples_payload_returns_none_for_unknown_module() {
        assert!(modules_examples_payload("unknown").is_none());
    }

    #[test]
    fn discover_examples_filters_query_tag_and_complexity() {
        let matches = discover_examples(
            Some("fallback"),
            Some("routing"),
            Some("advanced"),
            false,
        );

        assert!(matches.iter().any(|row| row.name == "web-provider-routing"));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn example_by_name_returns_expected_run_hint() {
        let row = example_by_name("main").expect("main example exists");
        assert_eq!(row.path, "examples/main.gr");
        assert_eq!(row.run, "grapheme run examples/main.gr");
    }

    #[test]
    fn execute_source_runs_core_echo() {
                let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "hello from sdk") {
    state { current }
  }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let result = engine.execute_source(source).expect("execution should succeed");

        assert_eq!(
            result
                .final_state
                .get("current")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str()),
            Some("hello from sdk")
        );
    }

    #[test]
    fn format_result_supports_yaml_and_json() {
                let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "hello") {
    state { current }
  }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let result = engine.execute_source(source).expect("execution should succeed");

        let yaml = engine
            .format_result(&result, StructuredMode::Yaml)
            .expect("yaml format should succeed");
        let json = engine
            .format_result(&result, StructuredMode::Json)
            .expect("json format should succeed");

        assert!(yaml.contains("artifact_id:"));
        assert!(json.contains("\"artifact_id\""));
    }

    #[test]
    fn execute_artifact_preserves_lint_warnings_from_compilation_path() {
                let source = r#"import core from "grapheme/core"

query Q {
    core.echo(message: "hi") {
    state { current }
  }
}
"#;

        let compiled = Compiler::compile_source(source, CompilerOptions::default())
            .expect("compile should succeed");
        let engine = GraphemeEngine::builder().build();
        let result = engine
            .execute_compiled(&compiled)
            .expect("compiled execution should succeed");

        assert_eq!(result.lint_warnings.len(), compiled.compilation.lint_warnings.len());
    }

    #[test]
    fn capability_observer_receives_host_calls() {
        let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "observe") {
    state { current }
  }
}
"#;

        let observed = Arc::new(Mutex::new(Vec::<String>::new()));
        let observed_ref = Arc::clone(&observed);

        let engine = GraphemeEngine::builder()
            .with_capability_observer(move |call| {
                observed_ref
                    .lock()
                    .expect("lock observer")
                    .push(format!("{}.{}", call.module.clone().unwrap_or_default(), call.op));
            })
            .build();

        let result = engine.execute_source(source).expect("execution should succeed");
        assert_eq!(
            result
                .final_state
                .get("current")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str()),
            Some("observe")
        );

        let calls = observed.lock().expect("lock observer snapshot");
        assert!(calls.iter().any(|v| v.ends_with(".echo")));
    }

    #[test]
    fn capability_interceptor_can_override_stdlib_dispatch() {
        let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "original") {
    state { current }
  }
}
"#;

        let engine = GraphemeEngine::builder()
            .with_capability_interceptor(|call| {
                if call.op == "echo" {
                    return Some(Ok(serde_json::json!({"message": "intercepted"})));
                }
                None
            })
            .build();

        let result = engine.execute_source(source).expect("execution should succeed");
        assert_eq!(
            result
                .final_state
                .get("current")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str()),
            Some("intercepted")
        );
    }

        #[test]
        fn sql_runtime_flow_denied_by_policy_when_not_allowlisted() {
                let source = r#"import sql from "grapheme/sql"

query SqlDenied {
    sql.query(connection: "sqlite::memory:", sql: "select 1 as ok") {
        state { current }
    }
}
"#;

                let engine = GraphemeEngine::builder().build();
                let result = engine.execute_source(source).expect("execution should complete");

                assert!(matches!(result.execution.outcome, ExecutionOutcome::FatalFailure));
                assert!(result
                        .execution
                        .message
                        .as_deref()
                        .unwrap_or_default()
                        .contains("sql module is disabled"));
        }

        #[test]
        fn sql_runtime_flow_succeeds_when_connection_allowlisted() {
                let source = r#"import sql from "grapheme/sql"

query SqlAllowed {
    sql.query(connection: "sqlite::memory:", sql: "select 1 as ok") {
        state { current }
    }
}
"#;

                let engine = GraphemeEngine::builder()
                        .with_policy_guard(PolicyGuard {
                                allowed_sql_connections: vec!["sqlite::memory:".to_string()],
                                ..PolicyGuard::default()
                        })
                        .build();
                let result = engine.execute_source(source).expect("execution should complete");

                assert!(matches!(result.execution.outcome, ExecutionOutcome::Succeeded));
                assert_eq!(
                        result
                                .final_state
                                .get("current")
                                .and_then(|v| v.get("ok"))
                                .and_then(|v| v.as_bool()),
                        Some(true)
                );
        }

        #[test]
        fn surreal_runtime_flow_denied_by_policy_when_not_allowlisted() {
                let source = r#"import surreal from "grapheme/surreal"

query SurrealDenied {
    surreal.query(connection: "local", query: "return true;") {
        state { current }
    }
}
"#;

                let engine = GraphemeEngine::builder().build();
                let result = engine.execute_source(source).expect("execution should complete");

                assert!(matches!(result.execution.outcome, ExecutionOutcome::FatalFailure));
                assert!(result
                        .execution
                        .message
                        .as_deref()
                        .unwrap_or_default()
                        .contains("surreal module is disabled"));
        }

        #[test]
        fn surreal_runtime_flow_reaches_module_when_connection_allowlisted() {
                let source = r#"import surreal from "grapheme/surreal"

query SurrealAllowed {
    surreal.query(connection: "local", query: "return true;") {
        state { current }
    }
}
"#;

                let engine = GraphemeEngine::builder()
                        .with_policy_guard(PolicyGuard {
                                allowed_surreal_connections: vec!["local".to_string()],
                                ..PolicyGuard::default()
                        })
                        .build();
                let result = engine.execute_source(source).expect("execution should complete");

                assert!(matches!(result.execution.outcome, ExecutionOutcome::Succeeded));
                assert_eq!(
                        result
                                .final_state
                                .get("current")
                                .and_then(|v| v.get("error"))
                                .and_then(|v| v.get("code"))
                                .and_then(|v| v.as_str()),
                        Some("surreal_connection_unresolved")
                );
        }

    #[test]
    fn execute_aot_matches_base_artifact_execution_parity() {
        let source = r#"import core from "grapheme/core"

query HelloAotParity {
    core.echo(message: "hello-aot-parity") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let compiled = Compiler::compile_source_to_aot(source, CompilerOptions::default())
            .expect("compile to aot should succeed");

        let interpreted = engine
            .execute_artifact(&compiled.artifact)
            .expect("interpreted execution should succeed");
        let staged = engine
            .execute_aot(&compiled.aot)
            .expect("aot-backed execution should succeed");

        assert!(matches!(
            (&interpreted.execution.outcome, &staged.execution.outcome),
            (ExecutionOutcome::Succeeded, ExecutionOutcome::Succeeded)
                | (ExecutionOutcome::RetryableFailure, ExecutionOutcome::RetryableFailure)
                | (ExecutionOutcome::FatalFailure, ExecutionOutcome::FatalFailure)
        ));
        assert_eq!(interpreted.final_state, staged.final_state);
    }

    #[test]
    fn format_aot_supports_yaml_and_json() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let aot = engine
            .compile_source_to_aot(source)
            .expect("compile source to aot should succeed");

        let yaml = engine
            .format_aot(&aot, StructuredMode::Yaml)
            .expect("yaml aot formatting should succeed");
        let json = engine
            .format_aot(&aot, StructuredMode::Json)
            .expect("json aot formatting should succeed");

        assert!(yaml.contains("stage: stage_a"));
        assert!(json.contains("\"stage\": \"stage_a\""));
    }

    #[test]
    fn compile_source_to_aot_stage_b_emits_container_metadata() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let imports = vec![
            "grapheme.runtime.host.v1::state.read".to_string(),
            "grapheme.runtime.host.v1::state.write".to_string(),
        ];
        let aot = engine
            .compile_source_to_aot_stage_b(source, b"\0asmstageb", &imports)
            .expect("compile source to stage_b aot should succeed");

        assert!(matches!(aot.stage, grapheme_artifact::AotStage::StageB));
        assert_eq!(aot.payload.format, "grapheme.aot.stage_b.v1");
        assert!(aot.payload.workflow_wasm.is_some());
    }

    #[test]
    fn execute_aot_rejects_stage_b_outside_host_boundary() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let mut stage_a = engine
            .compile_source_to_aot(source)
            .expect("compile source to aot should succeed");

        stage_a.stage = grapheme_artifact::AotStage::StageB;
        stage_a.payload.format = "grapheme.aot.stage_b.v1".to_string();
        stage_a.payload.workflow_wasm = Some(grapheme_artifact::AotWorkflowWasmContainer {
            byte_len: 8,
            sha256: "sha256:deadbeef".to_string(),
            entry_export: "_start".to_string(),
            allowed_imports: vec!["wasi_snapshot_preview1::fd_write".to_string()],
            inline_wasm_hex: None,
        });

        let err = engine
            .execute_aot(&stage_a)
            .expect_err("stage_b boundary escape should be rejected");

        assert!(matches!(err, GraphemeSdkError::Contract(_)));
        assert!(err.to_string().contains("outside host interface boundary"));
    }

    #[test]
    fn execute_aot_stage_b_routes_through_runtime_stage_b_path() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder().build();
        let imports = vec![
            "grapheme.runtime.host.v1::state.read".to_string(),
            "grapheme.runtime.host.v1::state.write".to_string(),
        ];
        let stage_b = engine
            .compile_source_to_aot_stage_b(source, b"\0asmstageb", &imports)
            .expect("compile source to stage_b should succeed");

        let result = engine
            .execute_aot(&stage_b)
            .expect("stage_b execution should succeed");

        assert!(result
            .execution
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("stage_b scaffold executed via parity path")
            || result
                .execution
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("stage_b container executed directly via wasix backend"));

        let stage_b_event_found = result
            .final_state
            .get("runtime_events")
            .and_then(|events| events.as_array())
            .map(|events| {
                events.iter().any(|event| {
                    event
                        .get("kind")
                        .and_then(|v| v.as_str())
                        == Some("aot.stage_b.container_routed")
                })
            })
            .unwrap_or(false);

        assert!(stage_b_event_found);
    }

    #[cfg(not(feature = "wasix-runtime"))]
    #[test]
    fn execute_aot_stage_b_strict_mode_rejects_when_container_runtime_unavailable() {
        let source = r#"import core from "grapheme/core"

query HelloAot {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let engine = GraphemeEngine::builder()
            .with_strict_stage_b_container_execution(true)
            .build();
        let imports = vec![
            "grapheme.runtime.host.v1::state.read".to_string(),
            "grapheme.runtime.host.v1::state.write".to_string(),
        ];
        let stage_b = engine
            .compile_source_to_aot_stage_b(source, b"\0asmstageb", &imports)
            .expect("compile source to stage_b should succeed");

        let err = engine
            .execute_aot(&stage_b)
            .expect_err("strict mode should reject fallback when container runtime is unavailable");

        assert!(matches!(err, GraphemeSdkError::Contract(_)));
        assert!(err
            .to_string()
            .contains("strict stage_b container execution required"));
    }
}

