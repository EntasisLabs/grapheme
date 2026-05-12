/// ─────────────────────────────────────────────────────────────
///  Grapheme  —  CLI
///  Usage:
///    grapheme <file.gr>
///    grapheme parse <file.gr>
///    grapheme compile <file.gr> --emit ast|hir|mir|artifact
///    grapheme plugins build [all|core|io ...]
///    grapheme run <file.gr> [--bind module=path.wasm ...] [--json] [--native-modules]
///    grapheme modules [search <query> | info <module> | types <module> | examples <module>]
/// ─────────────────────────────────────────────────────────────

use grapheme_artifact::{ExecutionResult, MirInst};
use grapheme_compiler::{Compiler, CompilerError, CompilerOptions};
use grapheme_compiler::verifier::LintWarning;
use grapheme_runtime::{
    CapabilityCall, CapabilityHost, HostCallError, PolicyGuard, RuntimeEngine,
    TracePolicy, TraceProjection,
};
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::env;
use std::fs;
use std::process::{self, Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunOutputMode {
    Plain,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscoveryOutputMode {
    Yaml,
    Json,
}

struct RunOptions {
    bindings: Vec<(String, PathBuf)>,
    output_mode: RunOutputMode,
    native_modules: bool,
    stream_steps: bool,
    trace_profile: TraceProfile,
    trace_steps: Option<usize>,
    trace_projection: Option<TraceProjection>,
    trace_max_string_bytes: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceProfile {
    Lean,
    Debug,
}

struct PluginBuildSpec {
    name: &'static str,
    manifest_rel: &'static str,
    wasm_binary_name: &'static str,
    output_rel: &'static str,
}

const PLUGIN_BUILD_SPECS: &[PluginBuildSpec] = &[
    PluginBuildSpec {
        name: "core",
        manifest_rel: "plugins/core-echo-rs/Cargo.toml",
        wasm_binary_name: "core-echo-plugin",
        output_rel: "plugins/core-echo-rs.wasm",
    },
    PluginBuildSpec {
        name: "io",
        manifest_rel: "plugins/io-rs/Cargo.toml",
        wasm_binary_name: "io-plugin",
        output_rel: "plugins/io-rs.wasm",
    },
    PluginBuildSpec {
        name: "http",
        manifest_rel: "plugins/http-rs/Cargo.toml",
        wasm_binary_name: "http-plugin",
        output_rel: "plugins/http-rs.wasm",
    },
    PluginBuildSpec {
        name: "memory",
        manifest_rel: "plugins/memory-rs/Cargo.toml",
        wasm_binary_name: "memory-plugin",
        output_rel: "plugins/memory-rs.wasm",
    },
    PluginBuildSpec {
        name: "tcp",
        manifest_rel: "plugins/tcp-rs/Cargo.toml",
        wasm_binary_name: "tcp-plugin",
        output_rel: "plugins/tcp-rs.wasm",
    },
    PluginBuildSpec {
        name: "smtp",
        manifest_rel: "plugins/smtp-rs/Cargo.toml",
        wasm_binary_name: "smtp-plugin",
        output_rel: "plugins/smtp-rs.wasm",
    },
    PluginBuildSpec {
        name: "secrets",
        manifest_rel: "plugins/secrets-rs/Cargo.toml",
        wasm_binary_name: "secrets-plugin",
        output_rel: "plugins/secrets-rs.wasm",
    },
    PluginBuildSpec {
        name: "docs",
        manifest_rel: "plugins/docs-rs/Cargo.toml",
        wasm_binary_name: "docs-plugin",
        output_rel: "plugins/docs-rs.wasm",
    },
];

const HOST_PREFERRED_MODULES: &[&str] = &["http", "tcp", "smtp"];

fn main() {
    let args: Vec<String> = env::args().collect();

    if let Err(e) = run(args) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), CompilerError> {
    if args.len() < 2 {
        print_usage();
        return Err(CompilerError::RuntimeError(
            "missing command or file path".to_string(),
        ));
    }

    if args.len() == 2 && matches!(args[1].as_str(), "help" | "--help" | "-h") {
        print_usage();
        return Ok(());
    }

    // Backwards-compatible mode: `grapheme file.gr` maps to parse.
    if args.len() == 2
        && args[1] != "parse"
        && args[1] != "compile"
        && args[1] != "run"
        && args[1] != "plugins"
        && args[1] != "modules"
    {
        return emit_parse(&args[1], DiscoveryOutputMode::Yaml);
    }

    match args[1].as_str() {
        "parse" => {
            emit_parse_cmd(&args[2..])
        }
        "compile" => emit_compile_cmd(&args[2..]),
        "plugins" => emit_plugins(&args),
        "run" => {
            if args.len() < 3 {
                print_usage();
                return Err(CompilerError::RuntimeError(
                    "run requires a file path".to_string(),
                ));
            }
            let (file_path, run_options) = parse_run_args(&args[2..])?;
            run_program(&file_path, run_options)
        }
        "modules" => emit_modules_cmd(&args[2..]),
        _ => {
            print_usage();
            Err(CompilerError::RuntimeError(format!(
                "unknown command '{}'",
                args[1]
            )))
        }
    }
}

fn emit_plugins(args: &[String]) -> Result<(), CompilerError> {
    if args.len() < 3 {
        print_usage();
        return Err(CompilerError::RuntimeError(
            "plugins requires a subcommand (build)".to_string(),
        ));
    }

    match args[2].as_str() {
        "build" => {
            let targets = if args.len() > 3 {
                args[3..].to_vec()
            } else {
                vec!["all".to_string()]
            };
            build_plugins(&targets)
        }
        other => Err(CompilerError::RuntimeError(format!(
            "unknown plugins subcommand '{}'; expected build",
            other
        ))),
    }
}

fn build_plugins(targets: &[String]) -> Result<(), CompilerError> {
    ensure_wasi_target_installed()?;

    let root = env::current_dir()
        .map_err(|e| CompilerError::RuntimeError(format!("resolve current directory: {e}")))?;

    let selected_specs = resolve_plugin_selection(targets)?;

    for spec in selected_specs {
        let manifest = root.join(spec.manifest_rel);
        let plugin_dir = manifest.parent().ok_or_else(|| {
            CompilerError::RuntimeError(format!(
                "invalid plugin manifest path '{}'",
                spec.manifest_rel
            ))
        })?;
        let wasm_source = plugin_dir
            .join("target")
            .join("wasm32-wasmer-wasi")
            .join("release")
            .join(format!("{}.wasm", spec.wasm_binary_name));
        let output = root.join(spec.output_rel);

        run_cmd(
            "cargo",
            &[
                "wasix",
                "build",
                "--manifest-path",
                &manifest.to_string_lossy(),
                "--release"
            ],
        )?;

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CompilerError::RuntimeError(format!(
                    "create output directory '{}': {e}",
                    parent.display()
                ))
            })?;
        }

        fs::copy(&wasm_source, &output).map_err(|e| {
            CompilerError::RuntimeError(format!(
                "copy '{}' to '{}': {e}",
                wasm_source.display(),
                output.display()
            ))
        })?;

        println!("Built {}", output.display());
    }

    Ok(())
}

