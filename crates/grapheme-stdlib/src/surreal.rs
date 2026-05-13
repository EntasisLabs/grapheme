use serde_json::{json, Value as JsonValue};
use std::env;
use std::time::{Duration, Instant};
use surrealdb::engine::remote::http::Http;
use surrealdb::opt::Config;
use surrealdb::Surreal;

pub fn query(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };
    let query_text = match required_string(args, "query") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_query", &e),
    };

    let vars = args
        .get("vars")
        .cloned()
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("vars").cloned())
        })
        .filter(|v| v.is_object());

    run_query(args, &connection, &query_text, vars)
}

pub fn select(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };
    let thing_or_table = match required_string(args, "thing_or_table") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_thing_or_table", &e),
    };

    let query_text = build_select_query(
        &thing_or_table,
        optional_string(args, "where").as_deref(),
        optional_u64(args, "limit"),
    );

    run_query(args, &connection, &query_text, None)
}

pub fn health(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };

    let out = run_query(args, &connection, "return true;", None);
    if out.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        return json!({
            "ok": true,
            "connection": connection,
            "latency_ms": out.get("elapsed_ms").cloned().unwrap_or(JsonValue::from(0u64)),
        });
    }

    out
}

pub fn create(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };
    let thing_or_table = match required_string(args, "thing_or_table") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_thing_or_table", &e),
    };
    let data = match required_value(args, "data") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_data", &e),
    };

    let query_text = build_create_query(&thing_or_table);
    run_query(args, &connection, &query_text, Some(json!({ "data": data })))
}

pub fn update(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };
    let thing_or_table = match required_string(args, "thing_or_table") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_thing_or_table", &e),
    };
    let data = match required_value(args, "data") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_data", &e),
    };

    let query_text = build_update_query(&thing_or_table);
    run_query(args, &connection, &query_text, Some(json!({ "data": data })))
}

pub fn delete(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };
    let thing_or_table = match required_string(args, "thing_or_table") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_thing_or_table", &e),
    };

    let query_text = build_delete_query(&thing_or_table);
    run_query(args, &connection, &query_text, None)
}

fn run_query(args: &JsonValue, connection: &str, query_text: &str, vars: Option<JsonValue>) -> JsonValue {
    let resolved = match resolve_connection(args, connection) {
        Ok(v) => v,
        Err(e) => return error_payload("connection_error", "surreal_connection_unresolved", &e),
    };

    let started = Instant::now();
    let timeout = Duration::from_millis(optional_u64(args, "timeout_ms").unwrap_or(10_000));
    let max_payload_bytes = optional_u64(args, "max_payload_bytes");
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            return error_payload(
                "runtime_error",
                "tokio_runtime_init_failed",
                &e.to_string(),
            )
        }
    };

    let result: Result<JsonValue, JsonValue> = runtime.block_on(async {
        let config = Config::new().query_timeout(Some(timeout));
        let endpoint = base_endpoint(&resolved.url);
        let db = Surreal::new::<Http>((endpoint.as_str(), config))
            .await
            .map_err(|e| {
                error_payload(
                    "connection_error",
                    "surreal_connect_failed",
                    &format!("connect failed: {e}"),
                )
            })?;

        if let Some(token) = resolved.token {
            db.authenticate(token).await.map_err(|e| {
                error_payload(
                    "connection_error",
                    "surreal_auth_failed",
                    &format!("authenticate failed: {e}"),
                )
            })?;
        }

        if let (Some(ns), Some(db_name)) = (resolved.namespace.as_ref(), resolved.database.as_ref()) {
            db.use_ns(ns).use_db(db_name).await.map_err(|e| {
                error_payload(
                    "connection_error",
                    "surreal_scope_failed",
                    &format!("failed to select namespace/database: {e}"),
                )
            })?;
        }

        let mut query = db.query(query_text);
        if let Some(bind_vars) = vars {
            query = query.bind(bind_vars);
        }

        let mut response = query.await.map_err(|e| {
            error_payload(
                "query_error",
                "surreal_query_failed",
                &format!("query failed: {e}"),
            )
        })?;

        response = response.check().map_err(|e| {
            error_payload(
                "query_error",
                "surreal_query_failed",
                &e.to_string(),
            )
        })?;

        let mut statements = Vec::with_capacity(response.num_statements());
        for idx in 0..response.num_statements() {
            let statement: surrealdb::types::Value = response.take(idx).map_err(|e| {
                error_payload(
                    "query_error",
                    "surreal_result_decode_failed",
                    &format!("failed to decode statement {idx}: {e}"),
                )
            })?;
            let statement_json = serde_json::to_value(statement).map_err(|e| {
                error_payload(
                    "query_error",
                    "surreal_result_decode_failed",
                    &format!("failed to serialize statement {idx}: {e}"),
                )
            })?;
            statements.push(statement_json);
        }

        if let Err(message) = ensure_payload_within_limit(&statements, max_payload_bytes) {
            return Err(error_payload("query_error", "surreal_query_failed", &message));
        }

        Ok(json!({
            "ok": true,
            "connection": connection,
            "query": query_text,
            "result": statements,
            "elapsed_ms": started.elapsed().as_millis() as u64,
        }))
    });

    match result {
        Ok(v) => v,
        Err(err_payload) => err_payload,
    }
}

