use crate::{core, csv, html, http, json as json_mod, research, smtp, tcp, web, yaml};
use serde_json::{json, Value as JsonValue};

struct RegisteredModule {
    module_id: &'static str,
    handler: fn(&str, &JsonValue) -> Option<JsonValue>,
}

struct RegisteredOp {
    op: &'static str,
    handler: fn(&JsonValue) -> JsonValue,
}

const REGISTERED_MODULES: &[RegisteredModule] = &[
    RegisteredModule {
        module_id: "core",
        handler: dispatch_core,
    },
    RegisteredModule {
        module_id: "http",
        handler: dispatch_http,
    },
    RegisteredModule {
        module_id: "web",
        handler: dispatch_web,
    },
    RegisteredModule {
        module_id: "websearch",
        handler: dispatch_websearch,
    },
    RegisteredModule {
        module_id: "tcp",
        handler: dispatch_tcp,
    },
    RegisteredModule {
        module_id: "smtp",
        handler: dispatch_smtp,
    },
    RegisteredModule {
        module_id: "html",
        handler: dispatch_html,
    },
    RegisteredModule {
        module_id: "json",
        handler: dispatch_json,
    },
    RegisteredModule {
        module_id: "csv",
        handler: dispatch_csv,
    },
    RegisteredModule {
        module_id: "yaml",
        handler: dispatch_yaml,
    },
];

const CORE_OPS: &[RegisteredOp] = &[
    RegisteredOp { op: "echo", handler: core::echo },
    RegisteredOp { op: "tap", handler: core::tap },
    RegisteredOp {
        op: "pack_state_data",
        handler: core::pack_state_data,
    },
    RegisteredOp {
        op: "get_state",
        handler: core::get_state,
    },
    RegisteredOp { op: "get_data", handler: core::get_data },
    RegisteredOp {
        op: "apply_lane",
        handler: core::apply_lane,
    },
    RegisteredOp { op: "map", handler: core::map },
    RegisteredOp {
        op: "filter",
        handler: core::filter,
    },
    RegisteredOp { op: "find", handler: core::find },
    RegisteredOp {
        op: "reduce",
        handler: core::reduce,
    },
    RegisteredOp {
        op: "group_by",
        handler: core::group_by,
    },
    RegisteredOp { op: "merge", handler: core::merge },
    RegisteredOp { op: "pick", handler: core::pick },
    RegisteredOp {
        op: "validate_schema",
        handler: core::validate_schema,
    },
    RegisteredOp { op: "split", handler: core::split },
    RegisteredOp { op: "join", handler: core::join },
    RegisteredOp {
        op: "replace",
        handler: core::replace,
    },
    RegisteredOp { op: "trim", handler: core::trim },
    RegisteredOp { op: "lower", handler: core::lower },
    RegisteredOp { op: "upper", handler: core::upper },
    RegisteredOp {
        op: "contains",
        handler: core::contains,
    },
    RegisteredOp {
        op: "get_path",
        handler: core::get_path,
    },
    RegisteredOp {
        op: "set_path",
        handler: core::set_path,
    },
    RegisteredOp {
        op: "has_path",
        handler: core::has_path,
    },
];

const JSON_OPS: &[RegisteredOp] = &[RegisteredOp {
    op: "parse",
    handler: json_mod::parse,
}];

const CSV_OPS: &[RegisteredOp] = &[RegisteredOp {
    op: "to_list",
    handler: csv::to_list,
}];

const YAML_OPS: &[RegisteredOp] = &[RegisteredOp {
    op: "to_json",
    handler: yaml::to_json,
}];

const HTML_OPS: &[RegisteredOp] = &[
    RegisteredOp {
        op: "to_md",
        handler: html::to_md,
    },
    RegisteredOp {
        op: "clean_text",
        handler: html::clean_text,
    },
];

pub fn dispatch(module: &str, op: &str, args: &JsonValue) -> Option<JsonValue> {
    REGISTERED_MODULES
        .iter()
        .find(|m| m.module_id == module)
        .and_then(|m| (m.handler)(op, args))
}