fn resolve_plugin_selection(targets: &[String]) -> Result<Vec<&'static PluginBuildSpec>, CompilerError> {
    if targets.iter().any(|t| t == "all") {
        return Ok(PLUGIN_BUILD_SPECS.iter().collect());
    }

    let mut selected = Vec::new();

    for target in targets {
        if let Some(spec) = PLUGIN_BUILD_SPECS.iter().find(|s| s.name == target) {
            selected.push(spec);
        } else {
            let valid = PLUGIN_BUILD_SPECS
                .iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
                .join("|");
            return Err(CompilerError::RuntimeError(format!(
                "unknown plugin target '{}'; expected all|{}",
                target, valid
            )));
        }
    }

    Ok(selected)
}

fn ensure_wasi_target_installed() -> Result<(), CompilerError> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map_err(|e| CompilerError::RuntimeError(format!("run rustup target list --installed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CompilerError::RuntimeError(format!(
            "rustup target list --installed failed: {}",
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.lines().any(|line| line.trim() == "wasm32-wasip1") {
        return Err(CompilerError::RuntimeError(
            "missing Rust target wasm32-wasip1; install with: rustup target add wasm32-wasip1"
                .to_string(),
        ));
    }

    Ok(())
}

fn run_cmd(program: &str, args: &[&str]) -> Result<(), CompilerError> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| CompilerError::RuntimeError(format!("run {program}: {e}")))?;

    if !status.success() {
        return Err(CompilerError::RuntimeError(format!(
            "command failed: {} {}",
            program,
            args.join(" ")
        )));
    }

    Ok(())
}

