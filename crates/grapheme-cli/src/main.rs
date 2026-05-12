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
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{self, Command};
use std::time::Duration;
use websearch::{providers::DuckDuckGoProvider, web_search, SearchOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunOutputMode {
    Plain,
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

    // Backwards-compatible mode: `grapheme file.gr` maps to parse.
    if args.len() == 2
        && args[1] != "parse"
        && args[1] != "compile"
        && args[1] != "run"
        && args[1] != "plugins"
        && args[1] != "modules"
    {
        return emit_parse(&args[1]);
    }

    match args[1].as_str() {
        "parse" => {
            if args.len() != 3 {
                print_usage();
                return Err(CompilerError::RuntimeError(
                    "parse requires exactly one file path".to_string(),
                ));
            }
            emit_parse(&args[2])
        }
        "compile" => emit_compile(&args),
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

fn emit_modules() -> Result<(), CompilerError> {
    let manifests = grapheme_runtime::core_v1_manifests();
    print_json(&manifests)
}

fn emit_modules_cmd(args: &[String]) -> Result<(), CompilerError> {
    if args.is_empty() {
        return emit_modules();
    }

    match args[0].as_str() {
        "search" => {
            if args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules search requires a query".to_string(),
                ));
            }
            emit_modules_search(&args[1])
        }
        "info" => {
            if args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules info requires a module id".to_string(),
                ));
            }
            emit_modules_info(&args[1])
        }
        "types" => {
            if args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules types requires a module id".to_string(),
                ));
            }
            emit_modules_types(&args[1])
        }
        "examples" => {
            if args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules examples requires a module id".to_string(),
                ));
            }
            emit_modules_examples(&args[1])
        }
        other => Err(CompilerError::RuntimeError(format!(
            "unknown modules subcommand '{}'; expected search|info|types|examples",
            other
        ))),
    }
}

fn emit_modules_search(query: &str) -> Result<(), CompilerError> {
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

    print_json(&matches)
}

fn find_manifest(module: &str) -> Result<grapheme_runtime::ModuleManifest, CompilerError> {
    grapheme_runtime::core_v1_manifests()
        .into_iter()
        .find(|m| m.module_id.eq_ignore_ascii_case(module))
        .ok_or_else(|| CompilerError::RuntimeError(format!("unknown module '{}'", module)))
}

fn emit_modules_info(module: &str) -> Result<(), CompilerError> {
    let manifest = find_manifest(module)?;
    print_json(&manifest)
}

fn emit_modules_types(module: &str) -> Result<(), CompilerError> {
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

    print_json(&json!({
        "module_id": manifest.module_id,
        "types": types,
    }))
}

fn emit_modules_examples(module: &str) -> Result<(), CompilerError> {
    let module_id = module.to_lowercase();
    let examples: &[&str] = match module_id.as_str() {
        "http" => &["examples/http-get.gr"],
        "websearch" => &[
            "examples/websearch-basic.gr",
            "examples/websearch-materials.gr",
            "examples/websearch-report.gr",
        ],
        "tcp" => &["examples/tcp-connect.gr"],
        "smtp" => &["examples/smtp-send.gr"],
        "io" => &["examples/io-list.gr"],
        "memory" => &["examples/memory-roundtrip.gr"],
        "secrets" => &["examples/secrets-handle.gr", "examples/secrets-sign.gr"],
        "json" | "csv" | "yaml" | "html" => &[
            "examples/request-transform-output.gr",
            "examples/transform-cookbook/yaml-json-parse-field.gr",
            "examples/transform-cookbook/csv-to-json-envelope.gr",
            "examples/transform-cookbook/http-html-markdown.gr",
        ],
        "core" => &[
            "examples/core-merge.gr",
            "examples/core-filter.gr",
            "examples/transform-cookbook/core-string-ops.gr",
            "examples/transform-cookbook/core-list-ops.gr",
            "examples/transform-cookbook/core-reduce-modes.gr",
            "examples/transform-cookbook/core-path-ops.gr",
        ],
        _ => &[],
    };

    if examples.is_empty() {
        return Err(CompilerError::RuntimeError(format!(
            "no curated examples are registered for module '{}'",
            module
        )));
    }

    print_json(&json!({
        "module_id": module_id,
        "examples": examples,
    }))
}

#[derive(Serialize)]
struct CliRunOutput {
    artifact_id: String,
    execution: ExecutionResult,
    final_state: serde_json::Value,
}

struct CliHost;

