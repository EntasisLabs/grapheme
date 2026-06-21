//! Canonical module operation signatures used by runtime and tooling.
//!
//! This crate provides typed op metadata (args/effects/schema refs) consumed by
//! runtime manifest checks, editor tooling, and conformance tests.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    String,
    Number,
    Boolean,
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

#[derive(Debug, Clone, Copy)]
pub struct ObjectFieldSpec {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStability {
    Stable,
    Experimental,
    Deprecated,
}

impl SignatureStability {
    pub fn as_str(self) -> &'static str {
        match self {
            SignatureStability::Stable => "stable",
            SignatureStability::Experimental => "experimental",
            SignatureStability::Deprecated => "deprecated",
        }
    }
}

pub const HOST_ENVELOPE_SCHEMA: &str = "grapheme.host.result.envelope/v1";

#[derive(Debug, Clone, Copy)]
pub struct OpSpec {
    pub module: &'static str,
    pub op: &'static str,
    pub args: &'static [ArgSpec],
    pub effect: SignatureEffect,
    pub input_schema_ref: Option<&'static str>,
    pub output_schema_ref: Option<&'static str>,
}

const CORE_ECHO_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "message",
    ty: ArgType::String,
    required: false,
}];
const CORE_TAP_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "message",
    ty: ArgType::String,
    required: false,
}];
const CORE_PACK_STATE_DATA_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "state",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "data",
        ty: ArgType::Any,
        required: false,
    },
];
const CORE_GET_STATE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "input",
    ty: ArgType::Any,
    required: false,
}];
const CORE_GET_DATA_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "input",
    ty: ArgType::Any,
    required: false,
}];
const CORE_APPLY_LANE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "lane",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "fields",
        ty: ArgType::Object,
        required: true,
    },
    ArgSpec {
        name: "input",
        ty: ArgType::Object,
        required: false,
    },
];
const CORE_MAP_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "items",
        ty: ArgType::Array,
        required: false,
    },
    ArgSpec {
        name: "field",
        ty: ArgType::String,
        required: false,
    },
];
const CORE_FILTER_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "items",
        ty: ArgType::Array,
        required: false,
    },
    ArgSpec {
        name: "field",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "equals",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_FIND_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "items",
        ty: ArgType::Array,
        required: false,
    },
    ArgSpec {
        name: "field",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "equals",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_REDUCE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "items",
        ty: ArgType::Array,
        required: false,
    },
    ArgSpec {
        name: "mode",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "initial",
        ty: ArgType::Any,
        required: false,
    },
];
const CORE_GROUP_BY_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "items",
        ty: ArgType::Array,
        required: false,
    },
    ArgSpec {
        name: "field",
        ty: ArgType::String,
        required: true,
    },
];
const CORE_MERGE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "left",
        ty: ArgType::Object,
        required: false,
    },
    ArgSpec {
        name: "right",
        ty: ArgType::Object,
        required: false,
    },
];
const CORE_PICK_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "fields",
        ty: ArgType::Array,
        required: true,
    },
    ArgSpec {
        name: "input",
        ty: ArgType::Object,
        required: false,
    },
];
const CORE_VALIDATE_SCHEMA_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "required",
        ty: ArgType::Array,
        required: true,
    },
    ArgSpec {
        name: "data",
        ty: ArgType::Object,
        required: true,
    },
];
const CORE_ADD_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "a",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "b",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_SUB_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "a",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "b",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_INC_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "value",
    ty: ArgType::Any,
    required: false,
}];
const CORE_DEC_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "value",
    ty: ArgType::Any,
    required: false,
}];
const CORE_EQ_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "a",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "b",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_LT_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "a",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "b",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_GT_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "a",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "b",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_GTE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "a",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "b",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_LTE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "a",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "b",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_INC_FIELD_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "field",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "input",
        ty: ArgType::Object,
        required: false,
    },
];
const CORE_DEC_FIELD_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "field",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "input",
        ty: ArgType::Object,
        required: false,
    },
];
const CORE_SET_FIELDS_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "fields",
        ty: ArgType::Object,
        required: true,
    },
    ArgSpec {
        name: "input",
        ty: ArgType::Object,
        required: false,
    },
];
const CORE_SPLIT_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "text",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "sep",
        ty: ArgType::String,
        required: false,
    },
];
const CORE_JOIN_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "items",
        ty: ArgType::Array,
        required: false,
    },
    ArgSpec {
        name: "sep",
        ty: ArgType::String,
        required: false,
    },
];
const CORE_REPLACE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "text",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "from",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "to",
        ty: ArgType::String,
        required: false,
    },
];
const CORE_TRIM_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "text",
    ty: ArgType::String,
    required: false,
}];
const CORE_LOWER_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "text",
    ty: ArgType::String,
    required: false,
}];
const CORE_UPPER_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "text",
    ty: ArgType::String,
    required: false,
}];
const CORE_CONTAINS_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "haystack",
        ty: ArgType::Any,
        required: false,
    },
    ArgSpec {
        name: "needle",
        ty: ArgType::Any,
        required: true,
    },
];
const CORE_GET_PATH_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "path",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "input",
        ty: ArgType::Any,
        required: false,
    },
];
const CORE_SET_PATH_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "path",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "value",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "input",
        ty: ArgType::Object,
        required: false,
    },
];
const CORE_HAS_PATH_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "path",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "input",
        ty: ArgType::Any,
        required: false,
    },
];