fn collect_called_modules(artifact: &grapheme_artifact::ArtifactEnvelope) -> Vec<String> {
    let mut modules = BTreeSet::new();

    for function in &artifact.payload.mir.functions {
        for block in &function.blocks {
            for inst in &block.instructions {
                let MirInst::Call {
                    module,
                    capability,
                    ..
                } = inst else {
                    continue;
                };

                let module_id = module
                    .as_deref()
                    .map(|m| m.to_lowercase())
                    .or_else(|| capability.0.split('.').next().map(|m| m.to_lowercase()));

                if let Some(module_id) = module_id {
                    modules.insert(module_id);
                }
            }
        }
    }

    modules.into_iter().collect()
}

fn plugin_spec_by_name(name: &str) -> Option<&'static PluginBuildSpec> {
    PLUGIN_BUILD_SPECS.iter().find(|spec| spec.name == name)
}

fn emit_modules(mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let manifests = grapheme_runtime::core_v1_manifests();
    print_discovery(&manifests, mode)
}

fn emit_modules_cmd(args: &[String]) -> Result<(), CompilerError> {
    let (mode, cmd_args) = parse_discovery_args(args)?;

    if cmd_args.is_empty() {
        return emit_modules(mode);
    }

    match cmd_args[0].as_str() {
        "help" | "--help" | "-h" => {
            print_modules_usage();
            Ok(())
        }
        "search" => {
            if cmd_args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules search requires a query".to_string(),
                ));
            }
            emit_modules_search(&cmd_args[1], mode)
        }
        "info" => {
            if cmd_args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules info requires a module id".to_string(),
                ));
            }
            emit_modules_info(&cmd_args[1], mode)
        }
        "types" => {
            if cmd_args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules types requires a module id".to_string(),
                ));
            }
            emit_modules_types(&cmd_args[1], mode)
        }
        "examples" => {
            if cmd_args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules examples requires a module id".to_string(),
                ));
            }
            emit_modules_examples(&cmd_args[1], mode)
        }
        other => Err(CompilerError::RuntimeError(format!(
            "unknown modules subcommand '{}'; expected search|info|types|examples",
            other
        ))),
    }
}

fn parse_discovery_args(args: &[String]) -> Result<(DiscoveryOutputMode, Vec<String>), CompilerError> {
    let mut mode = DiscoveryOutputMode::Yaml;
    let mut cmd_args = Vec::new();

    for arg in args {
        match arg.as_str() {
            "--yaml" => mode = DiscoveryOutputMode::Yaml,
            "--json" => mode = DiscoveryOutputMode::Json,
            _ => cmd_args.push(arg.clone()),
        }
    }

    Ok((mode, cmd_args))
}

fn parse_structured_output_flag(flag: &str) -> Option<DiscoveryOutputMode> {
    match flag {
        "--yaml" => Some(DiscoveryOutputMode::Yaml),
        "--json" => Some(DiscoveryOutputMode::Json),
        _ => None,
    }
}

fn emit_modules_search(query: &str, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let q = query.to_lowercase();
    let matches = grapheme_runtime::core_v1_manifests()
        .into_iter()
        .filter(|m| {
            m.module_id.to_lowercase().contains(&q)
                || m.exported_ops
                    .iter()
                    .any(|op| op.op.to_lowercase().contains(&q))
        })
        .map(|m| m.module_id)
        .collect::<Vec<_>>();

    print_discovery(&matches, mode)
}

fn find_manifest(module: &str) -> Result<grapheme_runtime::ModuleManifest, CompilerError> {
    grapheme_runtime::core_v1_manifests()
        .into_iter()
        .find(|m| m.module_id.eq_ignore_ascii_case(module))
        .ok_or_else(|| CompilerError::RuntimeError(format!("unknown module '{}'", module)))
}

fn emit_modules_info(module: &str, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let manifest = find_manifest(module)?;
    print_discovery(&manifest, mode)
}