fn dispatch_core(op: &str, args: &JsonValue) -> Option<JsonValue> {
    dispatch_table(op, args, CORE_OPS)
}

fn dispatch_http(op: &str, args: &JsonValue) -> Option<JsonValue> {
    match op {
        "get" => {
            let req = HttpGetRequest::from_args(args);
            Some(http::request("GET", &req.url, None))
        }
        "post" => {
            let req = HttpPostRequest::from_args(args);
            Some(http::request("POST", &req.url, req.body.as_ref()))
        }
        _ => None,
    }
}

fn dispatch_web(op: &str, args: &JsonValue) -> Option<JsonValue> {
    match op {
        "duckduckgo" => Some(web::search_provider(args, "duckduckgo")),
        "google" => Some(web::search_provider(args, "google")),
        "xaviv" => Some(web::search_provider(args, "xaviv")),
        "providers" => Some(web::providers()),
        "capabilities" => Some(web::capabilities(args)),
        _ => None,
    }
}

fn dispatch_websearch(op: &str, args: &JsonValue) -> Option<JsonValue> {
    match op {
        "search" => {
            let req = WebsearchSearchRequest::from_args(args);
            Some(web::search(&req.to_args_json()))
        }
        "research_materials" => {
            let req = ResearchMaterialsRequest::from_args(args);
            Some(research::materials(&req.to_args_json()))
        }
        "research_report" => {
            let req = ResearchReportRequest::from_args(args);
            Some(research::report(&req.to_args_json()))
        }
        _ => None,
    }
}

fn dispatch_tcp(op: &str, args: &JsonValue) -> Option<JsonValue> {
    match op {
        "connect" => {
            let req = TcpConnectRequest::from_args(args);
            Some(tcp::connect(&req.target))
        }
        "send" => {
            let req = TcpSendRequest::from_args(args);
            Some(tcp::send(&req.target, &req.data))
        }
        "receive" => {
            let req = TcpReceiveRequest::from_args(args);
            Some(tcp::receive(&req.target, req.max_bytes))
        }
        _ => None,
    }
}

fn dispatch_smtp(op: &str, args: &JsonValue) -> Option<JsonValue> {
    match op {
        "send_mail" => Some(smtp::send_mail(args)),
        _ => None,
    }
}

fn dispatch_html(op: &str, args: &JsonValue) -> Option<JsonValue> {
    dispatch_table(op, args, HTML_OPS)
}

fn dispatch_json(op: &str, args: &JsonValue) -> Option<JsonValue> {
    dispatch_table(op, args, JSON_OPS)
}

fn dispatch_csv(op: &str, args: &JsonValue) -> Option<JsonValue> {
    dispatch_table(op, args, CSV_OPS)
}

fn dispatch_yaml(op: &str, args: &JsonValue) -> Option<JsonValue> {
    dispatch_table(op, args, YAML_OPS)
}

fn dispatch_table(op: &str, args: &JsonValue, ops: &[RegisteredOp]) -> Option<JsonValue> {
    ops.iter().find(|entry| entry.op == op).map(|entry| (entry.handler)(args))
}

#[derive(Debug, Clone)]
struct HttpGetRequest {
    url: String,
}

impl HttpGetRequest {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            url: arg_str(args, "url").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
struct HttpPostRequest {
    url: String,
    body: Option<JsonValue>,
}

impl HttpPostRequest {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            url: arg_str(args, "url").unwrap_or_default(),
            body: args.get("body").cloned().or_else(|| {
                args.get("__input")
                    .and_then(|v| v.as_object())
                    .and_then(|obj| obj.get("body").cloned())
            }),
        }
    }
}

#[derive(Debug, Clone)]
struct TcpConnectRequest {
    target: String,
}

