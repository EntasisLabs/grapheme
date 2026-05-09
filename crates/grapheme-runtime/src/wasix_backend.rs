use std::path::Path;

use serde_json::{json, Value as JsonValue};

use crate::error::RuntimeError;
use crate::module_registry::ResolvedModuleCall;

#[cfg(feature = "wasix-runtime")]
use wasmer::{Engine, Module};
#[cfg(feature = "wasix-runtime")]
use wasmer_wasix::{
    Pipe,
    is_wasi_module,
    runners::wasi::{RuntimeOrEngine, WasiRunner},
    virtual_fs::{AsyncReadExt, AsyncWriteExt},
};
#[cfg(feature = "wasix-runtime")]
use wasmer_types::ModuleHash;

/// Placeholder for the upcoming Wasmer WASIX-backed execution engine.
///
/// This keeps the integration boundary explicit while the MIR interpreter
/// remains the default runtime path.
#[cfg(feature = "wasix-runtime")]
pub struct WasixBackend;

#[cfg(feature = "wasix-runtime")]
impl WasixBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_environment(&self) -> Result<(), RuntimeError> {
        // This is intentionally conservative for now; we only verify that the
        // feature is enabled and dependency wiring compiles.
        Ok(())
    }

    pub fn execute_call(
        &self,
        wasm_path: &Path,
        call: &ResolvedModuleCall,
        args: &JsonValue,
    ) -> Result<JsonValue, RuntimeError> {
        #[cfg(not(target_arch = "wasm32"))]
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| RuntimeError::RuntimeError(format!("init tokio runtime: {e}")))?;
        #[cfg(not(target_arch = "wasm32"))]
        let _guard = runtime.handle().enter();

        let wasm_bytes = std::fs::read(wasm_path).map_err(|e| {
            RuntimeError::RuntimeError(format!(
                "read wasm module '{}': {e}",
                wasm_path.display()
            ))
        })?;

        let engine = Engine::default();
        let module = Module::new(&engine, &wasm_bytes).map_err(|e| {
            RuntimeError::RuntimeError(format!(
                "compile wasm module '{}': {e}",
                wasm_path.display()
            ))
        })?;

        if !is_wasi_module(&module) {
            return Err(RuntimeError::ArtifactCompatibilityError(format!(
                "wasm module '{}' is not WASI/WASIX compatible",
                wasm_path.display()
            )));
        }

        let request = json!({
            "module": call.module_id,
            "op": call.op,
            "args": args,
        });

        let request_bytes = serde_json::to_vec(&request)
            .map_err(|e| RuntimeError::RuntimeError(format!("serialize wasix request: {e}")))?;

        let (mut stdin_tx, stdin_rx) = Pipe::channel();
        runtime
            .block_on(async { stdin_tx.write_all(&request_bytes).await })
            .map_err(|e| RuntimeError::RuntimeError(format!("write wasm stdin: {e}")))?;
        drop(stdin_tx);

        let (stdout_tx, mut stdout_rx) = Pipe::channel();

        {
            let mut runner = WasiRunner::new();
            runner
                .with_stdin(Box::new(stdin_rx))
                .with_stdout(Box::new(stdout_tx));

            runner
                .run_wasm(
                    RuntimeOrEngine::Engine(engine),
                    &call.module_id,
                    module,
                    ModuleHash::random(),
                )
                .map_err(|e| RuntimeError::RuntimeError(format!("run wasm module: {e}")))?;
        }

        let mut stdout = String::new();
        runtime
            .block_on(async { stdout_rx.read_to_string(&mut stdout).await })
            .map_err(|e| RuntimeError::RuntimeError(format!("read wasm stdout: {e}")))?;

        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Ok(JsonValue::Null);
        }

        match serde_json::from_str::<JsonValue>(trimmed) {
            Ok(parsed) => Ok(parsed),
            Err(_) => Ok(json!({ "stdout": trimmed })),
        }
    }
}
