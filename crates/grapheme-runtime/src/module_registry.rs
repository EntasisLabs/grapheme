use std::collections::HashMap;
use std::path::PathBuf;

use crate::module_manifest::{core_v1_manifests, ModuleAbi, ModuleManifest};

#[derive(Debug, Clone)]
pub struct ModuleBinding {
    pub manifest: ModuleManifest,
    pub wasm_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    bindings: HashMap<String, ModuleBinding>,
}

#[derive(Debug, Clone)]
pub struct ResolvedModuleCall {
    pub module_id: String,
    pub op: String,
    pub abi: ModuleAbi,
    pub wasm_path: Option<PathBuf>,
}

impl ModuleRegistry {
    pub fn from_core_v1() -> Self {
        let mut bindings = HashMap::new();
        for manifest in core_v1_manifests() {
            let module_id = manifest.module_id.clone();
            bindings.insert(
                module_id,
                ModuleBinding {
                    manifest,
                    wasm_path: None,
                },
            );
        }

        Self { bindings }
    }

    pub fn set_wasm_path(&mut self, module_id: &str, wasm_path: PathBuf) {
        if let Some(binding) = self.bindings.get_mut(module_id) {
            binding.wasm_path = Some(wasm_path);
        }
    }

    pub fn resolve_call(&self, module: Option<&str>, op: &str, capability: &str) -> Option<ResolvedModuleCall> {
        let module_id = module
            .map(|m| m.to_lowercase())
            .or_else(|| capability.split('.').next().map(|m| m.to_lowercase()))?;

        let binding = self.bindings.get(&module_id)?;

        let op_exists = binding.manifest.exported_ops.iter().any(|e| e.op == op);
        if !op_exists {
            return None;
        }

        Some(ResolvedModuleCall {
            module_id,
            op: op.to_string(),
            abi: if binding.wasm_path.is_some() {
                ModuleAbi::WasixV1
            } else {
                binding.manifest.abi.clone()
            },
            wasm_path: binding.wasm_path.clone(),
        })
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::from_core_v1()
    }
}
