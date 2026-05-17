//! Grapheme command-line interface.
//!
//! Provides parse/compile/build/run/modules command surfaces over compiler,
//! artifact, runtime, and SDK crates.

/// ─────────────────────────────────────────────────────────────
///  Grapheme  —  CLI
///  Usage:
///    grapheme <file.gr>
///    grapheme parse <file.gr>
///    grapheme compile <file.gr> --emit ast|hir|mir|artifact|aot
///    grapheme build <file.gr> [--aot-stage stage_a|stage_b] [--out path]
///    grapheme plugins build [all|core|io ...]")
///    grapheme run <file.gr> [--bind module=path.wasm ...] [--json] [--native-modules]
///                          [--aot-stage stage_a|stage_b] [--strict-stage-b]
///    grapheme modules [search <query> | ops <query> | info <module> | types <module> | examples <module>]
/// ─────────────────────────────────────────────────────────────

use grapheme_artifact::{ExecutionResult, MirInst};
use grapheme_compiler::ast::Definition;
use grapheme_compiler::{Compiler, CompilerError, CompilerOptions};
use grapheme_compiler::verifier::{ExecutableKindPolicyMode, LintWarning};
use grapheme_sdk::{
    discover_module_manifests, discover_examples, example_by_name, modules_examples_payload,
    modules_info_payload, modules_ops_payload, modules_search_payload, modules_types_payload,
    GraphemeEngine, ModuleSearchDetail, ModuleSearchOptions, StructuredMode,
};
use grapheme_runtime::{
    PolicyGuard,
    TracePolicy, TraceProjection,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
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
    aot_stage: Option<AotStageSelection>,
    strict_stage_b_container_execution: bool,
    allow_stage_b_fallback: bool,
    stream_steps: bool,
    trace_profile: TraceProfile,
    trace_steps: Option<usize>,
    trace_projection: Option<TraceProjection>,
    trace_max_string_bytes: Option<usize>,
    executable_kind_policy_mode: ExecutableKindPolicyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AotStageSelection {
    StageA,
    StageB,
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

struct BundledExample {
    name: &'static str,
    relative_path: &'static str,
    content: &'static str,
}

#[derive(Default)]
struct ExampleListFilter {
    query: Option<String>,
    tag: Option<String>,
    complexity: Option<String>,
    native_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct GraphemeProjectToml {
    #[serde(rename = "$schema", default)]
    schema: Option<String>,
    project: GraphemeProjectSection,
    #[serde(default)]
    examples: Option<GraphemeExamplesSection>,
}

#[derive(Debug, Clone, Deserialize)]
struct GraphemeProjectSection {
    name: String,
    main: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct GraphemeExamplesSection {
    #[serde(default)]
    namespaces: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
struct AotBuildManifest {
    source: String,
    output: String,
    stage: String,
    aot_id: String,
    runtime_contract: String,
    host_interface_id: String,
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
const STAGE_B_DEFAULT_ALLOWED_IMPORTS: &[&str] = &[
    "grapheme.runtime.host.v1::state.read",
    "grapheme.runtime.host.v1::state.write",
];
const STAGE_B_DEFAULT_WORKFLOW_WASM_BYTES: &[u8] = b"\0asmstageb";

const BUNDLED_EXAMPLES: &[BundledExample] = &[
    BundledExample {
        name: "main",
        relative_path: "examples/main.gr",
        content: include_str!("../bundled-examples/main.gr"),
    },
    BundledExample {
        name: "hello-world",
        relative_path: "examples/hello-world.gr",
        content: include_str!("../bundled-examples/hello-world.gr"),
    },
    BundledExample {
        name: "core-merge",
        relative_path: "examples/core-merge.gr",
        content: include_str!("../bundled-examples/core-merge.gr"),
    },
    BundledExample {
        name: "core-filter",
        relative_path: "examples/core-filter.gr",
        content: include_str!("../bundled-examples/core-filter.gr"),
    },
    BundledExample {
        name: "core-validate-schema",
        relative_path: "examples/core-validate-schema.gr",
        content: include_str!("../bundled-examples/core-validate-schema.gr"),
    },
    BundledExample {
        name: "request-transform-output",
        relative_path: "examples/request-transform-output.gr",
        content: include_str!("../bundled-examples/request-transform-output.gr"),
    },
    BundledExample {
        name: "mutation-update-preferences",
        relative_path: "examples/mutation-update-preferences.gr",
        content: include_str!("../bundled-examples/mutation-update-preferences.gr"),
    },
    BundledExample {
        name: "mutation-state-machine-apply",
        relative_path: "examples/mutation-state-machine-apply.gr",
        content: include_str!("../bundled-examples/mutation-state-machine-apply.gr"),
    },
    BundledExample {
        name: "resilience-composition",
        relative_path: "examples/resilience-composition.gr",
        content: include_str!("../bundled-examples/resilience-composition.gr"),
    },
    BundledExample {
        name: "subscription-heartbeat-readable",
        relative_path: "examples/subscription-heartbeat-readable.gr",
        content: include_str!("../bundled-examples/subscription-heartbeat-readable.gr"),
    },
    BundledExample {
        name: "http-get",
        relative_path: "examples/http-get.gr",
        content: include_str!("../bundled-examples/http-get.gr"),
    },
    BundledExample {
        name: "websearch-basic",
        relative_path: "examples/websearch-basic.gr",
        content: include_str!("../bundled-examples/websearch-basic.gr"),
    },
    BundledExample {
        name: "websearch-report",
        relative_path: "examples/websearch-report.gr",
        content: include_str!("../bundled-examples/websearch-report.gr"),
    },
    BundledExample {
        name: "web-provider-catalog",
        relative_path: "examples/web-provider-catalog.gr",
        content: include_str!("../bundled-examples/web-provider-catalog.gr"),
    },
    BundledExample {
        name: "web-provider-routing",
        relative_path: "examples/web-provider-routing.gr",
        content: include_str!("../bundled-examples/web-provider-routing.gr"),
    },
    BundledExample {
        name: "web-xaviv-planned",
        relative_path: "examples/web-xaviv-planned.gr",
        content: include_str!("../bundled-examples/web-xaviv-planned.gr"),
    },
];

fn main() {
    let args: Vec<String> = env::args().collect();

    if let Err(e) = run(args) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), CompilerError> {
    if args.len() < 2 {
        if let Ok(main_file) = resolve_project_main_path() {
            return run_program(&main_file, default_run_options());
        }

        print_usage();
        return Err(CompilerError::RuntimeError(
            "missing command or file path (or configure project main in grapheme.toml)".to_string(),
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
        && args[1] != "build"
        && args[1] != "run"
        && args[1] != "plugins"
        && args[1] != "examples"
        && args[1] != "modules"
    {
        return emit_parse(&args[1], DiscoveryOutputMode::Yaml);
    }

    match args[1].as_str() {
        "parse" => {
            emit_parse_cmd(&args[2..])
        }
        "compile" => emit_compile_cmd(&args[2..]),
        "build" => emit_build_cmd(&args[2..]),
        "plugins" => emit_plugins(&args),
        "examples" => emit_examples_cmd(&args[2..]),
        "run" => {
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

fn emit_examples_cmd(args: &[String]) -> Result<(), CompilerError> {
    if args.is_empty() || args[0] == "list" {
        let mut mode = None;
        let mut filter = ExampleListFilter::default();

        if !args.is_empty() {
            let mut i = 1;
            while i < args.len() {
                if let Some(flag_mode) = parse_structured_output_flag(&args[i]) {
                    mode = Some(flag_mode);
                    i += 1;
                    continue;
                }

                match args[i].as_str() {
                    "--query" => {
                        if i + 1 >= args.len() {
                            return Err(CompilerError::RuntimeError(
                                "examples list --query requires a value".to_string(),
                            ));
                        }
                        filter.query = Some(args[i + 1].to_lowercase());
                        i += 2;
                    }
                    "--tag" => {
                        if i + 1 >= args.len() {
                            return Err(CompilerError::RuntimeError(
                                "examples list --tag requires a value".to_string(),
                            ));
                        }
                        filter.tag = Some(args[i + 1].to_lowercase());
                        i += 2;
                    }
                    "--complexity" => {
                        if i + 1 >= args.len() {
                            return Err(CompilerError::RuntimeError(
                                "examples list --complexity requires a value".to_string(),
                            ));
                        }
                        filter.complexity = Some(args[i + 1].to_lowercase());
                        i += 2;
                    }
                    "--native-only" => {
                        filter.native_only = true;
                        i += 1;
                    }
                    other => {
                        return Err(CompilerError::RuntimeError(format!(
                            "unknown examples list flag '{}'",
                            other
                        )));
                    }
                }
            }
        }

        let rows = discover_examples(
            filter.query.as_deref(),
            filter.tag.as_deref(),
            filter.complexity.as_deref(),
            filter.native_only,
        );

        if let Some(mode) = mode {
            print_discovery(
                &json!({
                    "filters": {
                        "query": filter.query,
                        "tag": filter.tag,
                        "complexity": filter.complexity,
                        "native_only": filter.native_only,
                    },
                    "count": rows.len(),
                    "examples": rows,
                }),
                mode,
            )?;
        } else {
            println!("NAME\tCOMPLEXITY\tTAGS\tSUMMARY");
            for row in rows {
                let name = row.name;
                let complexity = row.complexity;
                let tags = row.tags.join(",");
                let summary = row.summary;
                println!("{}\t{}\t{}\t{}", name, complexity, tags, summary);
            }

            println!("\nTip: grapheme examples show <name> for quick guidance and source");
            println!("Tip: grapheme examples list --query web --tag routing --complexity advanced");
            println!("Tip: grapheme examples list --yaml for machine-readable details");
        }

        return Ok(());
    }

    match args[0].as_str() {
        "show" => {
            if args.len() < 2 {
                return Err(CompilerError::RuntimeError(
                    "examples show requires an example name".to_string(),
                ));
            }

            let mut mode = DiscoveryOutputMode::Yaml;
            let mut summary_only = false;
            let mut raw = false;
            for flag in &args[2..] {
                if let Some(flag_mode) = parse_structured_output_flag(flag) {
                    mode = flag_mode;
                    continue;
                }

                match flag.as_str() {
                    "--summary" => summary_only = true,
                    "--raw" => raw = true,
                    other => {
                        return Err(CompilerError::RuntimeError(format!(
                            "unknown examples show flag '{}'",
                            other
                        )));
                    }
                }
            }

            if summary_only && raw {
                return Err(CompilerError::RuntimeError(
                    "examples show flags --summary and --raw cannot be combined".to_string(),
                ));
            }

            let ex = find_bundled_example(&args[1])?;
            let row = example_by_name(ex.name).ok_or_else(|| {
                CompilerError::RuntimeError(format!(
                    "missing example metadata for '{}'",
                    ex.name
                ))
            })?;

            if summary_only {
                print_discovery(&row, mode)?;
            } else if raw {
                print!("{}", ex.content);
            } else {
                println!("name: {}", row.name);
                println!("path: {}", row.path);
                println!("summary: {}", row.summary);
                println!("use_when: {}", row.use_when);
                println!("complexity: {}", row.complexity);
                println!("run: {}", row.run);
                println!("tags: {}", row.tags.join(", "));
                println!("requires_native_modules: {}", row.requires_native_modules);

                println!("\n--- source ---");
                print!("{}", ex.content);
                println!("\n--- next ---");
                println!("{}", row.run);
                println!("grapheme examples show {} --summary --yaml", ex.name);
            }
            Ok(())
        }
        "init" => {
            let mut out_dir = PathBuf::from(".");
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--out" => {
                        if i + 1 >= args.len() {
                            return Err(CompilerError::RuntimeError(
                                "examples init --out requires a directory".to_string(),
                            ));
                        }
                        out_dir = PathBuf::from(&args[i + 1]);
                        i += 2;
                    }
                    other => {
                        return Err(CompilerError::RuntimeError(format!(
                            "unknown examples init flag '{}'",
                            other
                        )));
                    }
                }
            }

            for ex in BUNDLED_EXAMPLES {
                let target = out_dir.join(ex.relative_path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        CompilerError::RuntimeError(format!(
                            "create examples directory '{}': {e}",
                            parent.display()
                        ))
                    })?;
                }
                fs::write(&target, ex.content).map_err(|e| {
                    CompilerError::RuntimeError(format!(
                        "write bundled example '{}': {e}",
                        target.display()
                    ))
                })?;
            }

            println!(
                "Initialized {} bundled examples into {}",
                BUNDLED_EXAMPLES.len(),
                out_dir.display()
            );
            println!("Next:");
            println!("  grapheme examples list");
            println!("  grapheme examples show main");
            println!("  grapheme run {}/examples/main.gr", out_dir.display());
            Ok(())
        }
        other => Err(CompilerError::RuntimeError(format!(
            "unknown examples subcommand '{}'; expected list|show|init",
            other
        ))),
    }
}

fn find_bundled_example(name: &str) -> Result<&'static BundledExample, CompilerError> {
    BUNDLED_EXAMPLES.iter().find(|ex| ex.name == name).ok_or_else(|| {
        CompilerError::RuntimeError(format!(
            "unknown bundled example '{}'; run 'grapheme examples list'",
            name
        ))
    })
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
    let manifests = discover_module_manifests();
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
            let (query, options) = parse_modules_search_args(&cmd_args[1..])?;
            emit_modules_search(&query, &options, mode)
        }
        "ops" => {
            if cmd_args.len() != 2 {
                return Err(CompilerError::RuntimeError(
                    "modules ops requires a query".to_string(),
                ));
            }
            emit_modules_ops(&cmd_args[1], mode)
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
            "unknown modules subcommand '{}'; expected search|ops|info|types|examples",
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

fn parse_modules_search_args(args: &[String]) -> Result<(String, ModuleSearchOptions), CompilerError> {
    if args.is_empty() {
        return Err(CompilerError::RuntimeError(
            "modules search requires a query".to_string(),
        ));
    }

    let query = args[0].clone();
    let mut options = ModuleSearchOptions::default();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--explain" => {
                options.explain = true;
                i += 1;
            }
            "--detail" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "modules search --detail requires concise|full".to_string(),
                    ));
                }
                options.detail = parse_modules_search_detail(&args[i + 1])?;
                options.explain = true;
                i += 2;
            }
            "--top" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "modules search --top requires an integer >= 1".to_string(),
                    ));
                }

                let top = args[i + 1].parse::<usize>().map_err(|_| {
                    CompilerError::RuntimeError(
                        "modules search --top requires an integer >= 1".to_string(),
                    )
                })?;
                if top == 0 {
                    return Err(CompilerError::RuntimeError(
                        "modules search --top requires an integer >= 1".to_string(),
                    ));
                }

                options.top = Some(top);
                options.explain = true;
                i += 2;
            }
            "--min-score" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "modules search --min-score requires a number >= 0".to_string(),
                    ));
                }

                let min_score = args[i + 1].parse::<f64>().map_err(|_| {
                    CompilerError::RuntimeError(
                        "modules search --min-score requires a number >= 0".to_string(),
                    )
                })?;
                if min_score < 0.0 {
                    return Err(CompilerError::RuntimeError(
                        "modules search --min-score requires a number >= 0".to_string(),
                    ));
                }

                options.min_score = Some(min_score);
                options.explain = true;
                i += 2;
            }
            other => {
                return Err(CompilerError::RuntimeError(format!(
                    "unknown modules search flag '{}'; expected --explain|--detail|--top|--min-score",
                    other
                )));
            }
        }
    }

    Ok((query, options))
}