fn emit_modules_types(module: &str, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let manifest = find_manifest(module)?;
    let types = manifest
        .exported_ops
        .iter()
        .map(|op| {
            json!({
                "op": op.op,
                "input_schema_ref": op.input_schema_ref,
                "output_schema_ref": op.output_schema_ref,
                "effect": op.effect,
            })
        })
        .collect::<Vec<_>>();

    print_discovery(&json!({
        "module_id": manifest.module_id,
        "types": types,
    }), mode)
}

fn emit_modules_examples(module: &str, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let module_id = module.to_lowercase();
    let examples: &[&str] = match module_id.as_str() {
        "http" => &["examples/http-get.gr"],
        "websearch" => &[
            "examples/websearch-materials.gr",
            "examples/websearch-report.gr",
        ],
        "tcp" => &["examples/tcp-connect.gr"],
        "smtp" => &["examples/smtp-send.gr"],
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
    };

    if examples.is_empty() {
        return Err(CompilerError::RuntimeError(format!(
            "no curated examples are registered for module '{}'",
            module
        )));
    }

    print_discovery(&json!({
        "module_id": module_id,
        "examples": examples,
    }), mode)
}

#[derive(Serialize)]
struct CliRunOutput {
    artifact_id: String,
    execution: ExecutionResult,
    final_state: serde_json::Value,
    lint_warnings: Vec<LintWarning>,
}

struct CliHost;

impl CliHost {
    fn resolve_module(call: &CapabilityCall) -> String {
        call.module
            .as_deref()
            .map(|m| m.to_lowercase())
            .or_else(|| call.capability.split('.').next().map(|m| m.to_lowercase()))
            .unwrap_or_default()
    }

    fn dispatch(&self, module: &str, op: &str, args: &JsonValue) -> Option<JsonValue> {
        grapheme_stdlib::registry::dispatch(module, op, args)
    }
}

impl CapabilityHost for CliHost {
    fn call(&mut self, call: &CapabilityCall) -> Result<serde_json::Value, HostCallError> {
        let module = Self::resolve_module(call);
        if let Some(out) = self.dispatch(&module, &call.op, &call.args) {
            return Ok(out);
        }

        Ok(json!({
            "module": call.module,
            "op": call.op,
            "capability": call.capability,
            "arg_count": call.arg_count,
            "args": call.args,
            "step_index": call.step_index,
            "status": "ok"
        }))
    }
}

