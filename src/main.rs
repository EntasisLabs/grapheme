/// ─────────────────────────────────────────────────────────────
///  AgentQL  —  CLI
///  Usage: agentql <file.aql>
///  Parses the file, prints the AST as JSON.
/// ─────────────────────────────────────────────────────────────

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("usage: agentql <file.aql>");
        process::exit(1);
    }

    let source = fs::read_to_string(&args[1]).unwrap_or_else(|e| {
        eprintln!("error reading {}: {e}", args[1]);
        process::exit(1);
    });

    match agentql::parse(&source) {
        Ok(program) => {
            let json = serde_json::to_string_pretty(&program).unwrap();
            println!("{json}");
        }
        Err(e) => {
            eprintln!("parse error:\n{e}");
            process::exit(1);
        }
    }
}
