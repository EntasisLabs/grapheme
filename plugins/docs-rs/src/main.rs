use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Read, Write};

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    args: Value,
}

fn arg_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(ToOwned::to_owned)
}

fn write_json(value: &Value) {
    let mut stdout = io::stdout();
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let _ = stdout.write_all(&bytes);
    let _ = stdout.flush();
}

fn native_module_guide(topic: &str) -> Value {
    let base = json!({
        "title": "Grapheme Native Module Guide",
        "overview": [
            "Native modules are Wasm binaries executed through the Grapheme Wasix runtime path.",
            "Each module advertises operations in runtime manifests and receives JSON request payloads over stdin.",
            "Modules return JSON on stdout and should keep responses deterministic and schema-stable."
        ],
        "authoring_steps": [
            "Create a Rust crate with one binary entrypoint and serde/serde_json dependencies.",
            "Read a request JSON object from stdin with fields: op (string), args (object).",
            "Dispatch by op and return structured JSON output for each operation.",
            "Build with cargo --release --target wasm32-wasip1.",
            "Bind the produced .wasm using grapheme run --bind module=path.wasm or use --native-modules when wired in CLI specs."
        ],
        "request_contract": {
            "stdin": { "op": "string", "args": "json object" },
            "stdout": "json value"
        },
        "best_practices": [
            "Validate required args and return explicit error objects.",
            "Prefer stable key names and avoid shape drift across versions.",
            "Keep side effects explicit by operation name.",
            "Version module behavior and document operation arg types."
        ],
        "next": "Use docs.native_module_example(module: \"http\") for a concrete example and docs.native_module_registry() for the current module catalog."
    });

    match topic {
        "contract" => json!({ "topic": "contract", "request_contract": base["request_contract"] }),
        "steps" => json!({ "topic": "steps", "authoring_steps": base["authoring_steps"] }),
        "best_practices" => json!({ "topic": "best_practices", "best_practices": base["best_practices"] }),
        _ => base,
    }
}

fn native_module_registry() -> Value {
    json!({
        "modules": [
            { "id": "core", "ops": ["echo", "map", "filter", "merge", "pick", "validate_schema"] },
            { "id": "docs", "ops": ["native_module_guide", "native_module_registry", "native_module_example"] },
            { "id": "io", "ops": ["read_text", "write_text", "list_dir"] },
            { "id": "http", "ops": ["get", "post"] },
            { "id": "tcp", "ops": ["connect", "send", "receive"] },
            { "id": "smtp", "ops": ["send_mail"] },
            { "id": "memory", "ops": ["store_context", "load_context", "summarize_context"] },
            { "id": "secrets", "ops": ["get_secret_handle", "sign_request"] }
        ],
        "note": "This registry is a runtime-facing module index for authoring and discovery."
    })
}

fn native_module_example(module: &str) -> Value {
    match module {
        "http" => json!({
            "module": "http",
            "example_request": {
                "op": "get",
                "args": { "url": "https://httpbin.org/get" }
            },
            "example_gr": "import HTTP from \"grapheme/http\"\n\nquery HttpGetDemo {\n  HTTP.get(url: \"https://httpbin.org/get\") {\n    state { current }\n  }\n}"
        }),
        "core" => json!({
            "module": "core",
            "example_request": {
                "op": "pick",
                "args": { "fields": ["status", "url"] }
            },
            "example_gr": "import Core from \"grapheme/core\"\n\nquery PickDemo {\n  Core.pick(fields: [\"status\", \"url\"]) {\n    state { current }\n  }\n}"
        }),
        _ => json!({
            "module": module,
            "error": "no embedded example for this module yet",
            "hint": "Try module = \"http\" or module = \"core\""
        }),
    }
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        write_json(&json!({ "error": "failed to read request" }));
        return;
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => {
            write_json(&json!({ "error": "invalid request json" }));
            return;
        }
    };

    let result = match req.op.as_str() {
        "native_module_guide" => {
            let topic = arg_string(&req.args, "topic").unwrap_or_else(|| "all".to_string());
            native_module_guide(&topic)
        }
        "native_module_registry" => native_module_registry(),
        "native_module_example" => {
            let module = arg_string(&req.args, "module").unwrap_or_else(|| "http".to_string());
            native_module_example(&module)
        }
        other => json!({ "error": format!("unsupported docs op: {other}") }),
    };

    write_json(&result);
}
