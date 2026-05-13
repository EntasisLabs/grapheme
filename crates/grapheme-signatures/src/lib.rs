//! Canonical module operation signatures used by runtime and tooling.
//!
//! This crate provides typed op metadata (args/effects/schema refs) consumed by
//! runtime manifest checks, editor tooling, and conformance tests.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    String,
    Object,
    Array,
    Any,
}

#[derive(Debug, Clone, Copy)]
pub struct ArgSpec {
    pub name: &'static str,
    pub ty: ArgType,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureEffect {
    Pure,
    Network,
    Io,
    State,
    Secrets,
    Control,
}

#[derive(Debug, Clone, Copy)]
pub struct OpSpec {
    pub module: &'static str,
    pub op: &'static str,
    pub args: &'static [ArgSpec],
    pub effect: SignatureEffect,
    pub input_schema_ref: Option<&'static str>,
    pub output_schema_ref: Option<&'static str>,
}

const CORE_ECHO_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "message", ty: ArgType::String, required: false },
];
const CORE_TAP_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "message", ty: ArgType::String, required: false },
];
const CORE_PACK_STATE_DATA_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "state", ty: ArgType::Any, required: true },
    ArgSpec { name: "data", ty: ArgType::Any, required: false },
];
const CORE_GET_STATE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "input", ty: ArgType::Any, required: false },
];
const CORE_GET_DATA_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "input", ty: ArgType::Any, required: false },
];
const CORE_APPLY_LANE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "lane", ty: ArgType::String, required: true },
    ArgSpec { name: "fields", ty: ArgType::Object, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_MAP_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "items", ty: ArgType::Array, required: false },
    ArgSpec { name: "field", ty: ArgType::String, required: false },
];
const CORE_FILTER_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "items", ty: ArgType::Array, required: false },
    ArgSpec { name: "field", ty: ArgType::String, required: true },
    ArgSpec { name: "equals", ty: ArgType::Any, required: true },
];
const CORE_FIND_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "items", ty: ArgType::Array, required: false },
    ArgSpec { name: "field", ty: ArgType::String, required: true },
    ArgSpec { name: "equals", ty: ArgType::Any, required: true },
];
const CORE_REDUCE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "items", ty: ArgType::Array, required: false },
    ArgSpec { name: "mode", ty: ArgType::String, required: true },
    ArgSpec { name: "initial", ty: ArgType::Any, required: false },
];
const CORE_GROUP_BY_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "items", ty: ArgType::Array, required: false },
    ArgSpec { name: "field", ty: ArgType::String, required: true },
];
const CORE_MERGE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "left", ty: ArgType::Object, required: false },
    ArgSpec { name: "right", ty: ArgType::Object, required: false },
];
const CORE_PICK_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "fields", ty: ArgType::Array, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_VALIDATE_SCHEMA_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "required", ty: ArgType::Array, required: true },
    ArgSpec { name: "data", ty: ArgType::Object, required: true },
];
const CORE_ADD_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_SUB_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_INC_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "value", ty: ArgType::Any, required: false },
];
const CORE_DEC_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "value", ty: ArgType::Any, required: false },
];
const CORE_EQ_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_LT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_GT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_GTE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_LTE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "a", ty: ArgType::Any, required: true },
    ArgSpec { name: "b", ty: ArgType::Any, required: true },
];
const CORE_INC_FIELD_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "field", ty: ArgType::String, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_DEC_FIELD_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "field", ty: ArgType::String, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_SET_FIELDS_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "fields", ty: ArgType::Object, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_SPLIT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "text", ty: ArgType::String, required: false },
    ArgSpec { name: "sep", ty: ArgType::String, required: false },
];
const CORE_JOIN_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "items", ty: ArgType::Array, required: false },
    ArgSpec { name: "sep", ty: ArgType::String, required: false },
];
const CORE_REPLACE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "text", ty: ArgType::String, required: false },
    ArgSpec { name: "from", ty: ArgType::String, required: true },
    ArgSpec { name: "to", ty: ArgType::String, required: false },
];
const CORE_TRIM_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "text", ty: ArgType::String, required: false },
];
const CORE_LOWER_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "text", ty: ArgType::String, required: false },
];
const CORE_UPPER_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "text", ty: ArgType::String, required: false },
];
const CORE_CONTAINS_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "haystack", ty: ArgType::Any, required: false },
    ArgSpec { name: "needle", ty: ArgType::Any, required: true },
];
const CORE_GET_PATH_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "path", ty: ArgType::String, required: true },
    ArgSpec { name: "input", ty: ArgType::Any, required: false },
];
const CORE_SET_PATH_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "path", ty: ArgType::String, required: true },
    ArgSpec { name: "value", ty: ArgType::Any, required: true },
    ArgSpec { name: "input", ty: ArgType::Object, required: false },
];
const CORE_HAS_PATH_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "path", ty: ArgType::String, required: true },
    ArgSpec { name: "input", ty: ArgType::Any, required: false },
];