impl CapabilityHost for CliHost {
    fn call(&mut self, call: &CapabilityCall) -> Result<serde_json::Value, HostCallError> {
        let module = call
            .module
            .as_deref()
            .map(|m| m.to_lowercase())
            .or_else(|| call.capability.split('.').next().map(|m| m.to_lowercase()))
            .unwrap_or_default();

        match (module.as_str(), call.op.as_str()) {
            ("core", "echo") => {
                let message = call
                    .args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(json!({ "message": message }))
            }
            ("core", "map") => Ok(host_core_map(&call.args)),
            ("core", "filter") => Ok(host_core_filter(&call.args)),
            ("core", "find") => Ok(host_core_find(&call.args)),
            ("core", "reduce") => Ok(host_core_reduce(&call.args)),
            ("core", "group_by") => Ok(host_core_group_by(&call.args)),
            ("core", "merge") => Ok(host_core_merge(&call.args)),
            ("core", "pick") => Ok(host_core_pick(&call.args)),
            ("core", "validate_schema") => Ok(host_core_validate_schema(&call.args)),
            ("core", "split") => Ok(host_core_split(&call.args)),
            ("core", "join") => Ok(host_core_join(&call.args)),
            ("core", "replace") => Ok(host_core_replace(&call.args)),
            ("core", "trim") => Ok(host_core_trim(&call.args)),
            ("core", "lower") => Ok(host_core_lower(&call.args)),
            ("core", "upper") => Ok(host_core_upper(&call.args)),
            ("core", "contains") => Ok(host_core_contains(&call.args)),
            ("core", "get_path") => Ok(host_core_get_path(&call.args)),
            ("core", "set_path") => Ok(host_core_set_path(&call.args)),
            ("core", "has_path") => Ok(host_core_has_path(&call.args)),
            ("http", "get") => {
                let url = call
                    .args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(host_http_request("GET", url, None))
            }
            ("websearch", "search") => Ok(host_websearch_search(&call.args)),
            ("websearch", "research_materials") => Ok(host_websearch_research_materials(&call.args)),
            ("websearch", "research_report") => Ok(host_websearch_research_report(&call.args)),
            ("http", "post") => {
                let url = call
                    .args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(host_http_request("POST", url, call.args.get("body")))
            }
            ("tcp", "connect") => {
                let target = call
                    .args
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(host_tcp_connect(target))
            }
            ("tcp", "send") => {
                let target = call
                    .args
                    .get("target")
                    .or_else(|| call.args.get("session"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let data = call
                    .args
                    .get("data")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(host_tcp_send(target, data))
            }
            ("tcp", "receive") => {
                let target = call
                    .args
                    .get("target")
                    .or_else(|| call.args.get("session"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let max_bytes = call
                    .args
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(1024);
                Ok(host_tcp_receive(target, max_bytes))
            }
            ("smtp", "send_mail") => Ok(host_smtp_send_mail(&call.args)),
            ("html", "to_md") => Ok(host_html_to_md(&call.args)),
            ("html", "clean_text") => Ok(host_html_clean_text(&call.args)),
            ("json", "parse") => Ok(host_json_parse(&call.args)),
            ("csv", "to_list") => Ok(host_csv_to_list(&call.args)),
            ("yaml", "to_json") => Ok(host_yaml_to_json(&call.args)),
            _ => Ok(json!({
                "module": call.module,
                "op": call.op,
                "capability": call.capability,
                "arg_count": call.arg_count,
                "args": call.args,
                "step_index": call.step_index,
                "status": "ok"
            })),
        }
    }
}

fn host_html_to_md(args: &JsonValue) -> JsonValue {
    let html = arg_text(args, "html");
    let options = match parse_html_to_md_options(args) {
        Ok(options) => options,
        Err(err) => {
            return json!({
                "error": err,
                "text": "",
                "markdown": "",
            });
        }
    };

    match html_to_markdown_rs::convert(&html, options.clone()) {
        Ok(result) => {
            let markdown = result.content.as_deref().unwrap_or_default().to_string();
            let result_json = serde_json::to_value(&result).unwrap_or(JsonValue::Null);
            json!({
                "text": markdown.clone(),
                "markdown": markdown,
                "result": result_json,
                "used_options": options,
            })
        }
        Err(err) => json!({
            "error": format!("html to markdown conversion failed: {err}"),
            "text": "",
            "markdown": "",
        }),
    }
}

fn parse_html_to_md_options(args: &JsonValue) -> Result<Option<html_to_markdown_rs::ConversionOptions>, String> {
    let Some(raw) = args.get("options") else {
        return Ok(None);
    };

    if !raw.is_object() {
        return Err("html.to_md options must be an object".to_string());
    }

    serde_json::from_value::<html_to_markdown_rs::ConversionOptions>(raw.clone())
        .map(Some)
        .map_err(|err| format!("invalid html.to_md options: {err}"))
}

fn host_html_clean_text(args: &JsonValue) -> JsonValue {
    let raw = arg_text(args, "text");
    let cleaned = clean_page_text(&raw, arg_u64(args, "max_chars").map(|v| v as usize));
    json!({
        "text": cleaned,
        "length": cleaned.chars().count(),
    })
}

fn host_core_map(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let Some(field) = args.get("field").and_then(|v| v.as_str()) else {
        return JsonValue::Array(items);
    };

    JsonValue::Array(
        items
            .into_iter()
            .map(|item| item.get(field).cloned().unwrap_or(JsonValue::Null))
            .collect(),
    )
}

fn host_core_filter(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let Some(field) = args.get("field").and_then(|v| v.as_str()) else {
        return JsonValue::Array(items);
    };
    let equals = args.get("equals").cloned().unwrap_or(JsonValue::Null);

    JsonValue::Array(
        items
            .into_iter()
            .filter(|item| item.get(field).map(|v| v == &equals).unwrap_or(false))
            .collect(),
    )
}

fn host_core_find(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let Some(field) = args.get("field").and_then(|v| v.as_str()) else {
        return JsonValue::Null;
    };
    let equals = args.get("equals").cloned().unwrap_or(JsonValue::Null);

    items
        .into_iter()
        .find(|item| item.get(field).map(|v| v == &equals).unwrap_or(false))
        .unwrap_or(JsonValue::Null)
}

fn host_core_reduce(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("last");

    match mode {
        "sum" => {
            let sum = items
                .iter()
                .filter_map(|v| v.as_f64())
                .fold(0.0, |acc, n| acc + n);
            json!(sum)
        }
        "min" => items
            .iter()
            .filter_map(|v| v.as_f64())
            .reduce(f64::min)
            .map(|v| json!(v))
            .unwrap_or(JsonValue::Null),
        "max" => items
            .iter()
            .filter_map(|v| v.as_f64())
            .reduce(f64::max)
            .map(|v| json!(v))
            .unwrap_or(JsonValue::Null),
        "avg" => {
            let nums = items.iter().filter_map(|v| v.as_f64()).collect::<Vec<_>>();
            if nums.is_empty() {
                JsonValue::Null
            } else {
                json!(nums.iter().sum::<f64>() / nums.len() as f64)
            }
        }
        "concat" => {
            let initial = args
                .get("initial")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let joined = items
                .iter()
                .map(core_scalar_to_key)
                .collect::<Vec<_>>()
                .join("");
            json!(format!("{initial}{joined}"))
        }
        "count" => json!(items.len()),
        "first" => items.first().cloned().unwrap_or(JsonValue::Null),
        "last" => items.last().cloned().unwrap_or(JsonValue::Null),
        _ => json!({
            "error": format!("unsupported reduce mode '{mode}'"),
            "supported_modes": ["sum", "min", "max", "avg", "concat", "count", "first", "last"]
        }),
    }
}

fn host_core_group_by(args: &JsonValue) -> JsonValue {
    let items = core_items(args);
    let Some(field) = args.get("field").and_then(|v| v.as_str()) else {
        return JsonValue::Object(serde_json::Map::new());
    };

    let mut grouped = serde_json::Map::new();
    for item in items {
        let key = item
            .get(field)
            .map(core_scalar_to_key)
            .unwrap_or_else(|| "null".to_string());

        let entry = grouped
            .entry(key)
            .or_insert_with(|| JsonValue::Array(Vec::new()));
        if let Some(values) = entry.as_array_mut() {
            values.push(item);
        }
    }

    JsonValue::Object(grouped)
}

fn host_core_merge(args: &JsonValue) -> JsonValue {
    let left = args
        .get("left")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .cloned()
        })
        .unwrap_or_default();

    let right = args
        .get("right")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut merged = left;
    for (k, v) in right {
        merged.insert(k, v);
    }

    JsonValue::Object(merged)
}

fn host_core_pick(args: &JsonValue) -> JsonValue {
    let input = args
        .get("input")
        .and_then(|v| v.as_object())
        .cloned()
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .cloned()
        })
        .unwrap_or_default();