fn parse_modules_search_detail(value: &str) -> Result<ModuleSearchDetail, CompilerError> {
    match value {
        "full" => Ok(ModuleSearchDetail::Full),
        "concise" => Ok(ModuleSearchDetail::Concise),
        _ => Err(CompilerError::RuntimeError(format!(
            "invalid modules search --detail '{}', expected concise|full",
            value
        ))),
    }
}

fn emit_modules_search(
    query: &str,
    options: &ModuleSearchOptions,
    mode: DiscoveryOutputMode,
) -> Result<(), CompilerError> {
    let payload = modules_search_payload(query, options);
    print_discovery(&payload, mode)
}

fn emit_modules_ops(query: &str, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let payload = modules_ops_payload(query);
    print_discovery(&payload, mode)
}

fn emit_modules_info(module: &str, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let payload = modules_info_payload(module).ok_or_else(|| {
        CompilerError::RuntimeError(format!("unknown module '{}'", module))
    })?;
    print_discovery(&payload, mode)
}

fn emit_modules_types(module: &str, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let payload = modules_types_payload(module).ok_or_else(|| {
        CompilerError::RuntimeError(format!("unknown module '{}'", module))
    })?;
    print_discovery(&payload, mode)
}

fn emit_modules_examples(module: &str, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let payload = modules_examples_payload(module).ok_or_else(|| {
        CompilerError::RuntimeError(format!(
            "no curated examples are registered for module '{}'",
            module
        ))
    })?;
    print_discovery(&payload, mode)
}