const IO_READ_TEXT_ARGS: &[ArgSpec] = &[ArgSpec { name: "path", ty: ArgType::String, required: true }];
const IO_WRITE_TEXT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "path", ty: ArgType::String, required: true },
    ArgSpec { name: "text", ty: ArgType::String, required: true },
];
const IO_LIST_DIR_ARGS: &[ArgSpec] = &[ArgSpec { name: "path", ty: ArgType::String, required: false }];

const HTTP_GET_ARGS: &[ArgSpec] = &[ArgSpec { name: "url", ty: ArgType::String, required: true }];
const HTTP_POST_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "url", ty: ArgType::String, required: true },
    ArgSpec { name: "body", ty: ArgType::Any, required: false },
];
const WEBSEARCH_SEARCH_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "query", ty: ArgType::String, required: true },
    ArgSpec { name: "provider", ty: ArgType::String, required: false },
    ArgSpec { name: "max_results", ty: ArgType::Any, required: false },
];
const WEB_PROVIDER_SEARCH_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "query", ty: ArgType::String, required: true },
    ArgSpec { name: "max_results", ty: ArgType::Any, required: false },
];
const WEB_CAPABILITIES_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "provider", ty: ArgType::String, required: false },
];
const WEBSEARCH_RESEARCH_REPORT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "query", ty: ArgType::String, required: true },
    ArgSpec { name: "provider", ty: ArgType::String, required: false },
    ArgSpec { name: "max_results", ty: ArgType::Any, required: false },
    ArgSpec { name: "per_source_chars", ty: ArgType::Any, required: false },
    ArgSpec { name: "report_chars", ty: ArgType::Any, required: false },
    ArgSpec { name: "md_options", ty: ArgType::Object, required: false },
];
const WEBSEARCH_RESEARCH_MATERIALS_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "query", ty: ArgType::String, required: true },
    ArgSpec { name: "provider", ty: ArgType::String, required: false },
    ArgSpec { name: "max_results", ty: ArgType::Any, required: false },
    ArgSpec { name: "per_source_chars", ty: ArgType::Any, required: false },
    ArgSpec { name: "md_options", ty: ArgType::Object, required: false },
    ArgSpec { name: "include_http_body", ty: ArgType::Any, required: false },
];

const TCP_CONNECT_ARGS: &[ArgSpec] = &[ArgSpec { name: "target", ty: ArgType::String, required: true }];
const TCP_SEND_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "session", ty: ArgType::String, required: false },
    ArgSpec { name: "target", ty: ArgType::String, required: false },
    ArgSpec { name: "data", ty: ArgType::String, required: false },
];
const TCP_RECEIVE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "session", ty: ArgType::String, required: false },
    ArgSpec { name: "target", ty: ArgType::String, required: false },
    ArgSpec { name: "max_bytes", ty: ArgType::Any, required: false },
];

const SMTP_SEND_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "to", ty: ArgType::String, required: true },
    ArgSpec { name: "from", ty: ArgType::String, required: false },
    ArgSpec { name: "server", ty: ArgType::String, required: false },
    ArgSpec { name: "subject", ty: ArgType::String, required: false },
    ArgSpec { name: "body", ty: ArgType::String, required: false },
];

const SECRETS_GET_ARGS: &[ArgSpec] = &[ArgSpec { name: "name", ty: ArgType::String, required: true }];
const SECRETS_SIGN_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "secret", ty: ArgType::String, required: true },
    ArgSpec { name: "payload", ty: ArgType::Any, required: false },
];