    let fields = args
        .get("fields")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut picked = serde_json::Map::new();
    for field in fields.iter().filter_map(|v| v.as_str()) {
        if let Some(value) = input.get(field) {
            picked.insert(field.to_string(), value.clone());
        }
    }
    JsonValue::Object(picked)
}

fn host_core_validate_schema(args: &JsonValue) -> JsonValue {
    let required = args
        .get("required")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let data = args
        .get("data")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let missing = required
        .iter()
        .filter_map(|v| v.as_str())
        .filter(|name| !data.contains_key(*name))
        .map(|s| JsonValue::String(s.to_string()))
        .collect::<Vec<_>>();

    json!({
        "ok": missing.is_empty(),
        "missing": missing,
    })
}

fn host_core_split(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    let sep = args.get("sep").and_then(|v| v.as_str()).unwrap_or(",");

    JsonValue::Array(
        text.split(sep)
            .map(|s| JsonValue::String(s.to_string()))
            .collect(),
    )
}

fn host_core_join(args: &JsonValue) -> JsonValue {
    let sep = args.get("sep").and_then(|v| v.as_str()).unwrap_or(",");
    let items = core_items(args);
    let joined = items
        .iter()
        .map(core_scalar_to_key)
        .collect::<Vec<_>>()
        .join(sep);
    json!({ "text": joined })
}

fn host_core_replace(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    let from = args.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let to = args.get("to").and_then(|v| v.as_str()).unwrap_or("");
    json!({ "text": text.replace(from, to) })
}

fn host_core_trim(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    json!({ "text": text.trim() })
}

fn host_core_lower(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    json!({ "text": text.to_lowercase() })
}

fn host_core_upper(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    json!({ "text": text.to_uppercase() })
}

fn host_core_contains(args: &JsonValue) -> JsonValue {
    let haystack = args
        .get("haystack")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or(JsonValue::Null);
    let needle = args.get("needle").cloned().unwrap_or(JsonValue::Null);

    let contains = match &haystack {
        JsonValue::String(s) => needle
            .as_str()
            .map(|n| s.contains(n))
            .unwrap_or(false),
        JsonValue::Array(items) => items.iter().any(|item| item == &needle),
        JsonValue::Object(map) => needle
            .as_str()
            .map(|k| map.contains_key(k))
            .unwrap_or(false),
        _ => false,
    };

    json!({ "contains": contains })
}

fn host_core_get_path(args: &JsonValue) -> JsonValue {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let input = args
        .get("input")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or(JsonValue::Null);

    json_get_path_value(&input, path).unwrap_or(JsonValue::Null)
}

fn host_core_set_path(args: &JsonValue) -> JsonValue {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let value = args.get("value").cloned().unwrap_or(JsonValue::Null);
    let input = args
        .get("input")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or_else(|| JsonValue::Object(serde_json::Map::new()));

    json_set_path_value(&input, path, value)
}

fn host_core_has_path(args: &JsonValue) -> JsonValue {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let input = args
        .get("input")
        .cloned()
        .or_else(|| args.get("__input").cloned())
        .unwrap_or(JsonValue::Null);

    json!({ "has_path": json_get_path_value(&input, path).is_some() })
}

fn core_items(args: &JsonValue) -> Vec<JsonValue> {
    args.get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_array())
                .cloned()
        })
        .unwrap_or_default()
}

fn core_scalar_to_key(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "<value>".to_string()),
    }
}