const IO_READ_TEXT_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "path",
    ty: ArgType::String,
    required: true,
}];
const IO_WRITE_TEXT_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "path",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "text",
        ty: ArgType::String,
        required: true,
    },
];
const IO_LIST_DIR_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "path",
    ty: ArgType::String,
    required: false,
}];

const HTTP_GET_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "url",
    ty: ArgType::String,
    required: true,
}];
const HTTP_POST_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "url",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "body",
        ty: ArgType::Any,
        required: false,
    },
];
const WEBSEARCH_SEARCH_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "query",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "provider",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "max_results",
        ty: ArgType::Number,
        required: false,
    },
];
const WEB_PROVIDER_SEARCH_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "query",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "max_results",
        ty: ArgType::Number,
        required: false,
    },
];
const WEB_CAPABILITIES_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "provider",
    ty: ArgType::String,
    required: false,
}];
const WEBSEARCH_RESEARCH_REPORT_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "query",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "provider",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "max_results",
        ty: ArgType::Number,
        required: false,
    },
    ArgSpec {
        name: "per_source_chars",
        ty: ArgType::Number,
        required: false,
    },
    ArgSpec {
        name: "report_chars",
        ty: ArgType::Number,
        required: false,
    },
    ArgSpec {
        name: "md_options",
        ty: ArgType::Object,
        required: false,
    },
];
const WEBSEARCH_RESEARCH_MATERIALS_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "query",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "provider",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "max_results",
        ty: ArgType::Number,
        required: false,
    },
    ArgSpec {
        name: "per_source_chars",
        ty: ArgType::Number,
        required: false,
    },
    ArgSpec {
        name: "md_options",
        ty: ArgType::Object,
        required: false,
    },
    ArgSpec {
        name: "include_http_body",
        ty: ArgType::Boolean,
        required: false,
    },
];

const TCP_CONNECT_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "target",
    ty: ArgType::String,
    required: true,
}];
const TCP_SEND_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "session",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "target",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "data",
        ty: ArgType::String,
        required: false,
    },
];
const TCP_RECEIVE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "session",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "target",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "max_bytes",
        ty: ArgType::Number,
        required: false,
    },
];

const SMTP_SEND_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "to",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "from",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "server",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "subject",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "body",
        ty: ArgType::String,
        required: false,
    },
];
const EMAIL_SEND_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "to",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "from",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "subject",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "body",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "host",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "server",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "port",
        ty: ArgType::Number,
        required: false,
    },
    ArgSpec {
        name: "username",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "password",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "security",
        ty: ArgType::String,
        required: false,
    },
];
const EMAIL_CAPABILITIES_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "provider",
    ty: ArgType::String,
    required: false,
}];

const SECRETS_GET_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "name",
    ty: ArgType::String,
    required: true,
}];
const SECRETS_SIGN_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "secret",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "payload",
        ty: ArgType::Any,
        required: false,
    },
];

const MEMORY_STORE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "key",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "value",
        ty: ArgType::Any,
        required: false,
    },
];
const MEMORY_LOAD_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "key",
    ty: ArgType::String,
    required: false,
}];

const DOCS_GUIDE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "topic",
    ty: ArgType::String,
    required: false,
}];
const DOCS_EXAMPLE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "module",
    ty: ArgType::String,
    required: false,
}];
const HTML_TO_MD_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "html",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "options",
        ty: ArgType::Object,
        required: false,
    },
];
const HTML_CLEAN_TEXT_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "text",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "max_chars",
        ty: ArgType::Number,
        required: false,
    },
];
const JSON_PARSE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "text",
    ty: ArgType::String,
    required: false,
}];
const CSV_TO_LIST_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "text",
    ty: ArgType::String,
    required: false,
}];
const YAML_TO_JSON_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "text",
    ty: ArgType::String,
    required: false,
}];
const DB_QUERY_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "sql",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "params",
        ty: ArgType::Any,
        required: false,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const DB_EXECUTE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "sql",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "params",
        ty: ArgType::Any,
        required: false,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const DB_TRANSACTION_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "steps",
        ty: ArgType::Array,
        required: true,
    },
    ArgSpec {
        name: "isolation",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const DB_HEALTH_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const SURREAL_QUERY_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "query",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "vars",
        ty: ArgType::Object,
        required: false,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const SURREAL_SELECT_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "thing_or_table",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "where",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "limit",
        ty: ArgType::Number,
        required: false,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const SURREAL_CREATE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "thing_or_table",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "data",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const SURREAL_UPDATE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "thing_or_table",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "data",
        ty: ArgType::Any,
        required: true,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const SURREAL_DELETE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "thing_or_table",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];