impl TcpConnectRequest {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            target: arg_str_alt(args, &["target", "session"]).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
struct TcpSendRequest {
    target: String,
    data: String,
}

impl TcpSendRequest {
    fn from_args(args: &JsonValue) -> Self {
        let data = arg_str(args, "data")
            .or_else(|| arg_str(args, "text"))
            .unwrap_or_default();

        Self {
            target: arg_str_alt(args, &["target", "session"]).unwrap_or_default(),
            data,
        }
    }
}

#[derive(Debug, Clone)]
struct TcpReceiveRequest {
    target: String,
    max_bytes: usize,
}

impl TcpReceiveRequest {
    fn from_args(args: &JsonValue) -> Self {
        let max_bytes = args
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .or_else(|| {
                args.get("__input")
                    .and_then(|v| v.as_object())
                    .and_then(|obj| obj.get("max_bytes"))
                    .and_then(|v| v.as_u64())
            })
            .map(|v| v as usize)
            .unwrap_or(1024);

        Self {
            target: arg_str_alt(args, &["target", "session"]).unwrap_or_default(),
            max_bytes,
        }
    }
}

fn arg_str(args: &JsonValue, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(key))
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
        })
}

fn arg_str_alt(args: &JsonValue, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = arg_str(args, key) {
            return Some(value);
        }
    }
    None
}

fn arg_u64(args: &JsonValue, key: &str) -> Option<u64> {
    args.get(key)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(key))
                .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
        })
}

fn arg_bool(args: &JsonValue, key: &str) -> Option<bool> {
    args.get(key)
        .and_then(parse_bool_value)
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(key))
                .and_then(parse_bool_value)
        })
}

fn parse_bool_value(v: &JsonValue) -> Option<bool> {
    v.as_bool().or_else(|| {
        v.as_str()
            .and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            })
    })
}

#[derive(Debug, Clone)]
struct WebsearchSearchRequest {
    query: Option<String>,
    provider: Option<String>,
    max_results: Option<u64>,
}

impl WebsearchSearchRequest {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            query: arg_str(args, "query").or_else(|| arg_str(args, "text")),
            provider: arg_str(args, "provider"),
            max_results: arg_u64(args, "max_results"),
        }
    }

    fn to_args_json(&self) -> JsonValue {
        let mut out = json!({});
        if let Some(query) = &self.query {
            out["query"] = JsonValue::String(query.clone());
        }
        if let Some(provider) = &self.provider {
            out["provider"] = JsonValue::String(provider.clone());
        }
        if let Some(max_results) = self.max_results {
            out["max_results"] = JsonValue::Number(max_results.into());
        }
        out
    }
}

#[derive(Debug, Clone)]
struct ResearchMaterialsRequest {
    query: Option<String>,
    provider: Option<String>,
    max_results: Option<u64>,
    per_source_chars: Option<u64>,
    include_http_body: Option<bool>,
    md_options: Option<JsonValue>,
}

impl ResearchMaterialsRequest {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            query: arg_str(args, "query").or_else(|| arg_str(args, "text")),
            provider: arg_str(args, "provider"),
            max_results: arg_u64(args, "max_results"),
            per_source_chars: arg_u64(args, "per_source_chars"),
            include_http_body: arg_bool(args, "include_http_body"),
            md_options: args
                .get("md_options")
                .cloned()
                .or_else(|| {
                    args.get("__input")
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("md_options").cloned())
                }),
        }
    }

    fn to_args_json(&self) -> JsonValue {
        let mut out = json!({});
        if let Some(query) = &self.query {
            out["query"] = JsonValue::String(query.clone());
        }
        if let Some(provider) = &self.provider {
            out["provider"] = JsonValue::String(provider.clone());
        }
        if let Some(max_results) = self.max_results {
            out["max_results"] = JsonValue::Number(max_results.into());
        }
        if let Some(per_source_chars) = self.per_source_chars {
            out["per_source_chars"] = JsonValue::Number(per_source_chars.into());
        }
        if let Some(include_http_body) = self.include_http_body {
            out["include_http_body"] = JsonValue::Bool(include_http_body);
        }
        if let Some(md_options) = &self.md_options {
            out["md_options"] = md_options.clone();
        }
        out
    }
}