fn json_get_path_value(input: &JsonValue, path: &str) -> Option<JsonValue> {
    let segments = path
        .split('.')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return Some(input.clone());
    }

    let mut current = input;
    for segment in segments {
        current = match current {
            JsonValue::Object(map) => map.get(segment)?,
            JsonValue::Array(items) => {
                let idx = segment.parse::<usize>().ok()?;
                items.get(idx)?
            }
            _ => return None,
        };
    }

    Some(current.clone())
}

fn json_set_path_value(input: &JsonValue, path: &str, value: JsonValue) -> JsonValue {
    let segments = path
        .split('.')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return input.clone();
    }

    let mut output = if input.is_object() {
        input.clone()
    } else {
        JsonValue::Object(serde_json::Map::new())
    };

    set_path_recursive(&mut output, &segments, value);
    output
}

fn set_path_recursive(node: &mut JsonValue, segments: &[&str], value: JsonValue) {
    if segments.is_empty() {
        return;
    }

    if segments.len() == 1 {
        if !node.is_object() {
            *node = JsonValue::Object(serde_json::Map::new());
        }
        if let Some(map) = node.as_object_mut() {
            map.insert(segments[0].to_string(), value);
        }
        return;
    }

    if !node.is_object() {
        *node = JsonValue::Object(serde_json::Map::new());
    }

    if let Some(map) = node.as_object_mut() {
        let child = map
            .entry(segments[0].to_string())
            .or_insert_with(|| JsonValue::Object(serde_json::Map::new()));
        set_path_recursive(child, &segments[1..], value);
    }
}

fn host_json_parse(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    match serde_json::from_str::<JsonValue>(&text) {
        Ok(value) => value,
        Err(err) => json!({ "error": format!("json parse failed: {err}") }),
    }
}

fn host_csv_to_list(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(text.as_bytes());

    let headers = match reader.headers() {
        Ok(h) => h.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        Err(err) => return json!({ "error": format!("csv header parse failed: {err}") }),
    };

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = match record {
            Ok(r) => r,
            Err(err) => return json!({ "error": format!("csv row parse failed: {err}") }),
        };

        let mut obj = serde_json::Map::new();
        for (idx, value) in record.iter().enumerate() {
            let key = headers
                .get(idx)
                .cloned()
                .unwrap_or_else(|| format!("col_{idx}"));
            obj.insert(key, JsonValue::String(value.to_string()));
        }
        rows.push(JsonValue::Object(obj));
    }

    JsonValue::Array(rows)
}

fn host_yaml_to_json(args: &JsonValue) -> JsonValue {
    let text = arg_text(args, "text");
    match serde_yaml::from_str::<serde_yaml::Value>(&text) {
        Ok(value) => match serde_json::to_value(value) {
            Ok(json) => json,
            Err(err) => json!({ "error": format!("yaml conversion failed: {err}") }),
        },
        Err(err) => json!({ "error": format!("yaml parse failed: {err}") }),
    }
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    let raw = args
        .get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| args.get("__input").and_then(|v| extract_text_from_input(v, key)))
        .unwrap_or_default();

    decode_escaped_text(&raw)
}

fn arg_u64(args: &JsonValue, key: &str) -> Option<u64> {
    args.get(key)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
}

fn arg_bool(args: &JsonValue, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| {
        v.as_bool().or_else(|| {
            v.as_str().and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            })
        })
    })
}

fn extract_text_from_input(input: &JsonValue, preferred_key: &str) -> Option<String> {
    if let Some(raw) = input.as_str() {
        return Some(raw.to_string());
    }

    let map = input.as_object()?;
    let mut probe_order = vec![preferred_key];
    probe_order.extend(["text", "body", "content", "html", "markdown", "data"]);

    for key in probe_order {
        if let Some(raw) = map.get(key).and_then(|v| v.as_str()) {
            return Some(raw.to_string());
        }
    }

    if map.len() == 1 {
        return map.values().next().and_then(|v| v.as_str()).map(ToOwned::to_owned);
    }

    None
}

fn decode_escaped_text(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

fn host_http_request(method: &str, url: &str, body: Option<&JsonValue>) -> JsonValue {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return json!({
            "error": "host http adapter supports only http:// and https:// URLs",
            "method": method,
            "url": url,
        });
    }

    let payload = body
        .map(|b| serde_json::to_vec(b).unwrap_or_else(|_| b"null".to_vec()))
        .unwrap_or_default();

    let mut request = if method == "POST" {
        let mut req = ehttp::Request::post(url, payload);
        req.headers.insert("Content-Type", "application/json");
        req
    } else {
        ehttp::Request::get(url)
    };
    request.timeout = Some(Duration::from_secs(15));

    let response = match ehttp::fetch_blocking(&request) {
        Ok(resp) => resp,
        Err(err) => {
            return json!({ "error": format!("request failed: {err}"), "method": method, "url": url });
        }
    };

    let response_body = String::from_utf8_lossy(&response.bytes).to_string();
    let status_line = format!("HTTP {} {}", response.status, response.status_text);

    json!({
        "method": method,
        "url": url,
        "status": response.status,
        "status_line": status_line,
        "body": response_body,
    })
}

fn host_websearch_search(args: &JsonValue) -> JsonValue {
    let query = arg_text(args, "query");
    if query.trim().is_empty() {
        return json!({ "error": "missing required arg: query" });
    }

    let provider = args
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("duckduckgo")
        .to_lowercase();

    if provider != "duckduckgo" {
        return json!({
            "error": format!("unsupported websearch provider '{}'; currently supported: duckduckgo", provider)
        });
    }

    let max_results = args
        .get("max_results")
        .and_then(|v| v.as_u64())
        .map(|v| v.min(20) as u32);

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            return json!({ "error": format!("websearch runtime init failed: {err}") });
        }
    };

    let search_result = runtime.block_on(async {
        let provider = DuckDuckGoProvider::new();
        web_search(SearchOptions {
            query: query.clone(),
            max_results,
            provider: Box::new(provider),
            ..Default::default()
        })
        .await
    });

    match search_result {
        Ok(results) => json!({
            "query": query,
            "provider": provider,
            "count": results.len(),
            "results": results,
        }),
        Err(err) => json!({
            "query": query,
            "provider": provider,
            "error": err.to_string(),
            "results": [],
        }),
    }
}