const SURREAL_HEALTH_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "connection",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "timeout_ms",
        ty: ArgType::Number,
        required: false,
    },
];

const DATA_READ_CSV_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "path",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "max_rows",
        ty: ArgType::Number,
        required: false,
    },
];
const DATA_FRAME_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "frame",
    ty: ArgType::Object,
    required: true,
}];
const DATA_FILTER_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "frame",
        ty: ArgType::Object,
        required: true,
    },
    ArgSpec {
        name: "column",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "eq",
        ty: ArgType::Any,
        required: true,
    },
];
const DATA_AGGREGATE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "frame",
        ty: ArgType::Object,
        required: true,
    },
    ArgSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
];
const DATA_GROUP_BY_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "frame",
        ty: ArgType::Object,
        required: true,
    },
    ArgSpec {
        name: "by",
        ty: ArgType::String,
        required: true,
    },
];
const PDF_GENERATE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "title",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "body",
        ty: ArgType::String,
        required: false,
    },
];
const PDF_EXTRACT_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "path",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "bytes",
        ty: ArgType::String,
        required: false,
    },
];
const IMAGE_RESIZE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "width",
        ty: ArgType::Number,
        required: true,
    },
    ArgSpec {
        name: "height",
        ty: ArgType::Number,
        required: true,
    },
];
const IMAGE_CONVERT_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "format",
    ty: ArgType::String,
    required: true,
}];
const IMAGE_METADATA_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "path",
        ty: ArgType::String,
        required: false,
    },
    ArgSpec {
        name: "bytes",
        ty: ArgType::String,
        required: false,
    },
];
const PLOT_SERIES_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "series",
    ty: ArgType::Array,
    required: true,
}];
const PLOT_SCATTER_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "points",
    ty: ArgType::Array,
    required: true,
}];
const MEDIA_PROBE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "path",
    ty: ArgType::String,
    required: true,
}];
const MEDIA_TRANSCODE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "input",
        ty: ArgType::String,
        required: true,
    },
    ArgSpec {
        name: "output",
        ty: ArgType::String,
        required: true,
    },
];

