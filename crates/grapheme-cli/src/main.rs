/// ─────────────────────────────────────────────────────────────
///  Grapheme  —  CLI
///  Usage:
///    grapheme <file.aql>
///    grapheme parse <file.aql>
///    grapheme compile <file.aql> --emit ast|hir|mir|artifact
///    grapheme plugins build [all|core|io ...]
///    grapheme run <file.aql> [--bind module=path.wasm ...] [--json] [--native-modules]
///    grapheme modules
/// ─────────────────────────────────────────────────────────────

use grapheme_artifact::{ExecutionResult, MirInst};
use grapheme_compiler::{Compiler, CompilerError, CompilerOptions};
use grapheme_runtime::{CapabilityCall, CapabilityHost, HostCallError, PolicyGuard, RuntimeEngine};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunOutputMode {
    Plain,
    Json,
}

struct RunOptions {
    bindings: Vec<(String, PathBuf)>,
    output_mode: RunOutputMode,
    native_modules: bool,
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

    // Backwards-compatible mode: `grapheme file.aql` maps to parse.
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
        "modules" => emit_modules(),
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
                } = inst;

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
            ("http", "get") => {
                let url = call
                    .args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(host_http_request("GET", url, None))
            }
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
        },
    ))
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
    eprintln!("  grapheme <file.aql>");
    eprintln!("  grapheme parse <file.aql>");
    eprintln!("  grapheme compile <file.aql> --emit ast|hir|mir|artifact");
    eprintln!("  grapheme plugins build [all|core|io ...]");
    eprintln!("  grapheme run <file.aql> [--bind module=path.wasm ...] [--json] [--native-modules]");
    eprintln!("  grapheme modules");
}