fn run_program(
    file_path: &str,
    run_options: RunOptions,
) -> Result<(), CompilerError> {
    let source = read_source(file_path)?;
    let compiled = Compiler::compile_source(&source, CompilerOptions::default())?;

    let cwd = env::current_dir()
        .map_err(|e| CompilerError::RuntimeError(format!("resolve current directory: {e}")))?;

    let trace_policy = trace_policy_from_run_options(&run_options);

    let mut module_bindings: HashMap<String, PathBuf> = HashMap::new();
    for (module, path) in run_options.bindings {
        module_bindings.insert(module, path);
    }

    if run_options.native_modules {
        let required_modules = collect_called_modules(&compiled.artifact);
        let plugin_targets = required_modules
            .into_iter()
            .filter(|module| !HOST_PREFERRED_MODULES.contains(&module.as_str()))
            .filter(|module| plugin_spec_by_name(module).is_some())
            .collect::<Vec<_>>();

        if !plugin_targets.is_empty() {
            build_plugins(&plugin_targets)?;
        }

        for module in plugin_targets {
            if module_bindings.contains_key(&module) {
                continue;
            }

            if let Some(spec) = plugin_spec_by_name(&module) {
                module_bindings.insert(module, cwd.join(spec.output_rel));
            }
        }
    }

    let mut host = CliHost;
    let mut options = grapheme_runtime::RuntimeOptions::default();
    options.policy_guard = policy_guard_from_env();
    options.trace_policy = trace_policy;
    options.stream_step_output =
        run_options.output_mode == RunOutputMode::Plain && run_options.stream_steps;
    let (is_set, max_steps) = parse_optional_usize_env("GRAPHEME_RUNTIME_MAX_STEPS")
        .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
    if is_set {
        options.max_steps = max_steps;
    }
    let (is_set, max_call_depth) = parse_optional_usize_env("GRAPHEME_RUNTIME_MAX_CALL_DEPTH")
        .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
    if is_set {
        options.max_call_depth = max_call_depth;
    }
    for (module, path) in module_bindings {
        options.module_registry.set_wasm_path(&module, path);
    }
    let runtime = RuntimeEngine::new(options);
    let (state, execution) = runtime
        .execute_artifact(&compiled.artifact, &mut host)
        .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;

    match run_options.output_mode {
        RunOutputMode::Json => {
            let out = CliRunOutput {
                artifact_id: compiled.artifact.artifact_id,
                execution,
                final_state: state.to_json(),
                lint_warnings: compiled.compilation.lint_warnings.clone(),
            };
            print_json(&out)
        }
        RunOutputMode::Plain => {
            if run_options.stream_steps {
                return Ok(());
            }

            let current_lines = collect_printable_lines_from_json(&state.current);
            if !current_lines.is_empty() {
                for line in current_lines {
                    println!("{line}");
                }
                return Ok(());
            }

            let mut printed_any = false;
            for step in state
                .pipeline
                .iter()
                .filter(|step| step.ok)
                .filter(|step| is_echo_step(&step.op))
            {
                if let Some(line) = printable_line_from_json(&step.output) {
                    println!("{line}");
                    printed_any = true;
                }
            }

            if printed_any {
                return Ok(());
            }

            let current = &state.current;
            if let Some(message) = current.get("message").and_then(|v| v.as_str()) {
                println!("{message}");
                Ok(())
            } else if let Some(text) = current.get("text").and_then(|v| v.as_str()) {
                println!("{text}");
                Ok(())
            } else if let Some(stdout) = current.get("stdout").and_then(|v| v.as_str()) {
                println!("{stdout}");
                Ok(())
            } else if current.is_string() {
                println!("{}", current.as_str().unwrap_or_default());
                Ok(())
            } else {
                let out = CliRunOutput {
                    artifact_id: compiled.artifact.artifact_id,
                    execution,
                    final_state: state.to_json(),
                    lint_warnings: compiled.compilation.lint_warnings.clone(),
                };
                print_json(&out)
            }
        }
    }
}

fn is_echo_step(op: &str) -> bool {
    op.eq_ignore_ascii_case("echo") || op.eq_ignore_ascii_case("core.echo")
}

fn printable_line_from_json(value: &JsonValue) -> Option<String> {
    if let Some(message) = value.get("message").and_then(|v| v.as_str()) {
        return Some(message.to_string());
    }
    if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }
    if let Some(stdout) = value.get("stdout").and_then(|v| v.as_str()) {
        return Some(stdout.to_string());
    }
    value.as_str().map(|s| s.to_string())
}

fn collect_printable_lines_from_json(value: &JsonValue) -> Vec<String> {
    let mut out = Vec::new();
    collect_printable_lines_into(value, &mut out);
    out
}

fn collect_printable_lines_into(value: &JsonValue, out: &mut Vec<String>) {
    if let Some(line) = printable_line_from_json(value) {
        out.push(line);
        return;
    }

    match value {
        JsonValue::Array(items) => {
            for item in items {
                collect_printable_lines_into(item, out);
            }
        }
        JsonValue::Object(map) => {
            if let Some(pipeline) = map.get("pipeline") {
                collect_printable_lines_into(pipeline, out);
            }
        }
        _ => {}
    }
}

fn policy_guard_from_env() -> PolicyGuard {
    PolicyGuard {
        allowed_http_domains: parse_csv_env("GRAPHEME_ALLOWED_HTTP_DOMAINS"),
        allowed_tcp_targets: parse_csv_env("GRAPHEME_ALLOWED_TCP_TARGETS"),
        allowed_smtp_domains: parse_csv_env("GRAPHEME_ALLOWED_SMTP_DOMAINS"),
        allowed_secret_names: parse_csv_env("GRAPHEME_ALLOWED_SECRETS"),
    }
}