const MEMORY_STORE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "key", ty: ArgType::String, required: false },
    ArgSpec { name: "value", ty: ArgType::Any, required: false },
];
const MEMORY_LOAD_ARGS: &[ArgSpec] = &[ArgSpec { name: "key", ty: ArgType::String, required: false }];

const DOCS_GUIDE_ARGS: &[ArgSpec] = &[ArgSpec { name: "topic", ty: ArgType::String, required: false }];
const DOCS_EXAMPLE_ARGS: &[ArgSpec] = &[ArgSpec { name: "module", ty: ArgType::String, required: false }];
const HTML_TO_MD_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "html", ty: ArgType::String, required: false },
    ArgSpec { name: "options", ty: ArgType::Object, required: false },
];
const HTML_CLEAN_TEXT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "text", ty: ArgType::String, required: false },
    ArgSpec { name: "max_chars", ty: ArgType::Any, required: false },
];
const JSON_PARSE_ARGS: &[ArgSpec] = &[ArgSpec { name: "text", ty: ArgType::String, required: false }];
const CSV_TO_LIST_ARGS: &[ArgSpec] = &[ArgSpec { name: "text", ty: ArgType::String, required: false }];
const YAML_TO_JSON_ARGS: &[ArgSpec] = &[ArgSpec { name: "text", ty: ArgType::String, required: false }];
const DB_QUERY_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "sql", ty: ArgType::String, required: true },
    ArgSpec { name: "params", ty: ArgType::Any, required: false },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const DB_EXECUTE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "sql", ty: ArgType::String, required: true },
    ArgSpec { name: "params", ty: ArgType::Any, required: false },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const DB_TRANSACTION_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "steps", ty: ArgType::Array, required: true },
    ArgSpec { name: "isolation", ty: ArgType::String, required: false },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const DB_HEALTH_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const SURREAL_QUERY_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "query", ty: ArgType::String, required: true },
    ArgSpec { name: "vars", ty: ArgType::Object, required: false },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const SURREAL_SELECT_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "thing_or_table", ty: ArgType::String, required: true },
    ArgSpec { name: "where", ty: ArgType::String, required: false },
    ArgSpec { name: "limit", ty: ArgType::Any, required: false },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const SURREAL_CREATE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "thing_or_table", ty: ArgType::String, required: true },
    ArgSpec { name: "data", ty: ArgType::Any, required: true },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const SURREAL_UPDATE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "thing_or_table", ty: ArgType::String, required: true },
    ArgSpec { name: "data", ty: ArgType::Any, required: true },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const SURREAL_DELETE_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "thing_or_table", ty: ArgType::String, required: true },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];
const SURREAL_HEALTH_ARGS: &[ArgSpec] = &[
    ArgSpec { name: "connection", ty: ArgType::String, required: true },
    ArgSpec { name: "timeout_ms", ty: ArgType::Any, required: false },
];