fn host_websearch_research_report(args: &JsonValue) -> JsonValue {
    let materials = host_websearch_research_materials(args);
    if materials.get("error").is_some() {
        return materials;
    }

    let query = materials
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let provider = materials
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("duckduckgo")
        .to_string();

    let report_chars = arg_u64(args, "report_chars")
        .map(|n| n as usize)
        .unwrap_or(2500);

    let sources = materials
        .get("sources")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut findings = Vec::new();
    let mut executive_points = Vec::new();

    for source in &sources {
        let title = source
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(untitled)")
            .to_string();
        let snippet = source
            .get("snippet")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let highlights = source
            .get("highlights")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                if snippet.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![normalize_report_line(&snippet)]
                }
            });

        let key_point = highlights
            .first()
            .cloned()
            .unwrap_or_else(|| snippet.clone());

        if !key_point.is_empty() {
            findings.push(format!("- {}: {}", title, key_point));
            executive_points.push(key_point.clone());
        }
    }

    let mut report_lines = vec![
        format!("Research report: {}", query),
        format!("Provider: {}", provider),
        format!("Sources analyzed: {}", sources.len()),
        "".to_string(),
        "Executive summary".to_string(),
    ];

    if executive_points.is_empty() {
        report_lines.push("- No strong findings extracted from fetched pages.".to_string());
    } else {
        for point in executive_points.into_iter().take(5) {
            report_lines.push(format!("- {}", point));
        }
    }

    report_lines.extend([
        "".to_string(),
        "Findings".to_string(),
    ]);

    if findings.is_empty() {
        report_lines.push("- No strong findings extracted from fetched pages.".to_string());
    } else {
        report_lines.extend(findings.into_iter());
    }

    report_lines.push("".to_string());
    report_lines.push("Source details".to_string());
    for source in &sources {
        let title = source
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(untitled)");
        let url = source.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let status = source.get("status").and_then(|v| v.as_u64());

        report_lines.push(format!("- {}", title));
        if !url.is_empty() {
            report_lines.push(format!("  url: {}", url));
        }
        if let Some(code) = status {
            report_lines.push(format!("  status: {}", code));
        }

        if let Some(highlights) = source.get("highlights").and_then(|v| v.as_array()) {
            for h in highlights.iter().take(3).filter_map(|v| v.as_str()) {
                report_lines.push(format!("  - {}", h));
            }
        }
    }

    report_lines.push("".to_string());
    report_lines.push("Sources".to_string());
    for source in &sources {
        let title = source
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(untitled)");
        let url = source.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if !url.is_empty() {
            report_lines.push(format!("- {} ({})", title, url));
        }
    }

    let report_text = report_lines.join("\n");
    let summary = if report_text.chars().count() > report_chars {
        report_text.chars().take(report_chars).collect::<String>()
    } else {
        report_text.clone()
    };

    json!({
        "query": query,
        "provider": provider,
        "count": sources.len(),
        "sources": sources,
        "report": summary,
        "materials": materials,
    })
}