#[derive(Debug, Clone)]
struct ResearchReportRequest {
    query: Option<String>,
    provider: Option<String>,
    max_results: Option<u64>,
    per_source_chars: Option<u64>,
    report_chars: Option<u64>,
    include_http_body: Option<bool>,
    md_options: Option<JsonValue>,
}

impl ResearchReportRequest {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            query: arg_str(args, "query").or_else(|| arg_str(args, "text")),
            provider: arg_str(args, "provider"),
            max_results: arg_u64(args, "max_results"),
            per_source_chars: arg_u64(args, "per_source_chars"),
            report_chars: arg_u64(args, "report_chars"),
            include_http_body: arg_bool(args, "include_http_body"),
            md_options: args
                .get("md_options")
                .cloned()
                .or_else(|| {
                    args.get("__input")
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("md_options").cloned())
                }),
        }
    }

    fn to_args_json(&self) -> JsonValue {
        let mut out = json!({});
        if let Some(query) = &self.query {
            out["query"] = JsonValue::String(query.clone());
        }
        if let Some(provider) = &self.provider {
            out["provider"] = JsonValue::String(provider.clone());
        }
        if let Some(max_results) = self.max_results {
            out["max_results"] = JsonValue::Number(max_results.into());
        }
        if let Some(per_source_chars) = self.per_source_chars {
            out["per_source_chars"] = JsonValue::Number(per_source_chars.into());
        }
        if let Some(report_chars) = self.report_chars {
            out["report_chars"] = JsonValue::Number(report_chars.into());
        }
        if let Some(include_http_body) = self.include_http_body {
            out["include_http_body"] = JsonValue::Bool(include_http_body);
        }
        if let Some(md_options) = &self.md_options {
            out["md_options"] = md_options.clone();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tcp_send_request_reads_target_and_data_from_pipeline_input_object() {
        let args = json!({
            "__input": {
                "target": "127.0.0.1:9000",
                "text": "hello"
            }
        });

        let req = TcpSendRequest::from_args(&args);
        assert_eq!(req.target, "127.0.0.1:9000");
        assert_eq!(req.data, "hello");
    }

    #[test]
    fn tcp_receive_request_uses_session_fallback_and_default_max_bytes() {
        let args = json!({
            "session": "127.0.0.1:9100"
        });

        let req = TcpReceiveRequest::from_args(&args);
        assert_eq!(req.target, "127.0.0.1:9100");
        assert_eq!(req.max_bytes, 1024);
    }

    #[test]
    fn http_post_request_reads_body_from_pipeline_input() {
        let args = json!({
            "url": "https://example.com",
            "__input": {
                "body": {"message": "ok"}
            }
        });

        let req = HttpPostRequest::from_args(&args);
        assert_eq!(req.url, "https://example.com");
        assert!(req.body.is_some());
    }

    #[test]
    fn websearch_search_request_accepts_query_from_pipeline_text() {
        let args = json!({
            "__input": {
                "text": "rust async runtime"
            }
        });

        let req = WebsearchSearchRequest::from_args(&args);
        assert_eq!(req.query.as_deref(), Some("rust async runtime"));
    }

    #[test]
    fn research_materials_request_reads_bool_and_numbers_from_pipeline_input() {
        let args = json!({
            "__input": {
                "query": "typed pipelines",
                "max_results": "7",
                "per_source_chars": 1200,
                "include_http_body": "true"
            }
        });

        let req = ResearchMaterialsRequest::from_args(&args);
        assert_eq!(req.query.as_deref(), Some("typed pipelines"));
        assert_eq!(req.max_results, Some(7));
        assert_eq!(req.per_source_chars, Some(1200));
        assert_eq!(req.include_http_body, Some(true));
    }

    #[test]
    fn research_report_request_includes_report_chars_in_normalized_args() {
        let args = json!({
            "query": "normalization",
            "report_chars": "3000"
        });

        let req = ResearchReportRequest::from_args(&args);
        let normalized = req.to_args_json();
        assert_eq!(
            normalized.get("report_chars").and_then(|v| v.as_u64()),
            Some(3000)
        );
    }
}