fn parse_csv_env(var: &str) -> Vec<String> {
    env::var(var)
        .ok()
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_optional_usize_env(var: &str) -> Result<(bool, Option<usize>), String> {
    let Some(raw) = env::var(var).ok() else {
        return Ok((false, None));
    };

    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("none") || trimmed.eq_ignore_ascii_case("unbounded") {
        return Ok((true, None));
    }

    trimmed
        .parse::<usize>()
        .map(|value| (true, Some(value)))
        .map_err(|_| format!("{var} must be an integer or 'none'"))
}

fn parse_run_args(
    args: &[String],
) -> Result<(String, RunOptions), CompilerError> {
    if args.is_empty() {
        return Err(CompilerError::RuntimeError("run requires a file path".to_string()));
    }

    let file_path = args[0].clone();
    let mut bindings = Vec::new();
    let mut output_mode = RunOutputMode::Plain;
    let mut native_modules = false;
    let mut stream_steps = false;
    let mut trace_profile = TraceProfile::Lean;
    let mut trace_steps: Option<usize> = None;
    let mut trace_projection: Option<TraceProjection> = None;
    let mut trace_max_string_bytes: Option<usize> = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--bind" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--bind requires module=path".to_string(),
                    ));
                }

                let value = &args[i + 1];
                let (module, path) = value.split_once('=').ok_or_else(|| {
                    CompilerError::RuntimeError(format!(
                        "invalid --bind format '{}', expected module=path",
                        value
                    ))
                })?;

                bindings.push((module.to_lowercase(), PathBuf::from(path)));
                i += 2;
            }
            "--json" => {
                output_mode = RunOutputMode::Json;
                i += 1;
            }
            "--native-modules" => {
                native_modules = true;
                i += 1;
            }
            "--stream-steps" => {
                stream_steps = true;
                i += 1;
            }
            "--trace-profile" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--trace-profile requires lean|debug".to_string(),
                    ));
                }

                trace_profile = parse_trace_profile(&args[i + 1])?;
                i += 2;
            }
            "--trace-steps" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--trace-steps requires an integer >= 0".to_string(),
                    ));
                }

                trace_steps = Some(parse_usize_flag("--trace-steps", &args[i + 1])?);
                i += 2;
            }
            "--trace-projection" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--trace-projection requires minimal|full".to_string(),
                    ));
                }

                trace_projection = Some(parse_trace_projection(&args[i + 1])?);
                i += 2;
            }
            "--trace-max-string-bytes" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--trace-max-string-bytes requires an integer >= 0".to_string(),
                    ));
                }

                trace_max_string_bytes = Some(parse_usize_flag(
                    "--trace-max-string-bytes",
                    &args[i + 1],
                )?);
                i += 2;
            }
            other => {
                return Err(CompilerError::RuntimeError(format!(
                    "unknown run flag '{}'",
                    other
                )));
            }
        }
    }

    Ok((
        file_path,
        RunOptions {
            bindings,
            output_mode,
            native_modules,
            stream_steps,
            trace_profile,
            trace_steps,
            trace_projection,
            trace_max_string_bytes,
        },
    ))
}

fn trace_policy_from_run_options(run_options: &RunOptions) -> TracePolicy {
    let mut policy = match run_options.trace_profile {
        TraceProfile::Lean => TracePolicy::lean_default(),
        TraceProfile::Debug => TracePolicy::debug_default(),
    };

    if let Some(steps) = run_options.trace_steps {
        policy.max_pipeline_steps = steps;
    }
    if let Some(projection) = run_options.trace_projection.clone() {
        policy.projection = projection;
    }
    if let Some(max_string_bytes) = run_options.trace_max_string_bytes {
        policy.max_string_bytes = max_string_bytes;
    }

    policy
}

fn parse_trace_profile(value: &str) -> Result<TraceProfile, CompilerError> {
    match value {
        "lean" => Ok(TraceProfile::Lean),
        "debug" => Ok(TraceProfile::Debug),
        _ => Err(CompilerError::RuntimeError(format!(
            "invalid --trace-profile '{}', expected lean|debug",
            value
        ))),
    }
}

fn parse_trace_projection(value: &str) -> Result<TraceProjection, CompilerError> {
    match value {
        "minimal" => Ok(TraceProjection::Minimal),
        "full" => Ok(TraceProjection::Full),
        _ => Err(CompilerError::RuntimeError(format!(
            "invalid --trace-projection '{}', expected minimal|full",
            value
        ))),
    }
}

fn parse_usize_flag(flag: &str, value: &str) -> Result<usize, CompilerError> {
    value.parse::<usize>().map_err(|_| {
        CompilerError::RuntimeError(format!("invalid {} value '{}', expected integer >= 0", flag, value))
    })
}