fn host_websearch_research_materials(args: &JsonValue) -> JsonValue {
    let search = host_websearch_search(args);
    if search.get("error").is_some() {
        return search;
    }

    let query = search
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let provider = search
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("duckduckgo")
        .to_string();
    let per_source_chars = arg_u64(args, "per_source_chars")
        .map(|n| n as usize)
        .unwrap_or(5000);
    let include_http_body = arg_bool(args, "include_http_body").unwrap_or(false);
    let md_options = args.get("md_options").cloned();

    let results = search
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut sources = Vec::new();

    for (idx, source) in results.into_iter().enumerate() {
        let title = source
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(untitled)")
            .to_string();
        let url = source
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let snippet = source
            .get("snippet")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if url.is_empty() {
            let snippet_clean = normalize_report_line(&snippet);
            let highlights = if snippet_clean.is_empty() {
                Vec::<String>::new()
            } else {
                vec![snippet_clean]
            };

            sources.push(json!({
                "source_id": format!("s{}", idx + 1),
                "title": title,
                "url": url,
                "snippet": snippet,
                "status": null,
                "ok": false,
                "fetch_error": "missing url",
                "conversion_error": null,
                "content_origin": "snippet_fallback",
                "content": highlights.first().cloned().unwrap_or_default(),
                "http": null,
                "markdown": "",
                "clean_text": "",
                "highlights": highlights,
                "citation": format!("[s{}] {}", idx + 1, title),
            }));
            continue;
        }

        let http = host_http_request("GET", &url, None);
        let status = http.get("status").and_then(|v| v.as_u64());
        let body = http
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fetch_error = http
            .get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut md_args = json!({ "html": body });
        if let Some(options) = md_options.clone() {
            md_args["options"] = options;
        }

        let md_result = host_html_to_md(&md_args);
        let markdown = md_result
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let cleaned = clean_page_text(&markdown, Some(per_source_chars));
        let conversion_error = md_result
            .get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let has_fetched_content = !cleaned.trim().is_empty() || !markdown.trim().is_empty();
        let can_use_fetched_content = fetch_error.is_none() && conversion_error.is_none() && has_fetched_content;

        let snippet_clean = normalize_report_line(&snippet);
        let content_origin = if can_use_fetched_content {
            "fetched_page"
        } else {
            "snippet_fallback"
        };

        let content = if can_use_fetched_content {
            if !cleaned.trim().is_empty() {
                cleaned.clone()
            } else {
                markdown.clone()
            }
        } else {
            snippet_clean.clone()
        };

        let highlights = if can_use_fetched_content {
            extract_source_highlights(&content, &snippet, 5, false)
        } else if snippet_clean.is_empty() {
            Vec::new()
        } else {
            vec![snippet_clean]
        };

        let mut source_obj = serde_json::Map::new();
        source_obj.insert("source_id".to_string(), json!(format!("s{}", idx + 1)));
        source_obj.insert("title".to_string(), json!(title));
        source_obj.insert("url".to_string(), json!(url));
        source_obj.insert("snippet".to_string(), json!(snippet));
        source_obj.insert("status".to_string(), json!(status));
        source_obj.insert("ok".to_string(), json!(fetch_error.is_none()));
        source_obj.insert("fetch_error".to_string(), json!(fetch_error));
        source_obj.insert("conversion_error".to_string(), json!(conversion_error));
        source_obj.insert("content_origin".to_string(), json!(content_origin));
        source_obj.insert("content".to_string(), json!(content));
        source_obj.insert("markdown".to_string(), json!(markdown));
        source_obj.insert("clean_text".to_string(), json!(cleaned));
        source_obj.insert("highlights".to_string(), json!(highlights));
        source_obj.insert(
            "citation".to_string(),
            json!(format!("[s{}] {}", idx + 1, source_obj.get("title").and_then(|v| v.as_str()).unwrap_or("(untitled)"))),
        );

        if include_http_body {
            source_obj.insert("http".to_string(), http);
        } else {
            source_obj.insert(
                "http".to_string(),
                json!({
                    "status": status,
                    "url": source_obj.get("url").cloned().unwrap_or(JsonValue::Null),
                    "status_line": source_obj
                        .get("status")
                        .and_then(|s| s.as_u64())
                        .map(|code| format!("HTTP {}", code))
                        .unwrap_or_else(|| "HTTP".to_string()),
                }),
            );
        }

        if let Some(result_payload) = md_result.get("result").cloned() {
            source_obj.insert("md_result".to_string(), result_payload);
        }

        sources.push(JsonValue::Object(source_obj));
    }

    json!({
        "query": query,
        "provider": provider,
        "count": sources.len(),
        "sources": sources,
    })
}

fn extract_source_highlights(text: &str, snippet: &str, max_points: usize, allow_snippet_fallback: bool) -> Vec<String> {
    let mut points = Vec::new();

    for line in text.lines().map(str::trim) {
        if line.len() < 35 || line.len() > 220 {
            continue;
        }
        if is_weak_fact_line(line) {
            continue;
        }

        let normalized = normalize_report_line(line);
        if normalized.is_empty() {
            continue;
        }

        if !points.iter().any(|existing: &String| existing.eq_ignore_ascii_case(&normalized)) {
            points.push(normalized);
        }

        if points.len() >= max_points {
            break;
        }
    }

    if allow_snippet_fallback && points.is_empty() && !snippet.trim().is_empty() {
        let normalized_snippet = normalize_report_line(snippet.trim());
        if !normalized_snippet.is_empty() {
            points.push(normalized_snippet);
        }
    }

    points
}

fn normalize_report_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_tag = false;

    for ch in line.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    out = out
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_weak_fact_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.starts_with("canonical:")
    || lower.starts_with("meta-")
    || lower.starts_with("og:")
    || lower.starts_with("twitter:")
        || lower.starts_with("keywords:")
        || lower.starts_with("description:")
        || lower.starts_with("author:")
        || lower.starts_with("published")
        || lower.starts_with("updated")
        || lower.starts_with("share this")
    || lower.contains("application/ld+json")
    || lower.contains("schema.org")
        || lower.contains("cookie")
        || lower.contains("privacy")
        || lower.contains("terms")
    || lower.contains("robots")
    || lower.contains("viewport")
    || lower.contains("favicon")
    || lower.contains(": http://")
    || lower.contains(": https://")
}

fn clean_page_text(raw: &str, max_chars: Option<usize>) -> String {
    let mut lines = Vec::new();
    let mut in_code = false;

    for original in raw.lines() {
        let line = original.trim();

        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || line.is_empty() {
            continue;
        }

        let mut candidate = line.to_string();
        for marker in [
            " const ",
            " window.",
            " document.",
            " function ",
            " @media ",
            " input[type=",
            " { font-family",
            "::-webkit",
            " appearance:",
            " let ",
            " var ",
        ] {
            if let Some(idx) = candidate.find(marker) {
                if idx > 20 {
                    candidate.truncate(idx);
                } else {
                    candidate.clear();
                }
                break;
            }
        }

        let candidate = candidate.trim();
        if candidate.is_empty() {
            continue;
        }

        let lower = candidate.to_lowercase();
        let noisy = lower.starts_with("skip to")
            || lower.contains("cookie")
            || lower.contains("privacy policy")
            || lower.contains("terms of service")
            || lower.starts_with("open menu")
            || lower.contains("keyboard shortcuts")
            || lower.starts_with("sign in")
            || lower.starts_with("sign up")
            || lower.starts_with("footer")
            || lower.starts_with("copyright")
            || candidate.starts_with("window.")
            || candidate.starts_with("document.")
            || candidate.starts_with("function ")
            || candidate.starts_with("const ")
            || candidate.starts_with("let ")
            || candidate.starts_with("var ")
            || candidate.starts_with("@media");

        if noisy {
            continue;
        }

        lines.push(candidate.to_string());
    }

    let cleaned = lines.join("\n");
    match max_chars {
        Some(limit) if cleaned.chars().count() > limit => cleaned.chars().take(limit).collect(),
        _ => cleaned,
    }
}

