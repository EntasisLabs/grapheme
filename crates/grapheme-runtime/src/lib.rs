//! Grapheme runtime engine and policy-governed capability execution.
//!
//! This crate executes verified artifact/AOT envelopes against a capability host,
//! enforces runtime policy, and tracks module lifecycle events.

pub mod error;
pub mod host;
pub mod module_manager;
pub mod module_manifest;
pub mod module_registry;
pub mod policy;
pub mod runtime;
pub mod state;
#[cfg(feature = "wasix-runtime")]
pub mod wasix_backend;

pub use error::RuntimeError;
pub use host::{CapabilityCall, CapabilityHost, HostCallError};
pub use module_manager::{
    ActivationResult, CompatibilityMode, LoadModuleRequest, ModuleGeneration, ModuleLifecycleEvent,
    ModuleLifecycleEventKind, ModuleLifecycleState, ModuleLoadError, ModuleManager,
};
pub use module_manifest::{
    core_v1_manifests, EffectKind, ExportedOp, ModuleAbi, ModuleManifest, ResourceLimits,
};
pub use module_registry::{ModuleBinding, ModuleRegistry, ResolvedModuleCall};
pub use policy::PolicyGuard;
pub use runtime::{RuntimeEngine, RuntimeOptions};
pub use state::{AgentState, TracePolicy, TraceProjection};
#[cfg(feature = "wasix-runtime")]
pub use wasix_backend::WasixBackend;
