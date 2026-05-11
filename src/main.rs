/// ─────────────────────────────────────────────────────────────
///  Grapheme  —  CLI
///  Usage:
///    grapheme <file.gr>
///    grapheme parse <file.gr>
///    grapheme compile <file.gr> --emit ast|hir|mir|artifact
///    grapheme run <file.gr>
/// ─────────────────────────────────────────────────────────────

use grapheme::host::{CapabilityCall, CapabilityHost, HostCallError};
use serde::Serialize;
use serde_json::json;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if let Err(e) = run(args) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), grapheme::GraphemeError> {
    if args.len() < 2 {
        print_usage();
        return Err(grapheme::GraphemeError::RuntimeError(
            "missing command or file path".to_string(),
        ));
    }

    // Backwards-compatible mode: `grapheme file.gr` maps to parse.
    if args.len() == 2 && args[1] != "parse" && args[1] != "compile" && args[1] != "run" {
        return emit_parse(&args[1]);
    }

    match args[1].as_str() {
        "parse" => {
            if args.len() != 3 {
                print_usage();
                return Err(grapheme::GraphemeError::RuntimeError(
                    "parse requires exactly one file path".to_string(),
                ));
            }
            emit_parse(&args[2])
        }
        "compile" => emit_compile(&args),
        "run" => {
            if args.len() != 3 {
                print_usage();
                return Err(grapheme::GraphemeError::RuntimeError(
                    "run requires exactly one file path".to_string(),
                ));
            }
            run_program(&args[2])
        }
        _ => {
            print_usage();
            Err(grapheme::GraphemeError::RuntimeError(format!(
                "unknown command '{}'",
                args[1]
            )))
        }
    }
}

#[derive(Serialize)]
struct CliRunOutput {
    artifact_id: String,
    execution: grapheme::ExecutionResult,
    final_state: serde_json::Value,
}

struct CliHost;

impl CapabilityHost for CliHost {
    fn call(&mut self, call: &CapabilityCall) -> Result<serde_json::Value, HostCallError> {
        // Runtime demo host: production adapters can map capabilities to tool/plugin invocations.
        Ok(json!({
            "capability": call.capability,
            "arg_count": call.arg_count,
            "step_index": call.step_index,
            "status": "ok"
        }))
    }
}

fn run_program(file_path: &str) -> Result<(), grapheme::GraphemeError> {
    let source = read_source(file_path)?;
    let compiled = grapheme::Compiler::compile_source(&source, grapheme::CompilerOptions::default())?;

    let mut host = CliHost;
    let runtime = grapheme::RuntimeEngine::default();
    let (state, execution) = runtime.execute_artifact(&compiled.artifact, &mut host)?;

    let out = CliRunOutput {
        artifact_id: compiled.artifact.artifact_id,
        execution,
        final_state: state.to_json(),
    };

    print_json(&out)
}

fn emit_parse(file_path: &str) -> Result<(), grapheme::GraphemeError> {
    let source = read_source(file_path)?;
    let program = grapheme::parse(&source)?;
    print_json(&program)?;
    Ok(())
}

fn emit_compile(args: &[String]) -> Result<(), grapheme::GraphemeError> {
    if args.len() < 3 {
        print_usage();
        return Err(grapheme::GraphemeError::RuntimeError(
            "compile requires a file path".to_string(),
        ));
    }

    let mut emit_target = "mir".to_string();
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--emit" => {
                if i + 1 >= args.len() {
                    return Err(grapheme::GraphemeError::RuntimeError(
                        "--emit requires a value".to_string(),
                    ));
                }
                emit_target = args[i + 1].clone();
                i += 2;
            }
            flag => {
                return Err(grapheme::GraphemeError::RuntimeError(format!(
                    "unknown compile flag '{}'",
                    flag
                )));
            }
        }
    }

    let source = read_source(&args[2])?;
    let compilation = grapheme::compile(&source)?;

    match emit_target.as_str() {
        "ast" => print_json(&compilation.ast)?,
        "hir" => print_json(&compilation.hir)?,
        "mir" => print_json(&compilation.mir)?,
        "artifact" => {
            let artifact = grapheme::artifact::build_artifact_from_compilation(&compilation, None)?;
            print_json(&artifact)?;
        }
        other => {
            return Err(grapheme::GraphemeError::RuntimeError(format!(
                "unsupported emit target '{}'; expected ast|hir|mir|artifact",
                other
            )));
        }
    }

    Ok(())
}

fn read_source(path: &str) -> Result<String, grapheme::GraphemeError> {
    fs::read_to_string(path)
        .map_err(|e| grapheme::GraphemeError::RuntimeError(format!("error reading {path}: {e}")))
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), grapheme::GraphemeError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| grapheme::GraphemeError::RuntimeError(format!("serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  grapheme <file.gr>");
    eprintln!("  grapheme parse <file.gr>");
    eprintln!("  grapheme compile <file.gr> --emit ast|hir|mir|artifact");
    eprintln!("  grapheme run <file.gr>");
}