fn host_tcp_connect(target: &str) -> JsonValue {
    if target.is_empty() {
        return json!({ "connected": false, "error": "missing target" });
    }

    match TcpStream::connect(target) {
        Ok(_) => json!({ "connected": true, "target": target, "session": target }),
        Err(err) => json!({ "connected": false, "target": target, "error": err.to_string() }),
    }
}

fn host_tcp_send(target: &str, data: &str) -> JsonValue {
    if target.is_empty() {
        return json!({ "sent": false, "error": "missing target/session" });
    }

    match TcpStream::connect(target) {
        Ok(mut stream) => {
            let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));
            match stream.write_all(data.as_bytes()) {
                Ok(_) => json!({ "sent": true, "target": target, "bytes": data.len() }),
                Err(err) => json!({ "sent": false, "target": target, "error": err.to_string() }),
            }
        }
        Err(err) => json!({ "sent": false, "target": target, "error": err.to_string() }),
    }
}

fn host_tcp_receive(target: &str, max_bytes: usize) -> JsonValue {
    if target.is_empty() {
        return json!({ "error": "missing target/session" });
    }

    match TcpStream::connect(target) {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
            let mut buf = vec![0u8; max_bytes];
            match stream.read(&mut buf) {
                Ok(n) => json!({
                    "target": target,
                    "bytes": n,
                    "data": String::from_utf8_lossy(&buf[..n]).to_string(),
                }),
                Err(err) => json!({ "target": target, "error": err.to_string() }),
            }
        }
        Err(err) => json!({ "target": target, "error": err.to_string() }),
    }
}

fn host_smtp_send_mail(args: &JsonValue) -> JsonValue {
    let server = args
        .get("server")
        .and_then(|v| v.as_str())
        .unwrap_or("127.0.0.1:25");
    let from = args
        .get("from")
        .and_then(|v| v.as_str())
        .unwrap_or("grapheme@localhost");
    let to = args.get("to").and_then(|v| v.as_str()).unwrap_or("");
    let subject = args
        .get("subject")
        .and_then(|v| v.as_str())
        .unwrap_or("(no subject)");
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("(empty body)");

    if to.is_empty() {
        return json!({ "accepted": false, "error": "missing smtp recipient: to" });
    }

    let mut stream = match TcpStream::connect(server) {
        Ok(s) => s,
        Err(err) => return json!({ "accepted": false, "server": server, "error": err.to_string() }),
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));

    let mut reader = match stream.try_clone() {
        Ok(s) => BufReader::new(s),
        Err(err) => return json!({ "accepted": false, "server": server, "error": err.to_string() }),
    };

    let mut run = || -> Result<(), String> {
        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 220 {
            return Err(format!("expected 220 banner, got {code}: {msg}"));
        }

        send_smtp_line(&mut stream, "HELO grapheme.local")?;
        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 250 {
            return Err(format!("HELO rejected: {code}: {msg}"));
        }

        send_smtp_line(&mut stream, &format!("MAIL FROM:<{from}>") )?;
        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 250 {
            return Err(format!("MAIL FROM rejected: {code}: {msg}"));
        }

        send_smtp_line(&mut stream, &format!("RCPT TO:<{to}>") )?;
        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 250 && code != 251 {
            return Err(format!("RCPT TO rejected: {code}: {msg}"));
        }

        send_smtp_line(&mut stream, "DATA")?;
        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 354 {
            return Err(format!("DATA rejected: {code}: {msg}"));
        }

        send_smtp_line(&mut stream, &format!("Subject: {subject}"))?;
        send_smtp_line(&mut stream, "")?;
        send_smtp_line(&mut stream, body)?;
        send_smtp_line(&mut stream, ".")?;

        let (code, msg) = read_smtp_response(&mut reader)?;
        if code != 250 {
            return Err(format!("message rejected: {code}: {msg}"));
        }

        let _ = send_smtp_line(&mut stream, "QUIT");
        Ok(())
    };

    match run() {
        Ok(()) => json!({
            "accepted": true,
            "server": server,
            "from": from,
            "to": to,
            "subject": subject,
        }),
        Err(err) => json!({ "accepted": false, "server": server, "error": err }),
    }
}

fn send_smtp_line(stream: &mut TcpStream, line: &str) -> Result<(), String> {
    stream
        .write_all(line.as_bytes())
        .map_err(|err| format!("smtp write failed: {err}"))?;
    stream
        .write_all(b"\r\n")
        .map_err(|err| format!("smtp write failed: {err}"))
}

fn read_smtp_response(reader: &mut BufReader<TcpStream>) -> Result<(u16, String), String> {
    let mut lines = Vec::new();

    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|err| format!("read smtp response failed: {err}"))?;
        if read == 0 {
            return Err("smtp server closed connection".to_string());
        }

        let trimmed = line.trim_end().to_string();
        let code = trimmed
            .get(0..3)
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or_else(|| format!("invalid smtp response line: {trimmed}"))?;
        let continuation = trimmed.as_bytes().get(3).copied() == Some(b'-');

        lines.push(trimmed);

        if !continuation {
            return Ok((code, lines.join("\n")));
        }
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

fn emit_parse(file_path: &str) -> Result<(), CompilerError> {
    let source = read_source(file_path)?;
    let program = grapheme_compiler::parse(&source)?;
    print_json(&program)?;
    Ok(())
}