fn emit_parse_cmd(args: &[String]) -> Result<(), CompilerError> {
    if args.is_empty() {
        print_usage();
        return Err(CompilerError::RuntimeError(
            "parse requires a file path".to_string(),
        ));
    }

    let file_path = args[0].as_str();
    let mut output_mode = DiscoveryOutputMode::Yaml;

    for flag in &args[1..] {
        if let Some(mode) = parse_structured_output_flag(flag) {
            output_mode = mode;
            continue;
        }

        return Err(CompilerError::RuntimeError(format!(
            "unknown parse flag '{}'",
            flag
        )));
    }

    emit_parse(file_path, output_mode)
}

fn emit_parse(file_path: &str, output_mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let source = read_source(file_path)?;
    let program = grapheme_compiler::parse(&source)?;
    print_discovery(&program, output_mode)?;
    Ok(())
}

fn emit_compile_cmd(args: &[String]) -> Result<(), CompilerError> {
    if args.is_empty() {
        print_usage();
        return Err(CompilerError::RuntimeError(
            "compile requires a file path".to_string(),
        ));
    }

    let file_path = args[0].as_str();
    let mut emit_target = "mir".to_string();
    let mut output_mode = DiscoveryOutputMode::Yaml;
    let mut i = 1;
    while i < args.len() {
        if let Some(mode) = parse_structured_output_flag(&args[i]) {
            output_mode = mode;
            i += 1;
            continue;
        }

        match args[i].as_str() {
            "--emit" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--emit requires a value".to_string(),
                    ));
                }
                emit_target = args[i + 1].clone();
                i += 2;
            }
            flag => {
                return Err(CompilerError::RuntimeError(format!(
                    "unknown compile flag '{}'",
                    flag
                )));
            }
        }
    }

    let source = read_source(file_path)?;
    let compilation = grapheme_compiler::compile(&source)?;

    match emit_target.as_str() {
        "ast" => print_discovery(&compilation.ast, output_mode)?,
        "hir" => print_discovery(&compilation.hir, output_mode)?,
        "mir" => print_discovery(&compilation.mir, output_mode)?,
        "artifact" => {
            let artifact = grapheme_artifact::build_artifact_from_mir(&compilation.mir, None)
                .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?;
            print_discovery(&artifact, output_mode)?;
        }
        other => {
            return Err(CompilerError::RuntimeError(format!(
                "unsupported emit target '{}'; expected ast|hir|mir|artifact",
                other
            )));
        }
    }

    Ok(())
}