pub const OP_SPECS: &[OpSpec] = &[
    OpSpec { module: "core", op: "echo", args: CORE_ECHO_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "tap", args: CORE_TAP_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "pack_state_data", args: CORE_PACK_STATE_DATA_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "get_state", args: CORE_GET_STATE_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "get_data", args: CORE_GET_DATA_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "apply_lane", args: CORE_APPLY_LANE_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "map", args: CORE_MAP_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "filter", args: CORE_FILTER_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "find", args: CORE_FIND_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "reduce", args: CORE_REDUCE_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "group_by", args: CORE_GROUP_BY_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "merge", args: CORE_MERGE_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "pick", args: CORE_PICK_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "validate_schema", args: CORE_VALIDATE_SCHEMA_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "add", args: CORE_ADD_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "sub", args: CORE_SUB_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "inc", args: CORE_INC_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "dec", args: CORE_DEC_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "eq", args: CORE_EQ_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "lt", args: CORE_LT_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "gt", args: CORE_GT_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "gte", args: CORE_GTE_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "lte", args: CORE_LTE_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "inc_field", args: CORE_INC_FIELD_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "dec_field", args: CORE_DEC_FIELD_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "set_fields", args: CORE_SET_FIELDS_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "split", args: CORE_SPLIT_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "join", args: CORE_JOIN_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "replace", args: CORE_REPLACE_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "trim", args: CORE_TRIM_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "lower", args: CORE_LOWER_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "upper", args: CORE_UPPER_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "contains", args: CORE_CONTAINS_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "get_path", args: CORE_GET_PATH_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "set_path", args: CORE_SET_PATH_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "core", op: "has_path", args: CORE_HAS_PATH_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "io", op: "read_text", args: IO_READ_TEXT_ARGS, effect: SignatureEffect::Io, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "io", op: "write_text", args: IO_WRITE_TEXT_ARGS, effect: SignatureEffect::Io, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "io", op: "list_dir", args: IO_LIST_DIR_ARGS, effect: SignatureEffect::Io, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "http", op: "get", args: HTTP_GET_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "http", op: "post", args: HTTP_POST_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "web", op: "duckduckgo", args: WEB_PROVIDER_SEARCH_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "web", op: "google", args: WEB_PROVIDER_SEARCH_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "web", op: "xaviv", args: WEB_PROVIDER_SEARCH_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "web", op: "providers", args: &[], effect: SignatureEffect::Control, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "web", op: "capabilities", args: WEB_CAPABILITIES_ARGS, effect: SignatureEffect::Control, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "websearch", op: "search", args: WEBSEARCH_SEARCH_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "websearch", op: "research_materials", args: WEBSEARCH_RESEARCH_MATERIALS_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "websearch", op: "research_report", args: WEBSEARCH_RESEARCH_REPORT_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "tcp", op: "connect", args: TCP_CONNECT_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "tcp", op: "send", args: TCP_SEND_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "tcp", op: "receive", args: TCP_RECEIVE_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "smtp", op: "send_mail", args: SMTP_SEND_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "secrets", op: "get_secret_handle", args: SECRETS_GET_ARGS, effect: SignatureEffect::Secrets, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "secrets", op: "sign_request", args: SECRETS_SIGN_ARGS, effect: SignatureEffect::Secrets, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "memory", op: "store_context", args: MEMORY_STORE_ARGS, effect: SignatureEffect::State, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "memory", op: "load_context", args: MEMORY_LOAD_ARGS, effect: SignatureEffect::State, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "memory", op: "summarize_context", args: &[], effect: SignatureEffect::State, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "docs", op: "native_module_guide", args: DOCS_GUIDE_ARGS, effect: SignatureEffect::Control, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "docs", op: "native_module_registry", args: &[], effect: SignatureEffect::Control, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "docs", op: "native_module_example", args: DOCS_EXAMPLE_ARGS, effect: SignatureEffect::Control, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "html", op: "to_md", args: HTML_TO_MD_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "html", op: "clean_text", args: HTML_CLEAN_TEXT_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "json", op: "parse", args: JSON_PARSE_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "csv", op: "to_list", args: CSV_TO_LIST_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "yaml", op: "to_json", args: YAML_TO_JSON_ARGS, effect: SignatureEffect::Pure, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "sql", op: "query", args: DB_QUERY_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "sql", op: "execute", args: DB_EXECUTE_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "sql", op: "transaction", args: DB_TRANSACTION_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "sql", op: "health", args: DB_HEALTH_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "surreal", op: "query", args: SURREAL_QUERY_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "surreal", op: "select", args: SURREAL_SELECT_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "surreal", op: "create", args: SURREAL_CREATE_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "surreal", op: "update", args: SURREAL_UPDATE_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "surreal", op: "delete", args: SURREAL_DELETE_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
    OpSpec { module: "surreal", op: "health", args: SURREAL_HEALTH_ARGS, effect: SignatureEffect::Network, input_schema_ref: None, output_schema_ref: None },
];

pub fn op_specs() -> &'static [OpSpec] {
    OP_SPECS
}

pub fn find_op_spec(module: &str, op: &str) -> Option<&'static OpSpec> {
    OP_SPECS.iter().find(|spec| spec.module == module && spec.op == op)
}

pub fn module_ops(module: &str) -> Vec<&'static OpSpec> {
    OP_SPECS
        .iter()
        .filter(|spec| spec.module.eq_ignore_ascii_case(module))
        .collect()
}