pub const OP_SPECS: &[OpSpec] = &[
    OpSpec {
        module: "core",
        op: "echo",
        args: CORE_ECHO_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "tap",
        args: CORE_TAP_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "pack_state_data",
        args: CORE_PACK_STATE_DATA_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "get_state",
        args: CORE_GET_STATE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "get_data",
        args: CORE_GET_DATA_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "apply_lane",
        args: CORE_APPLY_LANE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "map",
        args: CORE_MAP_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "filter",
        args: CORE_FILTER_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "find",
        args: CORE_FIND_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "reduce",
        args: CORE_REDUCE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "group_by",
        args: CORE_GROUP_BY_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "merge",
        args: CORE_MERGE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "pick",
        args: CORE_PICK_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "validate_schema",
        args: CORE_VALIDATE_SCHEMA_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "add",
        args: CORE_ADD_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "sub",
        args: CORE_SUB_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "inc",
        args: CORE_INC_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "dec",
        args: CORE_DEC_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "eq",
        args: CORE_EQ_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "lt",
        args: CORE_LT_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "gt",
        args: CORE_GT_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "gte",
        args: CORE_GTE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "lte",
        args: CORE_LTE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "inc_field",
        args: CORE_INC_FIELD_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "dec_field",
        args: CORE_DEC_FIELD_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "set_fields",
        args: CORE_SET_FIELDS_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "split",
        args: CORE_SPLIT_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "join",
        args: CORE_JOIN_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "replace",
        args: CORE_REPLACE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "trim",
        args: CORE_TRIM_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "lower",
        args: CORE_LOWER_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "upper",
        args: CORE_UPPER_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "contains",
        args: CORE_CONTAINS_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "get_path",
        args: CORE_GET_PATH_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "set_path",
        args: CORE_SET_PATH_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "core",
        op: "has_path",
        args: CORE_HAS_PATH_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "io",
        op: "read_text",
        args: IO_READ_TEXT_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "io",
        op: "write_text",
        args: IO_WRITE_TEXT_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "io",
        op: "list_dir",
        args: IO_LIST_DIR_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "http",
        op: "get",
        args: HTTP_GET_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "http",
        op: "post",
        args: HTTP_POST_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "web",
        op: "duckduckgo",
        args: WEB_PROVIDER_SEARCH_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "web",
        op: "google",
        args: WEB_PROVIDER_SEARCH_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "web",
        op: "xaviv",
        args: WEB_PROVIDER_SEARCH_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "web",
        op: "tavily",
        args: WEB_PROVIDER_SEARCH_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "web",
        op: "brave",
        args: WEB_PROVIDER_SEARCH_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "web",
        op: "providers",
        args: &[],
        effect: SignatureEffect::Control,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "web",
        op: "capabilities",
        args: WEB_CAPABILITIES_ARGS,
        effect: SignatureEffect::Control,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "websearch",
        op: "search",
        args: WEBSEARCH_SEARCH_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "websearch",
        op: "research_materials",
        args: WEBSEARCH_RESEARCH_MATERIALS_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "websearch",
        op: "research_report",
        args: WEBSEARCH_RESEARCH_REPORT_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "tcp",
        op: "connect",
        args: TCP_CONNECT_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "tcp",
        op: "send",
        args: TCP_SEND_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "tcp",
        op: "receive",
        args: TCP_RECEIVE_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "smtp",
        op: "send_mail",
        args: SMTP_SEND_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "email",
        op: "smtp",
        args: EMAIL_SEND_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "email",
        op: "gmail",
        args: EMAIL_SEND_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "email",
        op: "providers",
        args: &[],
        effect: SignatureEffect::Control,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "email",
        op: "capabilities",
        args: EMAIL_CAPABILITIES_ARGS,
        effect: SignatureEffect::Control,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "secrets",
        op: "get_secret_handle",
        args: SECRETS_GET_ARGS,
        effect: SignatureEffect::Secrets,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "secrets",
        op: "sign_request",
        args: SECRETS_SIGN_ARGS,
        effect: SignatureEffect::Secrets,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "memory",
        op: "store_context",
        args: MEMORY_STORE_ARGS,
        effect: SignatureEffect::State,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "memory",
        op: "load_context",
        args: MEMORY_LOAD_ARGS,
        effect: SignatureEffect::State,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "memory",
        op: "summarize_context",
        args: &[],
        effect: SignatureEffect::State,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "docs",
        op: "native_module_guide",
        args: DOCS_GUIDE_ARGS,
        effect: SignatureEffect::Control,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "docs",
        op: "native_module_registry",
        args: &[],
        effect: SignatureEffect::Control,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "docs",
        op: "native_module_example",
        args: DOCS_EXAMPLE_ARGS,
        effect: SignatureEffect::Control,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "html",
        op: "to_md",
        args: HTML_TO_MD_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "html",
        op: "clean_text",
        args: HTML_CLEAN_TEXT_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "json",
        op: "parse",
        args: JSON_PARSE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "csv",
        op: "to_list",
        args: CSV_TO_LIST_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "yaml",
        op: "to_json",
        args: YAML_TO_JSON_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "sql",
        op: "query",
        args: DB_QUERY_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "sql",
        op: "execute",
        args: DB_EXECUTE_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "sql",
        op: "transaction",
        args: DB_TRANSACTION_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "sql",
        op: "health",
        args: DB_HEALTH_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "surreal",
        op: "query",
        args: SURREAL_QUERY_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "surreal",
        op: "select",
        args: SURREAL_SELECT_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "surreal",
        op: "create",
        args: SURREAL_CREATE_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "surreal",
        op: "update",
        args: SURREAL_UPDATE_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "surreal",
        op: "delete",
        args: SURREAL_DELETE_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    OpSpec {
        module: "surreal",
        op: "health",
        args: SURREAL_HEALTH_ARGS,
        effect: SignatureEffect::Network,
        input_schema_ref: None,
        output_schema_ref: None,
    },
    #[cfg(feature = "data")]
    OpSpec {
        module: "data",
        op: "read_csv",
        args: DATA_READ_CSV_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "data")]
    OpSpec {
        module: "data",
        op: "filter",
        args: DATA_FILTER_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "data")]
    OpSpec {
        module: "data",
        op: "group_by",
        args: DATA_GROUP_BY_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "data")]
    OpSpec {
        module: "data",
        op: "aggregate",
        args: DATA_AGGREGATE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "data")]
    OpSpec {
        module: "data",
        op: "to_json",
        args: DATA_FRAME_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "data")]
    OpSpec {
        module: "data",
        op: "schema",
        args: DATA_FRAME_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "pdf")]
    OpSpec {
        module: "pdf",
        op: "generate",
        args: PDF_GENERATE_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "pdf")]
    OpSpec {
        module: "pdf",
        op: "extract_text",
        args: PDF_EXTRACT_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "image")]
    OpSpec {
        module: "image",
        op: "resize",
        args: IMAGE_RESIZE_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "image")]
    OpSpec {
        module: "image",
        op: "convert",
        args: IMAGE_CONVERT_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "image")]
    OpSpec {
        module: "image",
        op: "metadata",
        args: IMAGE_METADATA_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "plot")]
    OpSpec {
        module: "plot",
        op: "line",
        args: PLOT_SERIES_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "plot")]
    OpSpec {
        module: "plot",
        op: "bar",
        args: PLOT_SERIES_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "plot")]
    OpSpec {
        module: "plot",
        op: "scatter",
        args: PLOT_SCATTER_ARGS,
        effect: SignatureEffect::Pure,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "media")]
    OpSpec {
        module: "media",
        op: "probe",
        args: MEDIA_PROBE_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
    #[cfg(feature = "media")]
    OpSpec {
        module: "media",
        op: "transcode",
        args: MEDIA_TRANSCODE_ARGS,
        effect: SignatureEffect::Io,
        input_schema_ref: None,
        output_schema_ref: Some("grapheme.host.result.envelope/v1"),
    },
];