fn emit_compile(args: &[String]) -> Result<(), CompilerError> {
    if args.len() < 3 {
        print_usage();
        return Err(CompilerError::RuntimeError(
            "compile requires a file path".to_string(),
        ));
    }

    let mut emit_target = "mir".to_string();
    let mut i = 3;
    while i < args.len() {
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

    let source = read_source(&args[2])?;
    let compilation = grapheme_compiler::compile(&source)?;

    match emit_target.as_str() {
        "ast" => print_json(&compilation.ast)?,
        "hir" => print_json(&compilation.hir)?,
        "mir" => print_json(&compilation.mir)?,
        "artifact" => {
            let artifact = grapheme_artifact::build_artifact_from_mir(&compilation.mir, None)
                .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?;
            print_json(&artifact)?;
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

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  grapheme <file.gr>");
    eprintln!("  grapheme parse <file.gr>");
    eprintln!("  grapheme compile <file.gr> --emit ast|hir|mir|artifact");
    eprintln!("  grapheme plugins build [all|core|io ...]");
    eprintln!("  grapheme run <file.gr> [--bind module=path.wasm ...] [--json] [--native-modules] [--stream-steps]");
    eprintln!("               [--trace-profile lean|debug] [--trace-steps N]");
    eprintln!("               [--trace-projection minimal|full] [--trace-max-string-bytes N]");
    eprintln!("  grapheme modules");
    eprintln!("  grapheme modules search <query>");
    eprintln!("  grapheme modules info <module>");
    eprintln!("  grapheme modules types <module>");
    eprintln!("  grapheme modules examples <module>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn core_reduce_avg_computes_expected_value() {
        let out = host_core_reduce(&json!({
            "items": [3, 8, 2, 5],
            "mode": "avg"
        }));
        assert_eq!(out, json!(4.5));
    }

    #[test]
    fn core_reduce_concat_respects_initial_prefix() {
        let out = host_core_reduce(&json!({
            "items": ["-a", "-b"],
            "mode": "concat",
            "initial": "seed"
        }));
        assert_eq!(out, json!("seed-a-b"));
    }

    #[test]
    fn core_reduce_unknown_mode_returns_error_payload() {
        let out = host_core_reduce(&json!({
            "items": [1, 2],
            "mode": "mystery"
        }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn core_set_and_get_path_round_trip_nested_value() {
        let set_out = host_core_set_path(&json!({
            "input": {"rollout": {"stage": "canary"}},
            "path": "rollout.owner",
            "value": "platform"
        }));

        let get_out = host_core_get_path(&json!({
            "input": set_out,
            "path": "rollout.owner"
        }));

        assert_eq!(get_out, json!("platform"));
    }

    #[test]
    fn core_has_path_detects_presence_and_absence() {
        let present = host_core_has_path(&json!({
            "input": {"a": {"b": 1}},
            "path": "a.b"
        }));
        let missing = host_core_has_path(&json!({
            "input": {"a": {"b": 1}},
            "path": "a.c"
        }));

        assert_eq!(present.get("has_path"), Some(&json!(true)));
        assert_eq!(missing.get("has_path"), Some(&json!(false)));
    }

    #[test]
    fn websearch_rejects_unsupported_provider() {
        let out = host_websearch_search(&json!({
            "query": "rust",
            "provider": "google"
        }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn html_clean_text_filters_script_noise() {
        let out = host_html_clean_text(&json!({
            "text": "const x = 1;\nKeyboard shortcuts\nReal content line"
        }));
        let text = out.get("text").and_then(|v| v.as_str()).unwrap_or("");
        assert!(text.contains("Real content line"));
        assert!(!text.contains("Keyboard shortcuts"));
        assert!(!text.contains("const x = 1"));
    }

    #[test]
    fn research_report_requires_query() {
        let out = host_websearch_research_report(&json!({}));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn html_to_md_options_enable_document_structure() {
        let out = host_html_to_md(&json!({
            "html": "<h1>Title</h1><p>Hello</p>",
            "options": {
                "include_document_structure": true,
                "extract_metadata": true,
                "output_format": "markdown"
            }
        }));

        assert!(out.get("error").is_none());
        assert!(out.get("result").is_some());
        assert_eq!(
            out.get("used_options")
                .and_then(|v| v.get("include_document_structure"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn html_to_md_rejects_non_object_options() {
        let out = host_html_to_md(&json!({
            "html": "<p>hello</p>",
            "options": "strict"
        }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn extract_highlights_skips_canonical_metadata_lines() {
        let text = "canonical: https://example.com/article\nEggs are versatile and can be boiled, poached, or scrambled in under 10 minutes.\nkeywords: egg, cooking";
        let highlights = extract_source_highlights(text, "fallback snippet", 3, true);
        assert!(!highlights.is_empty());
        assert!(highlights[0].contains("Eggs are versatile"));
        assert!(!highlights[0].to_lowercase().starts_with("canonical:"));
    }

    #[test]
    fn normalize_report_line_removes_html_markup() {
        let line = "Learn <b>how</b> to cook <em>eggs</em> &amp; serve warm.";
        let normalized = normalize_report_line(line);
        assert_eq!(normalized, "Learn how to cook eggs & serve warm.");
    }

    #[test]
    fn extract_highlights_normalizes_snippet_fallback_markup() {
        let highlights = extract_source_highlights("", "Learn <b>how</b> to cook eggs", 3, true);
        assert_eq!(highlights, vec!["Learn how to cook eggs"]);
    }

    #[test]
    fn extract_highlights_no_snippet_fallback_when_disabled() {
        let highlights = extract_source_highlights("", "fallback snippet", 3, false);
        assert!(highlights.is_empty());
    }

    #[test]
    fn research_materials_requires_query() {
        let out = host_websearch_research_materials(&json!({}));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }
}