fn read_source(path: &str) -> Result<String, CompilerError> {
    fs::read_to_string(path)
    .map_err(|e| CompilerError::RuntimeError(format!("error reading {path}: {e}")))
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), CompilerError> {
    let json = serde_json::to_string_pretty(value)
    .map_err(|e| CompilerError::RuntimeError(format!("serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}

fn print_discovery<T: serde::Serialize>(value: &T, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    match mode {
        DiscoveryOutputMode::Json => print_json(value),
        DiscoveryOutputMode::Yaml => {
            let yaml = serde_yaml::to_string(value)
                .map_err(|e| CompilerError::RuntimeError(format!("serialize output: {e}")))?;
            print!("{yaml}");
            Ok(())
        }
    }
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  grapheme <file.gr>");
    eprintln!("  grapheme parse <file.gr> [--yaml|--json]");
    eprintln!("  grapheme compile <file.gr> [--emit ast|hir|mir|artifact] [--yaml|--json]");
    eprintln!("  grapheme plugins build [all|core|io ...]");
    eprintln!("  grapheme run <file.gr> [--bind module=path.wasm ...] [--json] [--native-modules] [--stream-steps]");
    eprintln!("               [--trace-profile lean|debug] [--trace-steps N]");
    eprintln!("               [--trace-projection minimal|full] [--trace-max-string-bytes N]");
    eprintln!("  grapheme modules [--yaml|--json]");
    eprintln!("  grapheme modules search <query> [--yaml|--json]");
    eprintln!("  grapheme modules info <module> [--yaml|--json]");
    eprintln!("  grapheme modules types <module> [--yaml|--json]");
    eprintln!("  grapheme modules examples <module> [--yaml|--json]");
    eprintln!("  grapheme help");
}

fn print_modules_usage() {
    eprintln!("usage:");
    eprintln!("  grapheme modules [--yaml|--json]");
    eprintln!("  grapheme modules search <query> [--yaml|--json]");
    eprintln!("  grapheme modules info <module> [--yaml|--json]");
    eprintln!("  grapheme modules types <module> [--yaml|--json]");
    eprintln!("  grapheme modules examples <module> [--yaml|--json]");
    eprintln!("\nnotes:");
    eprintln!("  --yaml is the default for modules discovery output");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dispatch_std(module: &str, op: &str, args: &JsonValue) -> JsonValue {
        grapheme_stdlib::registry::dispatch(module, op, args)
            .expect("expected stdlib registry op to be registered")
    }

    #[test]
    fn core_reduce_avg_computes_expected_value() {
        let out = dispatch_std("core", "reduce", &json!({
            "items": [3, 8, 2, 5],
            "mode": "avg"
        }));
        assert_eq!(out, json!(4.5));
    }

    #[test]
    fn core_reduce_concat_respects_initial_prefix() {
        let out = dispatch_std("core", "reduce", &json!({
            "items": ["-a", "-b"],
            "mode": "concat",
            "initial": "seed"
        }));
        assert_eq!(out, json!("seed-a-b"));
    }

    #[test]
    fn core_reduce_unknown_mode_returns_error_payload() {
        let out = dispatch_std("core", "reduce", &json!({
            "items": [1, 2],
            "mode": "mystery"
        }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn core_set_and_get_path_round_trip_nested_value() {
        let set_out = dispatch_std("core", "set_path", &json!({
            "input": {"rollout": {"stage": "canary"}},
            "path": "rollout.owner",
            "value": "platform"
        }));

        let get_out = dispatch_std("core", "get_path", &json!({
            "input": set_out,
            "path": "rollout.owner"
        }));

        assert_eq!(get_out, json!("platform"));
    }

    #[test]
    fn core_has_path_detects_presence_and_absence() {
        let present = dispatch_std("core", "has_path", &json!({
            "input": {"a": {"b": 1}},
            "path": "a.b"
        }));
        let missing = dispatch_std("core", "has_path", &json!({
            "input": {"a": {"b": 1}},
            "path": "a.c"
        }));

        assert_eq!(present.get("has_path"), Some(&json!(true)));
        assert_eq!(missing.get("has_path"), Some(&json!(false)));
    }

    #[test]
    fn core_apply_lane_merges_fields_into_target_lane() {
        let out = dispatch_std("core", "apply_lane", &json!({
            "lane": "state",
            "fields": { "status": "collecting" },
            "__input": { "state": { "attempt": 1 }, "data": { "text": "x" } }
        }));

        assert_eq!(
            out.get("state")
                .and_then(|v| v.get("attempt"))
                .and_then(|v| v.as_i64()),
            Some(1)
        );
        assert_eq!(
            out.get("state")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str()),
            Some("collecting")
        );
    }

    #[test]
    fn core_math_and_comparison_ops_are_live() {
        let add = dispatch_std("core", "add", &json!({ "a": 7, "b": 5 }));
        let sub = dispatch_std("core", "sub", &json!({ "a": 7, "b": 5 }));
        let gt = dispatch_std("core", "gt", &json!({ "a": 7, "b": 5 }));
        let eq = dispatch_std("core", "eq", &json!({ "a": "x", "b": "x" }));

        assert_eq!(add, json!(12.0));
        assert_eq!(sub, json!(2.0));
        assert_eq!(gt.get("value").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(eq.get("value").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn core_field_mutation_ops_are_live() {
        let inc = dispatch_std(
            "core",
            "inc_field",
            &json!({ "field": "count", "input": { "count": 3 } }),
        );
        let dec = dispatch_std(
            "core",
            "dec_field",
            &json!({ "field": "count", "input": { "count": 3 } }),
        );
        let set = dispatch_std(
            "core",
            "set_fields",
            &json!({ "fields": { "status": "ok" }, "input": { "count": 1 } }),
        );

        assert_eq!(inc.get("count").and_then(|v| v.as_f64()), Some(4.0));
        assert_eq!(dec.get("count").and_then(|v| v.as_f64()), Some(2.0));
        assert_eq!(set.get("status").and_then(|v| v.as_str()), Some("ok"));
    }

}