#[derive(Serialize)]
struct CliRunOutput {
    artifact_id: String,
    execution: ExecutionResult,
    final_state: serde_json::Value,
    lint_warnings: Vec<LintWarning>,
}

fn run_program(
    file_path: &str,
    run_options: RunOptions,
) -> Result<(), CompilerError> {
    let source = read_source(file_path)?;
    let mut compiler_options = CompilerOptions::default();
    compiler_options.compile_options.executable_kind_policy_mode =
        run_options.executable_kind_policy_mode;
    let compiled = Compiler::compile_source(&source, compiler_options.clone())?;

    let cwd = env::current_dir()
        .map_err(|e| CompilerError::RuntimeError(format!("resolve current directory: {e}")))?;

    let trace_policy = trace_policy_from_run_options(&run_options);
    let strict_stage_b_container_execution = resolve_stage_b_strict_mode(&run_options);

    let mut module_bindings = run_options.bindings;

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
            if module_bindings.iter().any(|(bound_module, _)| bound_module == &module) {
                continue;
            }

            if let Some(spec) = plugin_spec_by_name(&module) {
                module_bindings.push((module, cwd.join(spec.output_rel)));
            }
        }
    }

    let mut engine_builder = GraphemeEngine::builder()
        .with_policy_guard(policy_guard_from_env())
        .with_trace_policy(trace_policy)
        .with_compiler_options(compiler_options)
        .with_strict_stage_b_container_execution(strict_stage_b_container_execution)
        .with_stream_step_output(
            run_options.output_mode == RunOutputMode::Plain && run_options.stream_steps,
        );

    let (is_set, max_steps) = parse_optional_usize_env("GRAPHEME_RUNTIME_MAX_STEPS")
        .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
    if is_set {
        engine_builder = engine_builder.with_max_steps(max_steps);
    }
    let (is_set, max_call_depth) = parse_optional_usize_env("GRAPHEME_RUNTIME_MAX_CALL_DEPTH")
        .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
    if is_set {
        engine_builder = engine_builder.with_max_call_depth(max_call_depth);
    }

    for (module, path) in module_bindings {
        engine_builder = engine_builder.with_module_path(&module, path);
    }

    let engine = engine_builder.build();
    let mut result = match run_options.aot_stage {
        None => engine
            .execute_compiled(&compiled)
            .map_err(|e| CompilerError::RuntimeError(e.to_string()))?,
        Some(AotStageSelection::StageA) => {
            let aot = engine
                .compile_source_to_aot(&source)
                .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
            engine
                .execute_aot(&aot)
                .map_err(|e| CompilerError::RuntimeError(e.to_string()))?
        }
        Some(AotStageSelection::StageB) => {
            let imports = STAGE_B_DEFAULT_ALLOWED_IMPORTS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            let aot = engine
                .compile_source_to_aot_stage_b(
                    &source,
                    STAGE_B_DEFAULT_WORKFLOW_WASM_BYTES,
                    &imports,
                )
                .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
            engine
                .execute_aot(&aot)
                .map_err(|e| CompilerError::RuntimeError(e.to_string()))?
        }
    };

    if result.lint_warnings.is_empty() {
        result.lint_warnings = compiled.compilation.lint_warnings.clone();
    }

    let final_state = result.final_state;
    let execution = result.execution;
    let lint_warnings = result.lint_warnings;

    match run_options.output_mode {
        RunOutputMode::Json => {
            let out = CliRunOutput {
                artifact_id: result.artifact_id,
                execution,
                final_state: final_state.clone(),
                lint_warnings,
            };
            let rendered = engine
                .format_result(
                    &grapheme_sdk::ExecuteResultPayload {
                        artifact_id: out.artifact_id,
                        execution: out.execution,
                        final_state: out.final_state,
                        lint_warnings: out.lint_warnings,
                    },
                    StructuredMode::Json,
                )
                .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
            println!("{rendered}");
            Ok(())
        }
        RunOutputMode::Plain => {
            if run_options.stream_steps {
                return Ok(());
            }

            let current_lines = collect_printable_lines_from_json(
                final_state.get("current").unwrap_or(&final_state),
            );
            if !current_lines.is_empty() {
                for line in current_lines {
                    println!("{line}");
                }
                return Ok(());
            }

            let mut printed_any = false;
            if let Some(pipeline) = final_state.get("pipeline").and_then(|v| v.as_array()) {
                for step in pipeline {
                    let ok = step.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                    let op = step.get("op").and_then(|v| v.as_str()).unwrap_or_default();
                    if !ok || !is_echo_step(op) {
                        continue;
                    }
                    if let Some(line) = step.get("output").and_then(printable_line_from_json) {
                        println!("{line}");
                        printed_any = true;
                    }
                }
            }

            if printed_any {
                return Ok(());
            }

            let current = final_state.get("current").unwrap_or(&final_state);
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
                    artifact_id: result.artifact_id,
                    execution,
                    final_state,
                    lint_warnings,
                };
                print_json(&out)
            }
        }
    }
}

