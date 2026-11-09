//! Grapheme runtime engine and policy-governed capability execution.
//!
//! This crate executes verified artifact/AOT envelopes against a capability host,
//! enforces runtime policy, and tracks module lifecycle events.

pub mod error;
pub mod host;
pub mod module_discovery;
pub mod module_hotload;
pub mod module_manager;
pub mod module_manifest;
pub mod module_registry;
pub mod policy;
pub mod runtime;
#[cfg(feature = "stage-b")]
pub mod stage_b;
pub mod state;
pub mod store_session;
#[cfg(feature = "wasix-runtime")]
pub mod wasix_backend;

pub use error::RuntimeError;
pub use host::{CapabilityCall, CapabilityHost, HostCallError};
pub use module_discovery::{
    DiscoveredWasmModule, MANIFEST_SCHEMA, WasmDiscoveryReport, WasmModuleManifest,
    discover_wasm_modules, discovered_module_to_load_request,
};
pub use module_hotload::{
    HOTLOAD_SCHEMA, HotloadError, HotloadStore, apply_hotload_store, default_hotload_store_path,
    export_hotload, hotload_status_payload, import_hotload, load_hotload_store, save_hotload_store,
    sync_registry_from_manager,
};
pub use module_manager::{
    ActivationResult, CompatibilityMode, LoadModuleRequest, ModuleGeneration, ModuleLifecycleEvent,
    ModuleLifecycleEventKind, ModuleLifecycleState, ModuleLoadError, ModuleManager,
};
pub use module_manifest::{
    EffectKind, ExportedOp, ModuleAbi, ModuleManifest, ResourceLimits, core_v1_manifests,
};
pub use module_registry::{ModuleBinding, ModuleRegistry, ResolvedModuleCall};
pub use policy::PolicyGuard;
pub use runtime::{RuntimeEngine, RuntimeOptions};
pub use state::{AgentState, TracePolicy, TraceProjection};
pub use store_session::{StoreCommit, StoreRevision, StoreSession};
#[cfg(feature = "wasix-runtime")]
pub use wasix_backend::WasixBackend;
