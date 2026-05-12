use serde::{Deserialize, Serialize};

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
        exported_ops: vec![op("to_md", EffectKind::Pure), op("clean_text", EffectKind::Pure)],
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
        exported_ops: vec![op("parse", EffectKind::Pure)],
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
        exported_ops: vec![op("to_list", EffectKind::Pure)],
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
        exported_ops: vec![op("to_json", EffectKind::Pure)],
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
        exported_ops: vec![
            op("native_module_guide", EffectKind::Control),
            op("native_module_registry", EffectKind::Control),
            op("native_module_example", EffectKind::Control),
        ],
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
        exported_ops: vec![
            op("echo", EffectKind::Pure),
            op("tap", EffectKind::Pure),
            op("pack_state_data", EffectKind::Pure),
            op("get_state", EffectKind::Pure),
            op("get_data", EffectKind::Pure),
            op("map", EffectKind::Pure),
            op("filter", EffectKind::Pure),
            op("find", EffectKind::Pure),
            op("reduce", EffectKind::Pure),
            op("group_by", EffectKind::Pure),
            op("merge", EffectKind::Pure),
            op("pick", EffectKind::Pure),
            op("validate_schema", EffectKind::Pure),
            op("add", EffectKind::Pure),
            op("sub", EffectKind::Pure),
            op("inc", EffectKind::Pure),
            op("dec", EffectKind::Pure),
            op("eq", EffectKind::Pure),
            op("lt", EffectKind::Pure),
            op("gt", EffectKind::Pure),
            op("gte", EffectKind::Pure),
            op("lte", EffectKind::Pure),
            op("inc_field", EffectKind::Pure),
            op("dec_field", EffectKind::Pure),
            op("set_fields", EffectKind::Pure),
            op("split", EffectKind::Pure),
            op("join", EffectKind::Pure),
            op("replace", EffectKind::Pure),
            op("trim", EffectKind::Pure),
            op("lower", EffectKind::Pure),
            op("upper", EffectKind::Pure),
            op("contains", EffectKind::Pure),
            op("get_path", EffectKind::Pure),
            op("set_path", EffectKind::Pure),
            op("has_path", EffectKind::Pure),
        ],
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
        exported_ops: vec![
            op("read_text", EffectKind::Io),
            op("write_text", EffectKind::Io),
            op("list_dir", EffectKind::Io),
        ],
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
        exported_ops: vec![op("get", EffectKind::Network), op("post", EffectKind::Network)],
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
        exported_ops: vec![
            op("search", EffectKind::Network),
            op("research_materials", EffectKind::Network),
            op("research_report", EffectKind::Network),
        ],
        required_capabilities: vec!["websearch.execute".to_string()],
        limits: limits_standard(),
    }
}

fn module_tcp() -> ModuleManifest {
    ModuleManifest {
        module_id: "tcp".to_string(),
        version: "1.0.0".to_string(),
        abi: ModuleAbi::MirV1,
        entrypoint: "tcp.main".to_string(),
        exported_ops: vec![
            op("connect", EffectKind::Network),
            op("send", EffectKind::Network),
            op("receive", EffectKind::Network),
        ],
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
        exported_ops: vec![op("send_mail", EffectKind::Network)],
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
        exported_ops: vec![
            op("load_context", EffectKind::State),
            op("store_context", EffectKind::State),
            op("summarize_context", EffectKind::State),
        ],
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
        exported_ops: vec![
            op("get_secret_handle", EffectKind::Secrets),
            op("sign_request", EffectKind::Secrets),
        ],
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