fn default_run_options() -> RunOptions {
    RunOptions {
        bindings: vec![],
        output_mode: RunOutputMode::Plain,
        native_modules: false,
        aot_stage: None,
        strict_stage_b_container_execution: false,
        allow_stage_b_fallback: false,
        stream_steps: false,
        trace_profile: TraceProfile::Lean,
        trace_steps: None,
        trace_projection: None,
        trace_max_string_bytes: None,
        executable_kind_policy_mode: ExecutableKindPolicyMode::Compatibility,
    }
}

fn resolve_file_path_arg_or_project_main(
    args: &[String],
    command: &str,
) -> Result<(String, usize), CompilerError> {
    if let Some(first) = args.first() {
        if !first.starts_with("--") {
            return Ok((first.clone(), 1));
        }
    }

    let main = resolve_project_main_path().map_err(|_| {
        CompilerError::RuntimeError(format!(
            "{command} requires a file path or a project main configured in grapheme.toml"
        ))
    })?;

    Ok((main, 0))
}

fn resolve_project_main_path() -> Result<String, CompilerError> {
    let cwd = env::current_dir()
        .map_err(|e| CompilerError::RuntimeError(format!("resolve current directory: {e}")))?;

    resolve_project_main_path_from_dir(&cwd)
}

fn resolve_project_main_path_from_dir(start_dir: &Path) -> Result<String, CompilerError> {
    let Some((config_path, config)) = discover_project_config(start_dir)? else {
        return Err(CompilerError::RuntimeError(
            "could not find grapheme.toml in current directory or parent directories".to_string(),
        ));
    };

    if config.project.name.trim().is_empty() {
        return Err(CompilerError::RuntimeError(format!(
            "project name in '{}' must not be empty",
            config_path.display()
        )));
    }

    if let Some(schema) = &config.schema {
        if schema.trim().is_empty() {
            return Err(CompilerError::RuntimeError(format!(
                "'$schema' in '{}' must not be empty",
                config_path.display()
            )));
        }
    }

    if let Some(examples) = &config.examples {
        for (namespace, path) in &examples.namespaces {
            if namespace.trim().is_empty() || path.trim().is_empty() {
                return Err(CompilerError::RuntimeError(format!(
                    "examples.namespaces in '{}' must contain non-empty keys and values",
                    config_path.display()
                )));
            }
        }
    }

    let project_root = config_path
        .parent()
        .ok_or_else(|| CompilerError::RuntimeError("resolve project root".to_string()))?;

    enforce_project_glyph_uniqueness(project_root, &config, &config_path)?;

    let main_path = project_root.join(&config.project.main);
    Ok(main_path.to_string_lossy().to_string())
}

fn enforce_project_glyph_uniqueness(
    project_root: &Path,
    config: &GraphemeProjectToml,
    config_path: &Path,
) -> Result<(), CompilerError> {
    let mut scan_roots = BTreeSet::new();
    scan_roots.insert(project_root.join(&config.project.main));

    if let Some(examples) = &config.examples {
        for path in examples.namespaces.values() {
            scan_roots.insert(project_root.join(path));
        }
    }

    let mut files = BTreeSet::new();
    for root in scan_roots {
        collect_gr_files(&root, &mut files)?;
    }

    let mut glyph_index: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for file in files {
        let source = fs::read_to_string(&file).map_err(|e| {
            CompilerError::RuntimeError(format!("read '{}': {e}", file.display()))
        })?;

        let parsed = grapheme_compiler::parse(&source).map_err(|e| {
            CompilerError::RuntimeError(format!(
                "parse '{}' while validating glyph uniqueness from '{}': {}",
                file.display(),
                config_path.display(),
                e
            ))
        })?;

        for def in parsed.definitions {
            if let Definition::Glyph(glyph) = def {
                glyph_index
                    .entry(glyph.name)
                    .or_default()
                    .push(file.clone());
            }
        }
    }

    let duplicates = glyph_index
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .collect::<Vec<_>>();

    if !duplicates.is_empty() {
        let detail = duplicates
            .into_iter()
            .map(|(glyph, files)| {
                let joined = files
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("glyph '{}' in [{}]", glyph, joined)
            })
            .collect::<Vec<_>>()
            .join("; ");

        return Err(CompilerError::RuntimeError(format!(
            "glyph names must be unique across project main files (from '{}'): {}",
            config_path.display(),
            detail
        )));
    }

    Ok(())
}

fn collect_gr_files(path: &Path, out: &mut BTreeSet<PathBuf>) -> Result<(), CompilerError> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("gr") {
            out.insert(path.to_path_buf());
        }
        return Ok(());
    }

    let entries = fs::read_dir(path).map_err(|e| {
        CompilerError::RuntimeError(format!("read directory '{}': {e}", path.display()))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            CompilerError::RuntimeError(format!("read directory entry in '{}': {e}", path.display()))
        })?;
        collect_gr_files(&entry.path(), out)?;
    }

    Ok(())
}

fn discover_project_config(start_dir: &Path) -> Result<Option<(PathBuf, GraphemeProjectToml)>, CompilerError> {
    let mut dir = start_dir.to_path_buf();

    loop {
        let candidate = dir.join("grapheme.toml");
        if candidate.exists() {
            let raw = fs::read_to_string(&candidate).map_err(|e| {
                CompilerError::RuntimeError(format!("read '{}': {e}", candidate.display()))
            })?;

            let config = toml::from_str::<GraphemeProjectToml>(&raw).map_err(|e| {
                CompilerError::RuntimeError(format!("parse '{}': {e}", candidate.display()))
            })?;

            return Ok(Some((candidate, config)));
        }

        if !dir.pop() {
            break;
        }
    }

    Ok(None)
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
        allowed_sql_connections: parse_csv_env("GRAPHEME_ALLOWED_SQL_CONNECTIONS"),
        allowed_surreal_connections: parse_csv_env("GRAPHEME_ALLOWED_SURREAL_CONNECTIONS"),
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
    let (file_path, consumed) = resolve_file_path_arg_or_project_main(args, "run")?;
    let mut run_options = default_run_options();
    let mut i = consumed;

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

                run_options
                    .bindings
                    .push((module.to_lowercase(), PathBuf::from(path)));
                i += 2;
            }
            "--json" => {
                run_options.output_mode = RunOutputMode::Json;
                i += 1;
            }
            "--native-modules" => {
                run_options.native_modules = true;
                i += 1;
            }
            "--aot-stage" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--aot-stage requires stage_a|stage_b".to_string(),
                    ));
                }

                run_options.aot_stage = Some(parse_aot_stage_selection(&args[i + 1])?);
                i += 2;
            }
            "--strict-stage-b" => {
                run_options.strict_stage_b_container_execution = true;
                i += 1;
            }
            "--allow-stage-b-fallback" => {
                run_options.allow_stage_b_fallback = true;
                i += 1;
            }
            "--stream-steps" => {
                run_options.stream_steps = true;
                i += 1;
            }
            "--trace-profile" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--trace-profile requires lean|debug".to_string(),
                    ));
                }

                run_options.trace_profile = parse_trace_profile(&args[i + 1])?;
                i += 2;
            }
            "--trace-steps" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--trace-steps requires an integer >= 0".to_string(),
                    ));
                }

                run_options.trace_steps = Some(parse_usize_flag("--trace-steps", &args[i + 1])?);
                i += 2;
            }
            "--trace-projection" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--trace-projection requires minimal|full".to_string(),
                    ));
                }

                run_options.trace_projection = Some(parse_trace_projection(&args[i + 1])?);
                i += 2;
            }
            "--trace-max-string-bytes" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--trace-max-string-bytes requires an integer >= 0".to_string(),
                    ));
                }

                run_options.trace_max_string_bytes = Some(parse_usize_flag(
                    "--trace-max-string-bytes",
                    &args[i + 1],
                )?);
                i += 2;
            }
            "--type-policy" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--type-policy requires warn|strict".to_string(),
                    ));
                }

                run_options.executable_kind_policy_mode = parse_executable_kind_policy_mode(
                    "--type-policy",
                    &args[i + 1],
                )?;
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

    Ok((file_path, run_options))
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