fn build_select_query(thing_or_table: &str, where_clause: Option<&str>, limit: Option<u64>) -> String {
    let mut q = format!("select * from {}", thing_or_table);
    if let Some(where_clause) = where_clause {
        if !where_clause.trim().is_empty() {
            q.push_str(" where ");
            q.push_str(where_clause);
        }
    }
    if let Some(limit) = limit {
        q.push_str(&format!(" limit {}", limit));
    }
    q.push(';');
    q
}

fn build_create_query(thing_or_table: &str) -> String {
    format!("create {} content $data;", thing_or_table)
}

fn build_update_query(thing_or_table: &str) -> String {
    format!("update {} content $data;", thing_or_table)
}

fn build_delete_query(thing_or_table: &str) -> String {
    format!("delete {};", thing_or_table)
}

#[cfg(test)]
fn detect_surreal_error_message(value: &JsonValue) -> Option<String> {
    let rows = value.as_array()?;
    for row in rows {
        let status = row.get("status").and_then(|v| v.as_str()).unwrap_or_default();
        if status.eq_ignore_ascii_case("ERR") {
            let msg = row
                .get("result")
                .map(|v| {
                    if let Some(s) = v.as_str() {
                        s.to_string()
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_else(|| "surreal query failed".to_string());
            return Some(msg);
        }
    }
    None
}

fn ensure_payload_within_limit(
    statements: &[JsonValue],
    max_payload_bytes: Option<u64>,
) -> Result<(), String> {
    let Some(max_payload_bytes) = max_payload_bytes else {
        return Ok(());
    };

    let payload_bytes = serde_json::to_vec(statements)
        .map(|v| v.len())
        .map_err(|e| format!("serialize surreal result failed: {e}"))?;

    if payload_bytes > max_payload_bytes as usize {
        return Err(format!(
            "payload size {} exceeds max_payload_bytes {}",
            payload_bytes,
            max_payload_bytes
        ));
    }

    Ok(())
}

fn required_string(args: &JsonValue, key: &str) -> Result<String, String> {
    optional_string(args, key).ok_or_else(|| format!("missing required '{}'", key))
}

fn required_value(args: &JsonValue, key: &str) -> Result<JsonValue, String> {
    args.get(key)
        .cloned()
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(key).cloned())
        })
        .ok_or_else(|| format!("missing required '{}'", key))
}

fn optional_string(args: &JsonValue, key: &str) -> Option<String> {
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

fn optional_u64(args: &JsonValue, key: &str) -> Option<u64> {
    args.get(key)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(key))
                .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
        })
}

#[derive(Debug, Clone)]
struct ResolvedSurrealConnection {
    url: String,
    namespace: Option<String>,
    database: Option<String>,
    token: Option<String>,
}

fn resolve_connection(args: &JsonValue, connection: &str) -> Result<ResolvedSurrealConnection, String> {
    let mut resolved = if connection.starts_with("http://") || connection.starts_with("https://") {
        ResolvedSurrealConnection {
            url: connection.to_string(),
            namespace: None,
            database: None,
            token: None,
        }
    } else {
        resolve_connection_id(connection)?
    };

    if let Some(ns) = optional_string(args, "ns") {
        resolved.namespace = Some(ns);
    }
    if let Some(db) = optional_string(args, "db") {
        resolved.database = Some(db);
    }
    if let Some(token) = optional_string(args, "token") {
        resolved.token = Some(token);
    }

    Ok(resolved)
}

fn resolve_connection_id(connection: &str) -> Result<ResolvedSurrealConnection, String> {
    let env_key = format!(
        "GRAPHEME_SURREAL_CONNECTION_{}",
        connection
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
            .collect::<String>()
    );

    if let Ok(value) = env::var(&env_key) {
        if let Some(parsed) = parse_connection_value(&value) {
            return Ok(parsed);
        }
    }

    if let Ok(raw) = env::var("GRAPHEME_SURREAL_CONNECTIONS") {
        if let Ok(map) = serde_json::from_str::<JsonValue>(&raw) {
            if let Some(entry) = map.get(connection) {
                if let Some(parsed) = parse_connection_json(entry) {
                    return Ok(parsed);
                }
            }
        }
    }

    Err(format!(
        "connection '{}' is unresolved; set {} or GRAPHEME_SURREAL_CONNECTIONS",
        connection, env_key
    ))
}