pub fn op_specs() -> &'static [OpSpec] {
    OP_SPECS
}

pub fn find_op_spec(module: &str, op: &str) -> Option<&'static OpSpec> {
    OP_SPECS
        .iter()
        .find(|spec| spec.module == module && spec.op == op)
}

pub fn module_ops(module: &str) -> Vec<&'static OpSpec> {
    OP_SPECS
        .iter()
        .filter(|spec| spec.module.eq_ignore_ascii_case(module))
        .collect()
}

pub fn op_stability(module: &str, op: &str) -> SignatureStability {
    match (module, op) {
        // Provider is still exploratory and should remain explicitly tagged.
        ("web", "xaviv") => SignatureStability::Experimental,
        // Docs helper ops are internal-facing and may evolve quickly.
        ("docs", "native_module_guide")
        | ("docs", "native_module_registry")
        | ("docs", "native_module_example") => SignatureStability::Experimental,
        ("data", _)
        | ("pdf", _)
        | ("image", _)
        | ("plot", _)
        | ("media", _) => SignatureStability::Experimental,
        _ => SignatureStability::Stable,
    }
}

pub fn op_stability_label(module: &str, op: &str) -> &'static str {
    op_stability(module, op).as_str()
}

pub fn op_uses_host_envelope(module: &str, op: &str) -> bool {
    find_op_spec(module, op)
        .and_then(|spec| spec.output_schema_ref)
        .is_some_and(|schema| schema == HOST_ENVELOPE_SCHEMA)
}

pub fn host_envelope_output_fields() -> &'static [ObjectFieldSpec] {
    HOST_ENVELOPE_OUTPUT_FIELDS
}

pub fn op_output_type(module: &str, op: &str) -> ArgType {
    match (module, op) {
        ("core", "tap")
        | ("core", "get_state")
        | ("core", "get_data")
        | ("core", "find")
        | ("core", "reduce")
        | ("core", "get_path") => ArgType::Any,
        ("core", "map") | ("core", "filter") | ("core", "split") => ArgType::Array,
        ("core", "add") | ("core", "sub") | ("core", "inc") | ("core", "dec") => ArgType::Number,
        ("core", "echo")
        | ("core", "pack_state_data")
        | ("core", "apply_lane")
        | ("core", "group_by")
        | ("core", "merge")
        | ("core", "pick")
        | ("core", "validate_schema")
        | ("core", "eq")
        | ("core", "lt")
        | ("core", "gt")
        | ("core", "gte")
        | ("core", "lte")
        | ("core", "inc_field")
        | ("core", "dec_field")
        | ("core", "set_fields")
        | ("core", "join")
        | ("core", "replace")
        | ("core", "trim")
        | ("core", "lower")
        | ("core", "upper")
        | ("core", "contains")
        | ("core", "set_path")
        | ("core", "has_path") => ArgType::Object,
        ("io", "list_dir") => ArgType::Array,
        ("io", "read_text") | ("io", "write_text") => ArgType::Object,
        ("http", "get") | ("http", "post") => ArgType::Object,
        ("web", _) | ("websearch", _) => ArgType::Object,
        ("tcp", _) | ("smtp", _) | ("email", _) => ArgType::Object,
        ("sql", _) | ("surreal", _) => ArgType::Object,
        ("secrets", _) | ("memory", _) | ("docs", _) => ArgType::Object,
        ("html", _) => ArgType::Object,
        ("json", "parse") | ("yaml", "to_json") => ArgType::Any,
        ("csv", "to_list") => ArgType::Array,
        ("data", _) | ("pdf", _) | ("image", _) | ("plot", _) | ("media", _) => ArgType::Object,
        ("runtime", _) | ("policy", _) => ArgType::Object,
        _ => ArgType::Any,
    }
}

