use serde::{Deserialize, Serialize};
use grapheme_signatures::{module_ops, SignatureEffect};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub module_id: String,
    pub version: String,
    pub abi: ModuleAbi,
    pub entrypoint: String,
    pub exported_ops: Vec<ExportedOp>,
    pub required_capabilities: Vec<String>,
    pub limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleAbi {
    MirV1,
    WasixV1,
    WasixWitV15,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedOp {
    pub op: String,
    pub input_schema_ref: Option<String>,
    pub output_schema_ref: Option<String>,
    pub effect: EffectKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    Pure,
    Network,
    Io,
    State,
    Secrets,
    Control,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_ms: u64,
    pub max_memory_mb: u64,
    pub max_io_bytes: u64,
    pub max_network_calls: u32,
}

pub fn core_v1_manifests() -> Vec<ModuleManifest> {
    vec![
        module_core(),
        module_html(),
        module_json(),
        module_csv(),
        module_yaml(),
        module_docs(),
        module_io(),
        module_http(),
        module_web(),
        module_websearch(),
        module_tcp(),
        module_smtp(),
        module_memory(),
        module_runtime(),
        module_secrets(),
        module_policy(),
    ]
}

fn module_html() -> ModuleManifest {
    ModuleManifest {
        module_id: "html".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "html.main".to_string(),
        exported_ops: exported_ops_for("html"),
        required_capabilities: vec!["html.transform".to_string()],
        limits: limits_standard(),
    }
}

fn module_json() -> ModuleManifest {
    ModuleManifest {
        module_id: "json".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "json.main".to_string(),
        exported_ops: exported_ops_for("json"),
        required_capabilities: vec!["json.transform".to_string()],
        limits: limits_standard(),
    }
}

fn module_csv() -> ModuleManifest {
    ModuleManifest {
        module_id: "csv".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "csv.main".to_string(),
        exported_ops: exported_ops_for("csv"),
        required_capabilities: vec!["csv.transform".to_string()],
        limits: limits_standard(),
    }
}

fn module_yaml() -> ModuleManifest {
    ModuleManifest {
        module_id: "yaml".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "yaml.main".to_string(),
        exported_ops: exported_ops_for("yaml"),
        required_capabilities: vec!["yaml.transform".to_string()],
        limits: limits_standard(),
    }
}

fn module_docs() -> ModuleManifest {
    ModuleManifest {
        module_id: "docs".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::WasixV1,
        entrypoint: "docs.main".to_string(),
        exported_ops: exported_ops_for("docs"),
        required_capabilities: vec!["docs.read.native_modules".to_string()],
        limits: limits_standard(),
    }
}

fn limits_standard() -> ResourceLimits {
    ResourceLimits {
        max_cpu_ms: 5_000,
        max_memory_mb: 256,
        max_io_bytes: 10 * 1024 * 1024,
        max_network_calls: 50,
    }
}

fn module_core() -> ModuleManifest {
    ModuleManifest {
        module_id: "core".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "core.main".to_string(),
        exported_ops: exported_ops_for("core"),
        required_capabilities: vec!["core.execute".to_string()],
        limits: limits_standard(),
    }
}

fn module_io() -> ModuleManifest {
    ModuleManifest {
        module_id: "io".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::WasixV1,
        entrypoint: "io.main".to_string(),
        exported_ops: exported_ops_for("io"),
        required_capabilities: vec![
            "io.read.workspace".to_string(),
            "io.write.workspace".to_string(),
        ],
        limits: limits_standard(),
    }
}

fn module_http() -> ModuleManifest {
    ModuleManifest {
        module_id: "http".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "http.main".to_string(),
        exported_ops: exported_ops_for("http"),
        required_capabilities: vec![
            "http.get.allowed_domain".to_string(),
            "http.post.allowed_domain".to_string(),
        ],
        limits: limits_standard(),
    }
}

fn module_websearch() -> ModuleManifest {
    ModuleManifest {
        module_id: "websearch".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "websearch.main".to_string(),
        exported_ops: exported_ops_for("websearch"),
        required_capabilities: vec!["websearch.execute".to_string()],
        limits: limits_standard(),
    }
}

fn module_web() -> ModuleManifest {
    ModuleManifest {
        module_id: "web".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "web.main".to_string(),
        exported_ops: exported_ops_for("web"),
        required_capabilities: vec!["web.search.execute".to_string()],
        limits: limits_standard(),
    }
}

fn module_tcp() -> ModuleManifest {
    ModuleManifest {
        module_id: "tcp".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "tcp.main".to_string(),
        exported_ops: exported_ops_for("tcp"),
        required_capabilities: vec!["tcp.connect.allowed_target".to_string()],
        limits: limits_standard(),
    }
}

fn module_smtp() -> ModuleManifest {
    ModuleManifest {
        module_id: "smtp".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "smtp.main".to_string(),
        exported_ops: exported_ops_for("smtp"),
        required_capabilities: vec!["smtp.send.notifications".to_string()],
        limits: limits_standard(),
    }
}

fn module_memory() -> ModuleManifest {
    ModuleManifest {
        module_id: "memory".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "memory.main".to_string(),
        exported_ops: exported_ops_for("memory"),
        required_capabilities: vec!["memory.namespace.access".to_string()],
        limits: limits_standard(),
    }
}

fn module_runtime() -> ModuleManifest {
    ModuleManifest {
        module_id: "runtime".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "runtime.main".to_string(),
        exported_ops: vec![
            op("retry_with_backoff", EffectKind::Control),
            op("checkpoint_state", EffectKind::Control),
            op("emit_event", EffectKind::Control),
        ],
        required_capabilities: vec!["runtime.control".to_string()],
        limits: limits_standard(),
    }
}

fn module_secrets() -> ModuleManifest {
    ModuleManifest {
        module_id: "secrets".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::WasixV1,
        entrypoint: "secrets.main".to_string(),
        exported_ops: exported_ops_for("secrets"),
        required_capabilities: vec!["secrets.use.scoped".to_string()],
        limits: limits_standard(),
    }
}

fn module_policy() -> ModuleManifest {
    ModuleManifest {
        module_id: "policy".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "policy.main".to_string(),
        exported_ops: vec![
            op("check_capability", EffectKind::Control),
            op("check_data_egress", EffectKind::Control),
            op("require_approval", EffectKind::Control),
        ],
        required_capabilities: vec!["policy.enforce".to_string()],
        limits: limits_standard(),
    }
}

fn op(name: &str, effect: EffectKind) -> ExportedOp {
    ExportedOp {
        op: name.to_string(),
        input_schema_ref: None,
        output_schema_ref: None,
        effect,
    }
}

fn exported_ops_for(module_id: &str) -> Vec<ExportedOp> {
    module_ops(module_id)
        .into_iter()
        .map(|spec| ExportedOp {
            op: spec.op.to_string(),
            input_schema_ref: spec.input_schema_ref.map(|s| s.to_string()),
            output_schema_ref: spec.output_schema_ref.map(|s| s.to_string()),
            effect: effect_from_signature(spec.effect),
        })
        .collect()
}

fn effect_from_signature(effect: SignatureEffect) -> EffectKind {
    match effect {
        SignatureEffect::Pure => EffectKind::Pure,
        SignatureEffect::Network => EffectKind::Network,
        SignatureEffect::Io => EffectKind::Io,
        SignatureEffect::State => EffectKind::State,
        SignatureEffect::Secrets => EffectKind::Secrets,
        SignatureEffect::Control => EffectKind::Control,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const STDLIB_SCOPE_MODULES: &[&str] = &[
        "core",
        "http",
        "web",
        "websearch",
        "tcp",
        "smtp",
        "html",
        "json",
        "csv",
        "yaml",
    ];

    #[test]
    fn manifest_ops_are_registered_or_explicitly_unsupported_for_stdlib_scope() {
        let manifests = core_v1_manifests();
        let mut missing = Vec::new();

        for manifest in manifests
            .iter()
            .filter(|m| STDLIB_SCOPE_MODULES.contains(&m.module_id.as_str()))
        {
            for op in &manifest.exported_ops {
                if !grapheme_stdlib::registry::is_registered_op(&manifest.module_id, &op.op)
                    && !grapheme_stdlib::registry::is_explicitly_unsupported_signature_op(
                        &manifest.module_id,
                        &op.op,
                    )
                {
                    missing.push(format!("{}.{}", manifest.module_id, op.op));
                }
            }
        }

        assert!(
            missing.is_empty(),
            "manifest ops missing stdlib coverage: {}",
            missing.join(", ")
        );
    }

    #[test]
    fn stdlib_registered_ops_exist_in_runtime_manifests_for_stdlib_scope() {
        let manifests = core_v1_manifests();
        let mut missing = Vec::new();

        for module in STDLIB_SCOPE_MODULES {
            let Some(manifest) = manifests.iter().find(|m| m.module_id == *module) else {
                missing.push(format!("missing manifest for module '{module}'"));
                continue;
            };

            let manifest_ops = manifest
                .exported_ops
                .iter()
                .map(|op| op.op.as_str())
                .collect::<HashSet<_>>();

            for op in grapheme_stdlib::registry::registered_ops_for_module(module) {
                if !manifest_ops.contains(op) {
                    missing.push(format!("{module}.{op}"));
                }
            }
        }

        assert!(
            missing.is_empty(),
            "stdlib registered ops missing from runtime manifests: {}",
            missing.join(", ")
        );
    }
}