fn parse_connection_value(value: &str) -> Option<ResolvedSurrealConnection> {
    if let Ok(parsed) = serde_json::from_str::<JsonValue>(value) {
        return parse_connection_json(&parsed);
    }

    if value.starts_with("http://") || value.starts_with("https://") {
        return Some(ResolvedSurrealConnection {
            url: value.to_string(),
            namespace: None,
            database: None,
            token: None,
        });
    }

    None
}

fn parse_connection_json(value: &JsonValue) -> Option<ResolvedSurrealConnection> {
    if let Some(url) = value.as_str() {
        if url.starts_with("http://") || url.starts_with("https://") {
            return Some(ResolvedSurrealConnection {
                url: url.to_string(),
                namespace: None,
                database: None,
                token: None,
            });
        }
        return None;
    }

    let obj = value.as_object()?;
    let url = obj.get("url").and_then(|v| v.as_str())?.to_string();

    Some(ResolvedSurrealConnection {
        url,
        namespace: obj.get("ns").and_then(|v| v.as_str()).map(ToOwned::to_owned),
        database: obj.get("db").and_then(|v| v.as_str()).map(ToOwned::to_owned),
        token: obj.get("token").and_then(|v| v.as_str()).map(ToOwned::to_owned),
    })
}

fn base_endpoint(base: &str) -> String {
    base.trim_end_matches('/').trim_end_matches("/sql").to_string()
}

fn error_payload(kind: &str, code: &str, message: &str) -> JsonValue {
    json!({
        "ok": false,
        "error": {
            "kind": kind,
            "code": code,
            "message": message,
            "retryable": false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_query_builder_includes_where_and_limit() {
        let q = build_select_query("person", Some("active = true"), Some(5));
        assert_eq!(q, "select * from person where active = true limit 5;");
    }

    #[test]
    fn select_query_builder_handles_minimal_form() {
        let q = build_select_query("person", None, None);
        assert_eq!(q, "select * from person;");
    }

    #[test]
    fn create_query_builder_uses_content_binding() {
        let q = build_create_query("person:demo");
        assert_eq!(q, "create person:demo content $data;");
    }

    #[test]
    fn update_query_builder_uses_content_binding() {
        let q = build_update_query("person:demo");
        assert_eq!(q, "update person:demo content $data;");
    }

    #[test]
    fn delete_query_builder_targets_resource() {
        let q = build_delete_query("person:demo");
        assert_eq!(q, "delete person:demo;");
    }

    #[test]
    fn query_reports_unresolved_connection_id() {
        let out = query(&json!({
            "connection": "missing_surreal_conn",
            "query": "select * from person;"
        }));

        assert_eq!(
            out.get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("surreal_connection_unresolved")
        );
    }

    #[test]
    fn create_requires_data() {
        let out = create(&json!({
            "connection": "local",
            "thing_or_table": "person:demo"
        }));

        assert_eq!(
            out.get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("missing_data")
        );
    }

    #[test]
    fn create_reports_unresolved_connection_id() {
        let out = create(&json!({
            "connection": "missing_surreal_conn",
            "thing_or_table": "person:demo",
            "data": {"name": "Ada"}
        }));

        assert_eq!(
            out.get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("surreal_connection_unresolved")
        );
    }

    #[test]
    fn detect_surreal_error_message_extracts_err_row() {
        let payload = json!([
            {"status": "OK", "result": [{"id": 1}]},
            {"status": "ERR", "result": "syntax error"}
        ]);
        assert_eq!(
            detect_surreal_error_message(&payload).as_deref(),
            Some("syntax error")
        );
    }

    #[test]
    fn payload_limit_rejects_oversized_result() {
        let statements = vec![json!({"payload": "abcdefghijklmnopqrstuvwxyz"})];

        let err = ensure_payload_within_limit(&statements, Some(8))
            .expect_err("payload cap should reject oversized result");

        assert!(err.contains("exceeds max_payload_bytes"));
    }

    #[test]
    fn payload_limit_allows_large_result_within_boundary() {
        let statements = (0..128)
            .map(|i| json!({"idx": i, "payload": "aaaaaaaaaaaaaaaa"}))
            .collect::<Vec<_>>();

        ensure_payload_within_limit(&statements, Some(16_384))
            .expect("payload should be accepted within configured byte cap");
    }

    #[test]
    fn payload_limit_rejects_large_result_when_boundary_tight() {
        let statements = (0..128)
            .map(|i| json!({"idx": i, "payload": "aaaaaaaaaaaaaaaa"}))
            .collect::<Vec<_>>();

        let err = ensure_payload_within_limit(&statements, Some(512))
            .expect_err("payload should be rejected by tight byte cap");
        assert!(err.contains("exceeds max_payload_bytes"));
    }
}