const CORE_MESSAGE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[ObjectFieldSpec {
    name: "message",
    ty: ArgType::String,
    required: true,
}];

const CORE_STATE_DATA_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "state",
        ty: ArgType::Any,
        required: true,
    },
    ObjectFieldSpec {
        name: "data",
        ty: ArgType::Any,
        required: true,
    },
];

const CORE_BOOL_VALUE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[ObjectFieldSpec {
    name: "value",
    ty: ArgType::Boolean,
    required: true,
}];

const CORE_TEXT_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[ObjectFieldSpec {
    name: "text",
    ty: ArgType::String,
    required: true,
}];

const CORE_VALIDATE_SCHEMA_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "missing",
        ty: ArgType::Array,
        required: true,
    },
];

const CORE_CONTAINS_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[ObjectFieldSpec {
    name: "contains",
    ty: ArgType::Boolean,
    required: true,
}];

const CORE_HAS_PATH_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[ObjectFieldSpec {
    name: "has_path",
    ty: ArgType::Boolean,
    required: true,
}];

const DYNAMIC_OBJECT_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[];

const WEB_PROVIDER_SEARCH_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "count",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "results",
        ty: ArgType::Array,
        required: true,
    },
    ObjectFieldSpec {
        name: "provider",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "query",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const WEB_PROVIDERS_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "count",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "providers",
        ty: ArgType::Array,
        required: true,
    },
];

const WEB_CAPABILITIES_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "provider",
        ty: ArgType::Object,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "available_providers",
        ty: ArgType::Array,
        required: false,
    },
];

const WEBSEARCH_MATERIALS_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "query",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "provider",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "count",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "sources",
        ty: ArgType::Array,
        required: true,
    },
];

const WEBSEARCH_REPORT_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "query",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "provider",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "count",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "sources",
        ty: ArgType::Array,
        required: true,
    },
    ObjectFieldSpec {
        name: "report",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "materials",
        ty: ArgType::Object,
        required: true,
    },
];

const HTTP_RESPONSE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "method",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "url",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "status",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "status_line",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "body",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const TCP_CONNECT_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "connected",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "target",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "session",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const TCP_SEND_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "sent",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "target",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "bytes",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const TCP_RECEIVE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "target",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "bytes",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "data",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const SMTP_SEND_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "accepted",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "server",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "from",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "to",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "subject",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const SQL_QUERY_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "connection",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "row_count",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "rows",
        ty: ArgType::Array,
        required: false,
    },
    ObjectFieldSpec {
        name: "elapsed_ms",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::Object,
        required: false,
    },
];

const SQL_EXECUTE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "connection",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "rows_affected",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "elapsed_ms",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::Object,
        required: false,
    },
];

const SQL_HEALTH_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "connection",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "latency_ms",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::Object,
        required: false,
    },
];

const SQL_TRANSACTION_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "connection",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "committed",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "results",
        ty: ArgType::Array,
        required: false,
    },
    ObjectFieldSpec {
        name: "elapsed_ms",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::Object,
        required: false,
    },
];

const SURREAL_QUERY_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "connection",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "query",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "result",
        ty: ArgType::Array,
        required: false,
    },
    ObjectFieldSpec {
        name: "elapsed_ms",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::Object,
        required: false,
    },
];

const SURREAL_HEALTH_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "connection",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "latency_ms",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::Object,
        required: false,
    },
];

const HTML_TO_MD_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "text",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "markdown",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "result",
        ty: ArgType::Object,
        required: false,
    },
    ObjectFieldSpec {
        name: "used_options",
        ty: ArgType::Object,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const HTML_CLEAN_TEXT_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "text",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "length",
        ty: ArgType::Number,
        required: true,
    },
];