fn parse_aot_stage_selection(value: &str) -> Result<AotStageSelection, CompilerError> {
    match value {
        "stage_a" => Ok(AotStageSelection::StageA),
        "stage_b" => Ok(AotStageSelection::StageB),
        _ => Err(CompilerError::RuntimeError(format!(
            "invalid --aot-stage '{}', expected stage_a|stage_b",
            value
        ))),
    }
}

fn parse_executable_kind_policy_mode(
    flag: &str,
    value: &str,
) -> Result<ExecutableKindPolicyMode, CompilerError> {
    match value {
        "warn" => Ok(ExecutableKindPolicyMode::Compatibility),
        "strict" => Ok(ExecutableKindPolicyMode::StrictMutationOnly),
        _ => Err(CompilerError::RuntimeError(format!(
            "invalid {} '{}', expected warn|strict",
            flag, value
        ))),
    }
}

fn resolve_stage_b_strict_mode(run_options: &RunOptions) -> bool {
    let stage_b_selected = matches!(run_options.aot_stage, Some(AotStageSelection::StageB));
    run_options.strict_stage_b_container_execution
        || (stage_b_selected && !run_options.allow_stage_b_fallback)
}

fn parse_usize_flag(flag: &str, value: &str) -> Result<usize, CompilerError> {
    value.parse::<usize>().map_err(|_| {
        CompilerError::RuntimeError(format!("invalid {} value '{}', expected integer >= 0", flag, value))
    })
}

fn emit_parse_cmd(args: &[String]) -> Result<(), CompilerError> {
    let (file_path, consumed) = resolve_file_path_arg_or_project_main(args, "parse")?;
    let mut output_mode = DiscoveryOutputMode::Yaml;

    for flag in &args[consumed..] {
        if let Some(mode) = parse_structured_output_flag(flag) {
            output_mode = mode;
            continue;
        }

        return Err(CompilerError::RuntimeError(format!(
            "unknown parse flag '{}'",
            flag
        )));
    }

    emit_parse(&file_path, output_mode)
}

fn emit_parse(file_path: &str, output_mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let source = read_source(file_path)?;
    let program = grapheme_compiler::parse(&source)?;
    print_discovery(&program, output_mode)?;
    Ok(())
}

fn emit_compile_cmd(args: &[String]) -> Result<(), CompilerError> {
    let (file_path, consumed) = resolve_file_path_arg_or_project_main(args, "compile")?;
    let mut emit_target = "mir".to_string();
    let mut aot_stage = AotStageSelection::StageA;
    let mut output_mode = DiscoveryOutputMode::Yaml;
    let mut executable_kind_policy_mode = ExecutableKindPolicyMode::Compatibility;
    let mut i = consumed;
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
            "--aot-stage" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--aot-stage requires stage_a|stage_b".to_string(),
                    ));
                }

                aot_stage = parse_aot_stage_selection(&args[i + 1])?;
                i += 2;
            }
            "--type-policy" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--type-policy requires warn|strict".to_string(),
                    ));
                }

                executable_kind_policy_mode = parse_executable_kind_policy_mode(
                    "--type-policy",
                    &args[i + 1],
                )?;
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

    let source = read_source(&file_path)?;
    let mut compiler_options = CompilerOptions::default();
    compiler_options.compile_options.executable_kind_policy_mode = executable_kind_policy_mode;
    let compilation = Compiler::compile_source(&source, compiler_options)?.compilation;

    if emit_target != "aot" && aot_stage != AotStageSelection::StageA {
        return Err(CompilerError::RuntimeError(
            "--aot-stage is only valid when --emit aot".to_string(),
        ));
    }

    match emit_target.as_str() {
        "ast" => print_discovery(&compilation.ast, output_mode)?,
        "hir" => print_discovery(&compilation.hir, output_mode)?,
        "mir" => print_discovery(&compilation.mir, output_mode)?,
        "artifact" => {
            let artifact = grapheme_artifact::build_artifact_from_mir(&compilation.mir, None)
                .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?;
            print_discovery(&artifact, output_mode)?;
        }
        "aot" => {
            let artifact = grapheme_artifact::build_artifact_from_mir(&compilation.mir, None)
                .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?;
            let stage_a = grapheme_artifact::build_aot_from_artifact(&artifact)
                .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?;
            let aot = match aot_stage {
                AotStageSelection::StageA => stage_a,
                AotStageSelection::StageB => {
                    let imports = STAGE_B_DEFAULT_ALLOWED_IMPORTS
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>();
                    grapheme_artifact::build_stage_b_container_from_aot(
                        &stage_a,
                        STAGE_B_DEFAULT_WORKFLOW_WASM_BYTES,
                        &imports,
                    )
                    .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?
                }
            };
            print_discovery(&aot, output_mode)?;
        }
        other => {
            return Err(CompilerError::RuntimeError(format!(
                "unsupported emit target '{}'; expected ast|hir|mir|artifact|aot",
                other
            )));
        }
    }

    Ok(())
}

fn emit_build_cmd(args: &[String]) -> Result<(), CompilerError> {
    let (file_path, consumed) = resolve_file_path_arg_or_project_main(args, "build")?;
    let mut aot_stage = AotStageSelection::StageB;
    let mut output_mode = DiscoveryOutputMode::Json;
    let mut output_path: Option<PathBuf> = None;
    let mut i = consumed;
    while i < args.len() {
        if let Some(mode) = parse_structured_output_flag(&args[i]) {
            output_mode = mode;
            i += 1;
            continue;
        }

        match args[i].as_str() {
            "--aot-stage" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--aot-stage requires stage_a|stage_b".to_string(),
                    ));
                }
                aot_stage = parse_aot_stage_selection(&args[i + 1])?;
                i += 2;
            }
            "--out" => {
                if i + 1 >= args.len() {
                    return Err(CompilerError::RuntimeError(
                        "--out requires a path".to_string(),
                    ));
                }
                output_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                return Err(CompilerError::RuntimeError(format!(
                    "unknown build flag '{}'",
                    other
                )));
            }
        }
    }

    let source = read_source(&file_path)?;
    let compilation = grapheme_compiler::compile(&source)?;
    let artifact = grapheme_artifact::build_artifact_from_mir(&compilation.mir, None)
        .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?;
    let stage_a = grapheme_artifact::build_aot_from_artifact(&artifact)
        .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?;
    let aot = match aot_stage {
        AotStageSelection::StageA => stage_a,
        AotStageSelection::StageB => {
            let imports = STAGE_B_DEFAULT_ALLOWED_IMPORTS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            grapheme_artifact::build_stage_b_container_from_aot(
                &stage_a,
                STAGE_B_DEFAULT_WORKFLOW_WASM_BYTES,
                &imports,
            )
            .map_err(|e| CompilerError::ArtifactEmitError(e.to_string()))?
        }
    };

    let output = format_discovery(&aot, output_mode)?;
    let out_path = output_path.unwrap_or_else(|| default_build_output_path(&file_path, output_mode));
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            CompilerError::RuntimeError(format!(
                "create output directory '{}': {e}",
                parent.display()
            ))
        })?;
    }
    fs::write(&out_path, output).map_err(|e| {
        CompilerError::RuntimeError(format!(
            "write build output '{}': {e}",
            out_path.display()
        ))
    })?;

    let manifest = build_manifest_from_aot(&file_path, &out_path, aot_stage, &aot);
    let manifest_path = default_build_manifest_path(&out_path);
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| CompilerError::RuntimeError(format!("serialize build manifest: {e}")))?;
    fs::write(&manifest_path, manifest_json).map_err(|e| {
        CompilerError::RuntimeError(format!(
            "write build manifest '{}': {e}",
            manifest_path.display()
        ))
    })?;

    println!("Built {}", out_path.display());
    println!("Built {}", manifest_path.display());
    Ok(())
}

