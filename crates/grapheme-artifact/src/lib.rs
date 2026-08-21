//! Artifact and AOT contract types for Grapheme.
//!
//! This crate defines the serialized envelopes shared between compiler and runtime:
//! MIR artifacts, Stage A/Stage B AOT envelopes, and execution result summaries.

pub mod artifact;
pub mod capability;
pub mod mir;

pub use artifact::{
    build_aot_from_artifact, build_artifact_from_mir, build_stage_b_container_from_aot,
    validate_aot_host_interface_boundary, AotCompatibilityMetadata, AotEnvelope, AotPayload,
    AotStage, AotWorkflowWasmContainer, ArtifactEnvelope, ArtifactError, ArtifactPayload,
    ExecutionOutcome, ExecutionResult, TraceSummary,
};
pub use capability::{Capability, CapabilityPolicy};
pub use mir::{
    MirBlock, MirCompareOp, MirFunction, MirFunctionKind, MirInst, MirIntentConfig, MirLoopConfig,
    MirLoopMergeMode, MirLoopUntil, MirMatchCase, MirMatchTarget, MirParam, MirProgram,
    MirRetryConfig, MirTerminator, MirTimeoutConfig,
};