const IO_READ_TEXT_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "path",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "text",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const IO_WRITE_TEXT_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "path",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "bytes",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const SECRETS_GET_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "handle",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "name",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const SECRETS_SIGN_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "signature",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "algorithm",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const MEMORY_STORE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "key",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "stored",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const MEMORY_LOAD_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "key",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "value",
        ty: ArgType::Any,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const MEMORY_SUMMARIZE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "summary",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "count",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const DOCS_GUIDE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "topic",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "content",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const DOCS_REGISTRY_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "modules",
        ty: ArgType::Array,
        required: false,
    },
    ObjectFieldSpec {
        name: "count",
        ty: ArgType::Number,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const DOCS_EXAMPLE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "module",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "examples",
        ty: ArgType::Array,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const RUNTIME_POLICY_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "allowed",
        ty: ArgType::Boolean,
        required: false,
    },
    ObjectFieldSpec {
        name: "reason",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const HOST_ENVELOPE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "data",
        ty: ArgType::Object,
        required: true,
    },
    ObjectFieldSpec {
        name: "meta",
        ty: ArgType::Object,
        required: true,
    },
    ObjectFieldSpec {
        name: "error",
        ty: ArgType::String,
        required: false,
    },
];

const DATA_READ_CSV_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "frame",
        ty: ArgType::Object,
        required: true,
    },
];

const DATA_FILTER_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "frame",
        ty: ArgType::Object,
        required: true,
    },
];

const DATA_GROUP_BY_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "group",
        ty: ArgType::Object,
        required: true,
    },
];

const DATA_AGGREGATE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "result",
        ty: ArgType::Object,
        required: true,
    },
];

const DATA_SCHEMA_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "schema",
        ty: ArgType::Object,
        required: true,
    },
];

const DATA_TO_JSON_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "rows",
        ty: ArgType::Array,
        required: true,
    },
];

const PDF_GENERATE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "title",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "body",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "page_count",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "format",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "bytes_base64",
        ty: ArgType::String,
        required: true,
    },
];

const PDF_EXTRACT_TEXT_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "path",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "text",
        ty: ArgType::String,
        required: true,
    },
];

const IMAGE_METADATA_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "width",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "height",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "color_type",
        ty: ArgType::String,
        required: true,
    },
];

const IMAGE_BYTES_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "width",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "height",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "format",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "bytes_base64",
        ty: ArgType::String,
        required: true,
    },
];

const PLOT_CHART_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "format",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "content",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "bytes_base64",
        ty: ArgType::String,
        required: true,
    },
];

const MEDIA_PROBE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "path",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "stream_count",
        ty: ArgType::Number,
        required: true,
    },
    ObjectFieldSpec {
        name: "format_name",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "duration",
        ty: ArgType::String,
        required: false,
    },
    ObjectFieldSpec {
        name: "probe",
        ty: ArgType::Object,
        required: true,
    },
];

const MEDIA_TRANSCODE_OUTPUT_FIELDS: &[ObjectFieldSpec] = &[
    ObjectFieldSpec {
        name: "ok",
        ty: ArgType::Boolean,
        required: true,
    },
    ObjectFieldSpec {
        name: "op",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "input",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "output",
        ty: ArgType::String,
        required: true,
    },
    ObjectFieldSpec {
        name: "summary",
        ty: ArgType::Object,
        required: true,
    },
];