fn default_build_output_path(file_path: &str, mode: DiscoveryOutputMode) -> PathBuf {
    let ext = match mode {
        DiscoveryOutputMode::Json => "json",
        DiscoveryOutputMode::Yaml => "yaml",
    };
    PathBuf::from(format!("{file_path}.aot.{ext}"))
}

fn default_build_manifest_path(output_path: &PathBuf) -> PathBuf {
    PathBuf::from(format!("{}.manifest.json", output_path.display()))
}

fn build_manifest_from_aot(
    source: &str,
    output_path: &PathBuf,
    stage: AotStageSelection,
    aot: &grapheme_artifact::AotEnvelope,
) -> AotBuildManifest {
    AotBuildManifest {
        source: source.to_string(),
        output: output_path.display().to_string(),
        stage: match stage {
            AotStageSelection::StageA => "stage_a".to_string(),
            AotStageSelection::StageB => "stage_b".to_string(),
        },
        aot_id: aot.aot_id.clone(),
        runtime_contract: aot.compatibility.runtime_contract.clone(),
        host_interface_id: aot.payload.host_interface_id.clone(),
    }
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

fn format_discovery<T: serde::Serialize>(value: &T, mode: DiscoveryOutputMode) -> Result<String, CompilerError> {
    match mode {
        DiscoveryOutputMode::Json => serde_json::to_string_pretty(value)
            .map_err(|e| CompilerError::RuntimeError(format!("serialize output: {e}"))),
        DiscoveryOutputMode::Yaml => serde_yaml::to_string(value)
            .map_err(|e| CompilerError::RuntimeError(format!("serialize output: {e}"))),
    }
}

fn print_discovery<T: serde::Serialize>(value: &T, mode: DiscoveryOutputMode) -> Result<(), CompilerError> {
    let output = format_discovery(value, mode)?;
    print!("{output}");
    Ok(())
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  grapheme                    # run project main from grapheme.toml");
    eprintln!("  grapheme <file.gr>");
    eprintln!("  grapheme parse [<file.gr>] [--yaml|--json]");
    eprintln!("  grapheme compile [<file.gr>] [--emit ast|hir|mir|artifact|aot] [--aot-stage stage_a|stage_b] [--type-policy warn|strict] [--yaml|--json]");
    eprintln!("  grapheme build [<file.gr>] [--aot-stage stage_a|stage_b] [--out path] [--yaml|--json]");
    eprintln!("  grapheme plugins build [all|core|io ...]");
    eprintln!("  grapheme examples [list] [--yaml|--json] [--query q] [--tag tag] [--complexity level] [--native-only]");
    eprintln!("  grapheme examples show <name> [--summary] [--raw] [--yaml|--json]");
    eprintln!("  grapheme examples init [--out dir]");
    eprintln!("  grapheme run [<file.gr>] [--bind module=path.wasm ...] [--json] [--native-modules] [--aot-stage stage_a|stage_b] [--type-policy warn|strict] [--strict-stage-b] [--allow-stage-b-fallback] [--stream-steps]");
    eprintln!("               [--trace-profile lean|debug] [--trace-steps N]");
    eprintln!("               [--trace-projection minimal|full] [--trace-max-string-bytes N]");
    eprintln!("  grapheme modules [--yaml|--json]");
    eprintln!("  grapheme modules search <query> [--explain] [--detail concise|full] [--top N] [--min-score X] [--yaml|--json]");
    eprintln!("  grapheme modules ops <query> [--yaml|--json]");
    eprintln!("  grapheme modules info <module> [--yaml|--json]");
    eprintln!("  grapheme modules types <module> [--yaml|--json]");
    eprintln!("  grapheme modules examples <module> [--yaml|--json]");
    eprintln!("  grapheme help");
}

fn print_modules_usage() {
    eprintln!("usage:");
    eprintln!("  grapheme modules [--yaml|--json]");
    eprintln!("  grapheme modules search <query> [--explain] [--detail concise|full] [--top N] [--min-score X] [--yaml|--json]");
    eprintln!("  grapheme modules ops <query> [--yaml|--json]");
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn normalized(s: &str) -> String {
        s.replace("\r\n", "\n")
    }

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
    fn parse_run_args_supports_aot_stage_and_strict_stage_b_flags() {
        let args = vec![
            "examples/hello.gr".to_string(),
            "--aot-stage".to_string(),
            "stage_b".to_string(),
            "--strict-stage-b".to_string(),
            "--json".to_string(),
        ];

        let (_file, run_options) = parse_run_args(&args).expect("parse run args should succeed");

        assert!(matches!(run_options.aot_stage, Some(AotStageSelection::StageB)));
        assert!(run_options.strict_stage_b_container_execution);
        assert!(matches!(run_options.output_mode, RunOutputMode::Json));
    }

    #[test]
    fn parse_run_args_supports_type_policy_strict() {
        let args = vec![
            "examples/hello.gr".to_string(),
            "--type-policy".to_string(),
            "strict".to_string(),
        ];

        let (_file, run_options) = parse_run_args(&args).expect("parse run args should succeed");
        assert_eq!(
            run_options.executable_kind_policy_mode,
            ExecutableKindPolicyMode::StrictMutationOnly
        );
    }

    #[test]
    fn parse_executable_kind_policy_mode_rejects_unknown_value() {
        let err = parse_executable_kind_policy_mode("--type-policy", "aggressive")
            .expect_err("invalid type-policy should fail");
        assert!(err.to_string().contains("expected warn|strict"));
    }

    #[test]
    fn resolve_project_main_path_from_dir_uses_grapheme_toml_main() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = env::temp_dir().join(format!("grapheme-cli-project-main-{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");

        fs::write(
            dir.join("grapheme.toml"),
            r#""$schema" = "./grapheme.schema.json"

[project]
name = "demo"
main = "examples/main.gr"

[examples.namespaces]
core = "examples"
showcase = "examples/legacy/showcase"
"#,
        )
        .expect("write project config");

        let file = resolve_project_main_path_from_dir(&dir)
            .expect("resolve project main should succeed");
        let _ = fs::remove_dir_all(&dir);

        assert!(file.ends_with("examples/main.gr"));
    }

    #[test]
    fn resolve_project_main_path_from_dir_rejects_duplicate_glyph_names_across_namespaces() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = env::temp_dir().join(format!("grapheme-cli-glyph-dup-{unique}"));
        fs::create_dir_all(dir.join("examples")).expect("create examples dir");
        fs::create_dir_all(dir.join("examples/legacy/showcase")).expect("create showcase dir");

        fs::write(
            dir.join("grapheme.toml"),
            r#""$schema" = "./grapheme.schema.json"

[project]
name = "dup"
main = "examples/main.gr"

[examples.namespaces]
core = "examples"
showcase = "examples/legacy/showcase"
"#,
        )
        .expect("write config");

        fs::write(
            dir.join("examples/main.gr"),
            "glyph Main { core.echo(message: \"a\") }\n",
        )
        .expect("write main");

        fs::write(
            dir.join("examples/legacy/showcase/another.gr"),
            "glyph Main { core.echo(message: \"b\") }\n",
        )
        .expect("write duplicate glyph");

        let err = resolve_project_main_path_from_dir(&dir)
            .expect_err("duplicate glyph names should fail");
        let _ = fs::remove_dir_all(&dir);

        assert!(err.to_string().contains("glyph names must be unique"));
    }

    #[test]
    fn resolve_stage_b_strict_mode_defaults_to_strict_for_stage_b_runs() {
        let run_options = RunOptions {
            bindings: vec![],
            output_mode: RunOutputMode::Plain,
            native_modules: false,
            aot_stage: Some(AotStageSelection::StageB),
            strict_stage_b_container_execution: false,
            allow_stage_b_fallback: false,
            stream_steps: false,
            trace_profile: TraceProfile::Lean,
            trace_steps: None,
            trace_projection: None,
            trace_max_string_bytes: None,
            executable_kind_policy_mode: ExecutableKindPolicyMode::Compatibility,
        };

        assert!(resolve_stage_b_strict_mode(&run_options));
    }

    #[test]
    fn resolve_stage_b_strict_mode_allows_explicit_fallback_opt_out() {
        let run_options = RunOptions {
            bindings: vec![],
            output_mode: RunOutputMode::Plain,
            native_modules: false,
            aot_stage: Some(AotStageSelection::StageB),
            strict_stage_b_container_execution: false,
            allow_stage_b_fallback: true,
            stream_steps: false,
            trace_profile: TraceProfile::Lean,
            trace_steps: None,
            trace_projection: None,
            trace_max_string_bytes: None,
            executable_kind_policy_mode: ExecutableKindPolicyMode::Compatibility,
        };

        assert!(!resolve_stage_b_strict_mode(&run_options));
    }

    #[test]
    fn parse_aot_stage_selection_rejects_unknown_value() {
        let err = parse_aot_stage_selection("stage_c")
            .expect_err("invalid aot stage should fail");
        assert!(err.to_string().contains("expected stage_a|stage_b"));
    }

    #[test]
    fn default_build_output_path_uses_expected_suffixes() {
        let json = default_build_output_path("examples/hello.gr", DiscoveryOutputMode::Json);
        let yaml = default_build_output_path("examples/hello.gr", DiscoveryOutputMode::Yaml);

        assert_eq!(json.to_string_lossy(), "examples/hello.gr.aot.json");
        assert_eq!(yaml.to_string_lossy(), "examples/hello.gr.aot.yaml");
    }

    #[test]
    fn default_build_manifest_path_uses_output_suffix() {
        let out = PathBuf::from("examples/hello.gr.aot.json");
        let manifest = default_build_manifest_path(&out);
        assert_eq!(
            manifest.to_string_lossy(),
            "examples/hello.gr.aot.json.manifest.json"
        );
    }

    #[test]
    fn golden_aot_build_manifest_snapshot_contract() {
        let source = r#"import core from "grapheme/core"

query Hello {
    core.echo(message: "hello-aot") {
        state { current }
    }
}
"#;

        let artifact = grapheme_compiler::compile_to_artifact(source, Some("Hello"))
            .expect("artifact compile should succeed");
        let stage_a = grapheme_artifact::build_aot_from_artifact(&artifact)
            .expect("stage_a build should succeed");
        let imports = STAGE_B_DEFAULT_ALLOWED_IMPORTS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let stage_b = grapheme_artifact::build_stage_b_container_from_aot(
            &stage_a,
            STAGE_B_DEFAULT_WORKFLOW_WASM_BYTES,
            &imports,
        )
        .expect("stage_b build should succeed");

        let out = PathBuf::from("build/hello.aot.json");
        let manifest = build_manifest_from_aot("examples/hello.gr", &out, AotStageSelection::StageB, &stage_b);
        let rendered = serde_json::to_string_pretty(&manifest)
            .expect("serialize build manifest snapshot");
        let expected = include_str!("../tests/golden/aot-build-manifest.snapshot.json");
        assert_eq!(
            normalized(&rendered).trim_end(),
            normalized(expected).trim_end()
        );
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
    fn example_discovery_row_contains_summary_and_run_hint() {
        let row = discover_examples(None, None, None, false)
            .into_iter()
            .find(|e| e.name == "main")
            .expect("main discovery row exists");

        assert_eq!(row.name, "main");
        assert!(!row.summary.is_empty());
        assert_eq!(row.run, "grapheme run examples/main.gr");
    }

    #[test]
    fn examples_show_rejects_summary_and_raw_together() {
        let err = emit_examples_cmd(&[
            "show".to_string(),
            "main".to_string(),
            "--summary".to_string(),
            "--raw".to_string(),
        ])
        .expect_err("conflicting examples show flags should fail");

        assert!(err
            .to_string()
            .contains("--summary and --raw cannot be combined"));
    }

    #[test]
    fn example_matches_filters_supports_tag_complexity_and_query() {
        let matching = discover_examples(
            Some("fallback"),
            Some("routing"),
            Some("advanced"),
            false,
        );
        assert!(matching.iter().any(|e| e.name == "web-provider-routing"));

        let wrong_tag = discover_examples(None, Some("smtp"), None, false);
        assert!(wrong_tag.is_empty());
    }

    #[test]
    fn examples_list_rejects_missing_filter_values() {
        let err = emit_examples_cmd(&[
            "list".to_string(),
            "--tag".to_string(),
        ])
        .expect_err("missing tag value should fail");

        assert!(err.to_string().contains("--tag requires a value"));
    }

    #[test]
    fn parse_modules_search_args_supports_explain_flag() {
        let (query, options) = parse_modules_search_args(&[
            "web".to_string(),
            "--explain".to_string(),
        ])
        .expect("modules search args should parse");

        assert_eq!(query, "web");
        assert!(options.explain);
        assert_eq!(options.detail, ModuleSearchDetail::Full);
        assert!(options.top.is_none());
        assert!(options.min_score.is_none());
    }

    #[test]
    fn parse_modules_search_args_supports_detail_flag() {
        let (query, options) = parse_modules_search_args(&[
            "web".to_string(),
            "--detail".to_string(),
            "concise".to_string(),
        ])
        .expect("modules search detail should parse");

        assert_eq!(query, "web");
        assert!(options.explain);
        assert_eq!(options.detail, ModuleSearchDetail::Concise);
    }

    #[test]
    fn parse_modules_search_args_supports_top_and_min_score() {
        let (_query, options) = parse_modules_search_args(&[
            "web".to_string(),
            "--top".to_string(),
            "1".to_string(),
            "--min-score".to_string(),
            "100".to_string(),
        ])
        .expect("modules search top/min-score should parse");

        assert!(options.explain);
        assert_eq!(options.top, Some(1));
        assert_eq!(options.min_score, Some(100.0));
    }

    #[test]
    fn parse_modules_search_args_rejects_missing_detail_value() {
        let err = parse_modules_search_args(&[
            "web".to_string(),
            "--detail".to_string(),
        ])
        .expect_err("missing detail value should fail");

        assert!(err.to_string().contains("--detail requires concise|full"));
    }

    #[test]
    fn parse_modules_search_args_rejects_unknown_flag() {
        let err = parse_modules_search_args(&[
            "web".to_string(),
            "--oops".to_string(),
        ])
        .expect_err("unknown modules search flag should fail");

        assert!(err
            .to_string()
            .contains("expected --explain|--detail|--top|--min-score"));
    }

    #[test]
    fn search_modules_payload_explain_includes_guidance_fields() {
        let payload = modules_search_payload(
            "web",
            &ModuleSearchOptions {
                explain: true,
                detail: ModuleSearchDetail::Full,
                top: None,
                min_score: None,
            },
        );
        let matches = payload
            .get("matches")
            .and_then(|v| v.as_array())
            .expect("matches array present");

        let web = matches
            .iter()
            .find(|item| item.get("module_id").and_then(|v| v.as_str()) == Some("web"))
            .expect("web module present in explain matches");

        assert!(web.get("summary").and_then(|v| v.as_str()).is_some());
        assert!(web.get("use_when").and_then(|v| v.as_str()).is_some());
        assert!(web.get("avoid_when").and_then(|v| v.as_str()).is_some());
        assert!(web.get("score").and_then(|v| v.as_f64()).is_some());
        assert!(web
            .get("related_examples")
            .and_then(|v| v.as_array())
            .is_some());
    }

    #[test]
    fn search_modules_payload_concise_excludes_full_guidance_fields() {
        let payload = modules_search_payload(
            "web",
            &ModuleSearchOptions {
                explain: true,
                detail: ModuleSearchDetail::Concise,
                top: None,
                min_score: None,
            },
        );
        let matches = payload
            .get("matches")
            .and_then(|v| v.as_array())
            .expect("matches array present");
        let first = matches.first().expect("at least one match");

        assert!(first.get("score").and_then(|v| v.as_f64()).is_some());
        assert!(first.get("summary").and_then(|v| v.as_str()).is_some());
        assert!(first.get("use_when").is_none());
        assert!(first.get("avoid_when").is_none());
        assert!(first.get("matching_ops").is_none());
    }

    #[test]
    fn search_modules_payload_applies_top_and_min_score() {
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
        let matches = payload
            .get("matches")
            .and_then(|v| v.as_array())
            .expect("matches array present");
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].get("module_id").and_then(|v| v.as_str()),
            Some("web")
        );
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

    #[test]
    fn golden_modules_examples_yaml_contract() {
        let payload = modules_examples_payload("core").expect("core examples payload");

        let actual = format_discovery(&payload, DiscoveryOutputMode::Yaml)
            .expect("yaml format should succeed");
        let expected = include_str!("../tests/golden/modules-examples-core.yaml");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_modules_examples_json_contract() {
        let payload = modules_examples_payload("core").expect("core examples payload");

        let actual = format_discovery(&payload, DiscoveryOutputMode::Json)
            .expect("json format should succeed");
        let expected = include_str!("../tests/golden/modules-examples-core.json");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_modules_examples_sql_yaml_contract() {
        let payload = modules_examples_payload("sql").expect("sql examples payload");

        let actual = format_discovery(&payload, DiscoveryOutputMode::Yaml)
            .expect("yaml format should succeed");
        let expected = include_str!("../tests/golden/modules-examples-sql.yaml");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_modules_examples_sql_json_contract() {
        let payload = modules_examples_payload("sql").expect("sql examples payload");

        let actual = format_discovery(&payload, DiscoveryOutputMode::Json)
            .expect("json format should succeed");
        let expected = include_str!("../tests/golden/modules-examples-sql.json");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_modules_examples_surreal_yaml_contract() {
        let payload = modules_examples_payload("surreal").expect("surreal examples payload");

        let actual = format_discovery(&payload, DiscoveryOutputMode::Yaml)
            .expect("yaml format should succeed");
        let expected = include_str!("../tests/golden/modules-examples-surreal.yaml");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_modules_examples_surreal_json_contract() {
        let payload = modules_examples_payload("surreal").expect("surreal examples payload");

        let actual = format_discovery(&payload, DiscoveryOutputMode::Json)
            .expect("json format should succeed");
        let expected = include_str!("../tests/golden/modules-examples-surreal.json");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_modules_ops_core_yaml_contract() {
        let payload = modules_ops_payload("core");

        let actual = format_discovery(&payload, DiscoveryOutputMode::Yaml)
            .expect("yaml format should succeed");
        let expected = include_str!("../tests/golden/modules-ops-core.yaml");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_modules_ops_core_json_contract() {
        let payload = modules_ops_payload("core");

        let actual = format_discovery(&payload, DiscoveryOutputMode::Json)
            .expect("json format should succeed");
        let expected = include_str!("../tests/golden/modules-ops-core.json");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_parse_yaml_contract() {
                let source = r#"import core from "grapheme/core"

query HelloWorld {
    core.echo(message: "LETS GO?!!!!!") {
    state { current }
  }
}
"#;

        let program = grapheme_compiler::parse(source).expect("parse should succeed");
        let actual = format_discovery(&program, DiscoveryOutputMode::Yaml)
            .expect("yaml format should succeed");
        let expected = include_str!("../tests/golden/parse-hello-world.yaml");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_parse_json_contract() {
                let source = r#"import core from "grapheme/core"

query HelloWorld {
    core.echo(message: "LETS GO?!!!!!") {
    state { current }
  }
}
"#;

        let program = grapheme_compiler::parse(source).expect("parse should succeed");
        let actual = format_discovery(&program, DiscoveryOutputMode::Json)
            .expect("json format should succeed");
        let expected = include_str!("../tests/golden/parse-hello-world.json");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_compile_mir_yaml_contract() {
                let source = r#"import core from "grapheme/core"

query HelloWorld {
    core.echo(message: "LETS GO?!!!!!") {
    state { current }
  }
}
"#;

        let compilation = grapheme_compiler::compile(source).expect("compile should succeed");
        let actual = format_discovery(&compilation.mir, DiscoveryOutputMode::Yaml)
            .expect("yaml format should succeed");
        let expected = include_str!("../tests/golden/compile-mir-hello-world.yaml");
        assert_eq!(normalized(&actual), normalized(expected));
    }

    #[test]
    fn golden_compile_mir_json_contract() {
                let source = r#"import core from "grapheme/core"

query HelloWorld {
    core.echo(message: "LETS GO?!!!!!") {
    state { current }
  }
}
"#;

        let compilation = grapheme_compiler::compile(source).expect("compile should succeed");
        let actual = format_discovery(&compilation.mir, DiscoveryOutputMode::Json)
            .expect("json format should succeed");
        let expected = include_str!("../tests/golden/compile-mir-hello-world.json");
        assert_eq!(normalized(&actual), normalized(expected));
    }

}