pub fn op_output_object_fields(module: &str, op: &str) -> Option<&'static [ObjectFieldSpec]> {
    match (module, op) {
        ("core", "echo") => Some(CORE_MESSAGE_OUTPUT_FIELDS),
        ("core", "pack_state_data") => Some(CORE_STATE_DATA_OUTPUT_FIELDS),
        ("core", "eq") | ("core", "lt") | ("core", "gt") | ("core", "gte") | ("core", "lte") => {
            Some(CORE_BOOL_VALUE_OUTPUT_FIELDS)
        }
        ("core", "join")
        | ("core", "replace")
        | ("core", "trim")
        | ("core", "lower")
        | ("core", "upper") => Some(CORE_TEXT_OUTPUT_FIELDS),
        ("core", "validate_schema") => Some(CORE_VALIDATE_SCHEMA_OUTPUT_FIELDS),
        ("core", "contains") => Some(CORE_CONTAINS_OUTPUT_FIELDS),
        ("core", "has_path") => Some(CORE_HAS_PATH_OUTPUT_FIELDS),
        ("core", "apply_lane")
        | ("core", "group_by")
        | ("core", "merge")
        | ("core", "pick")
        | ("core", "inc_field")
        | ("core", "dec_field")
        | ("core", "set_fields")
        | ("core", "set_path") => Some(DYNAMIC_OBJECT_OUTPUT_FIELDS),

        ("web", "duckduckgo")
        | ("web", "google")
        | ("web", "xaviv")
        | ("web", "tavily")
        | ("web", "brave") => Some(WEB_PROVIDER_SEARCH_OUTPUT_FIELDS),
        ("web", "providers") => Some(WEB_PROVIDERS_OUTPUT_FIELDS),
        ("web", "capabilities") => Some(WEB_CAPABILITIES_OUTPUT_FIELDS),
        ("websearch", "search") => Some(WEB_PROVIDER_SEARCH_OUTPUT_FIELDS),
        ("websearch", "research_materials") => Some(WEBSEARCH_MATERIALS_OUTPUT_FIELDS),
        ("websearch", "research_report") => Some(WEBSEARCH_REPORT_OUTPUT_FIELDS),

        ("http", "get") | ("http", "post") => Some(HTTP_RESPONSE_OUTPUT_FIELDS),

        ("tcp", "connect") => Some(TCP_CONNECT_OUTPUT_FIELDS),
        ("tcp", "send") => Some(TCP_SEND_OUTPUT_FIELDS),
        ("tcp", "receive") => Some(TCP_RECEIVE_OUTPUT_FIELDS),
        ("smtp", "send_mail") | ("email", "smtp") | ("email", "gmail") => {
            Some(SMTP_SEND_OUTPUT_FIELDS)
        }
        ("email", "providers") => Some(WEB_PROVIDERS_OUTPUT_FIELDS),
        ("email", "capabilities") => Some(WEB_CAPABILITIES_OUTPUT_FIELDS),

        ("sql", "query") => Some(SQL_QUERY_OUTPUT_FIELDS),
        ("sql", "execute") => Some(SQL_EXECUTE_OUTPUT_FIELDS),
        ("sql", "health") => Some(SQL_HEALTH_OUTPUT_FIELDS),
        ("sql", "transaction") => Some(SQL_TRANSACTION_OUTPUT_FIELDS),

        ("surreal", "query")
        | ("surreal", "select")
        | ("surreal", "create")
        | ("surreal", "update")
        | ("surreal", "delete") => Some(SURREAL_QUERY_OUTPUT_FIELDS),
        ("surreal", "health") => Some(SURREAL_HEALTH_OUTPUT_FIELDS),

        ("html", "to_md") => Some(HTML_TO_MD_OUTPUT_FIELDS),
        ("html", "clean_text") => Some(HTML_CLEAN_TEXT_OUTPUT_FIELDS),

        ("io", "read_text") => Some(IO_READ_TEXT_OUTPUT_FIELDS),
        ("io", "write_text") => Some(IO_WRITE_TEXT_OUTPUT_FIELDS),

        ("secrets", "get_secret_handle") => Some(SECRETS_GET_OUTPUT_FIELDS),
        ("secrets", "sign_request") => Some(SECRETS_SIGN_OUTPUT_FIELDS),

        ("memory", "store_context") => Some(MEMORY_STORE_OUTPUT_FIELDS),
        ("memory", "load_context") => Some(MEMORY_LOAD_OUTPUT_FIELDS),
        ("memory", "summarize_context") => Some(MEMORY_SUMMARIZE_OUTPUT_FIELDS),

        ("docs", "native_module_guide") => Some(DOCS_GUIDE_OUTPUT_FIELDS),
        ("docs", "native_module_registry") => Some(DOCS_REGISTRY_OUTPUT_FIELDS),
        ("docs", "native_module_example") => Some(DOCS_EXAMPLE_OUTPUT_FIELDS),

        ("runtime", _) | ("policy", _) => Some(RUNTIME_POLICY_OUTPUT_FIELDS),

        ("data", "read_csv") => Some(DATA_READ_CSV_OUTPUT_FIELDS),
        ("data", "filter") => Some(DATA_FILTER_OUTPUT_FIELDS),
        ("data", "group_by") => Some(DATA_GROUP_BY_OUTPUT_FIELDS),
        ("data", "aggregate") => Some(DATA_AGGREGATE_OUTPUT_FIELDS),
        ("data", "schema") => Some(DATA_SCHEMA_OUTPUT_FIELDS),
        ("data", "to_json") => Some(DATA_TO_JSON_OUTPUT_FIELDS),

        ("pdf", "generate") => Some(PDF_GENERATE_OUTPUT_FIELDS),
        ("pdf", "extract_text") => Some(PDF_EXTRACT_TEXT_OUTPUT_FIELDS),

        ("image", "metadata") => Some(IMAGE_METADATA_OUTPUT_FIELDS),
        ("image", "resize") | ("image", "convert") => Some(IMAGE_BYTES_OUTPUT_FIELDS),

        ("plot", "line") | ("plot", "bar") | ("plot", "scatter") => Some(PLOT_CHART_OUTPUT_FIELDS),

        ("media", "probe") => Some(MEDIA_PROBE_OUTPUT_FIELDS),
        ("media", "transcode") => Some(MEDIA_TRANSCODE_OUTPUT_FIELDS),

        _ => None,
    }
}
