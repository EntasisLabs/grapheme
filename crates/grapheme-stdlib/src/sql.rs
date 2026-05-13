use serde_json::{json, Map, Value as JsonValue};
use sqlx::{AnyPool, Column, Row};
use std::env;
use std::sync::Once;
use std::time::Instant;

static SQLX_ANY_DRIVERS: Once = Once::new();

pub fn query(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };
    let sql = match required_string(args, "sql") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_sql", &e),
    };

    let params = match optional_params(args) {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "sql_params_invalid", &e),
    };
    let max_rows = optional_u64(args, "max_rows");
    let max_payload_bytes = optional_u64(args, "max_payload_bytes");

    let resolved = match resolve_connection(&connection) {
        Ok(v) => v,
        Err(e) => return error_payload("connection_error", "sql_connection_unresolved", &e),
    };

    let started = Instant::now();
    let rt = match tokio::runtime::Builder::new_current_thread()
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

    let result: Result<JsonValue, String> = rt.block_on(async {
        ensure_any_drivers();
        let pool = AnyPool::connect(&resolved)
            .await
            .map_err(|e| format!("connect failed: {e}"))?;

        let query = match bind_params(sqlx::query(&sql), &params) {
            Ok(query) => query,
            Err(e) => return Err(e),
        };

        let rows = query
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("query failed: {e}"))?;

        if let Some(max_rows) = max_rows {
            if rows.len() > max_rows as usize {
                return Err(format!(
                    "row count {} exceeds max_rows {}",
                    rows.len(),
                    max_rows
                ));
            }
        }

        let mut out_rows = Vec::with_capacity(rows.len());
        for row in rows {
            let mut obj = Map::new();
            for (idx, col) in row.columns().iter().enumerate() {
                let key = col.name().to_string();
                let value = decode_row_value(&row, idx);
                obj.insert(key, value);
            }
            out_rows.push(JsonValue::Object(obj));
        }

        if let Some(max_payload_bytes) = max_payload_bytes {
            let payload_bytes = serde_json::to_vec(&out_rows)
                .map(|v| v.len())
                .map_err(|e| format!("serialize query rows failed: {e}"))?;
            if payload_bytes > max_payload_bytes as usize {
                return Err(format!(
                    "payload size {} exceeds max_payload_bytes {}",
                    payload_bytes,
                    max_payload_bytes
                ));
            }
        }

        Ok(json!({
            "ok": true,
            "connection": connection,
            "row_count": out_rows.len(),
            "rows": out_rows,
            "elapsed_ms": started.elapsed().as_millis() as u64,
        }))
    });

    match result {
        Ok(v) => v,
        Err(e) => error_payload("query_error", "sql_query_failed", &e),
    }
}

pub fn execute(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };
    let sql = match required_string(args, "sql") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_sql", &e),
    };

    let params = match optional_params(args) {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "sql_params_invalid", &e),
    };

    let resolved = match resolve_connection(&connection) {
        Ok(v) => v,
        Err(e) => return error_payload("connection_error", "sql_connection_unresolved", &e),
    };

    let started = Instant::now();
    let rt = match tokio::runtime::Builder::new_current_thread()
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

    let result: Result<JsonValue, String> = rt.block_on(async {
        ensure_any_drivers();
        let pool = AnyPool::connect(&resolved)
            .await
            .map_err(|e| format!("connect failed: {e}"))?;

        let query = match bind_params(sqlx::query(&sql), &params) {
            Ok(query) => query,
            Err(e) => return Err(e),
        };

        let outcome = query
            .execute(&pool)
            .await
            .map_err(|e| format!("execute failed: {e}"))?;

        Ok(json!({
            "ok": true,
            "connection": connection,
            "rows_affected": outcome.rows_affected(),
            "elapsed_ms": started.elapsed().as_millis() as u64,
        }))
    });

    match result {
        Ok(v) => v,
        Err(e) => error_payload("query_error", "sql_execute_failed", &e),
    }
}

pub fn health(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };

    let resolved = match resolve_connection(&connection) {
        Ok(v) => v,
        Err(e) => return error_payload("connection_error", "sql_connection_unresolved", &e),
    };

    let started = Instant::now();
    let rt = match tokio::runtime::Builder::new_current_thread()
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

    let result: Result<JsonValue, String> = rt.block_on(async {
        ensure_any_drivers();
        let pool = AnyPool::connect(&resolved)
            .await
            .map_err(|e| format!("connect failed: {e}"))?;

        sqlx::query("select 1")
            .execute(&pool)
            .await
            .map_err(|e| format!("health check failed: {e}"))?;

        Ok(json!({
            "ok": true,
            "connection": connection,
            "latency_ms": started.elapsed().as_millis() as u64,
        }))
    });

    match result {
        Ok(v) => v,
        Err(e) => error_payload("connection_error", "sql_health_failed", &e),
    }
}

pub fn transaction(args: &JsonValue) -> JsonValue {
    let connection = match required_string(args, "connection") {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "missing_connection", &e),
    };

    let steps = match parse_transaction_steps(args) {
        Ok(v) => v,
        Err(e) => return error_payload("validation_error", "sql_transaction_invalid", &e),
    };

    let resolved = match resolve_connection(&connection) {
        Ok(v) => v,
        Err(e) => return error_payload("connection_error", "sql_connection_unresolved", &e),
    };

    let started = Instant::now();
    let rt = match tokio::runtime::Builder::new_current_thread()
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

    let result: Result<JsonValue, JsonValue> = rt.block_on(async {
        ensure_any_drivers();
        let pool = AnyPool::connect(&resolved)
            .await
            .map_err(|e| error_payload("connection_error", "sql_connect_failed", &format!("connect failed: {e}")))?;

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| error_payload("query_error", "sql_transaction_begin_failed", &format!("begin failed: {e}")))?;

        let mut results = Vec::with_capacity(steps.len());

        for (idx, step) in steps.iter().enumerate() {
            let query = bind_params(sqlx::query(&step.sql), &step.params)
                .map_err(|e| transaction_failure_payload(&connection, &results, idx, started, "sql_params_invalid", &e))?;

            if step.mode == TransactionStepMode::Query {
                let rows = query
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|e| transaction_failure_payload(&connection, &results, idx, started, "sql_transaction_step_failed", &format!("query step failed: {e}")))?;

                let mut out_rows = Vec::with_capacity(rows.len());
                for row in rows {
                    let mut obj = Map::new();
                    for (col_idx, col) in row.columns().iter().enumerate() {
                        obj.insert(col.name().to_string(), decode_row_value(&row, col_idx));
                    }
                    out_rows.push(JsonValue::Object(obj));
                }

                results.push(json!({
                    "mode": "query",
                    "row_count": out_rows.len(),
                    "rows": out_rows,
                }));
            } else {
                let outcome = query
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| transaction_failure_payload(&connection, &results, idx, started, "sql_transaction_step_failed", &format!("execute step failed: {e}")))?;

                results.push(json!({
                    "mode": "execute",
                    "rows_affected": outcome.rows_affected(),
                }));
            }
        }

        tx.commit()
            .await
            .map_err(|e| error_payload("query_error", "sql_transaction_commit_failed", &format!("commit failed: {e}")))?;

        Ok(json!({
            "ok": true,
            "connection": connection,
            "committed": true,
            "results": results,
            "elapsed_ms": started.elapsed().as_millis() as u64,
        }))
    });

    match result {
        Ok(v) => v,
        Err(v) => v,
    }
}

fn required_string(args: &JsonValue, key: &str) -> Result<String, String> {
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
        .ok_or_else(|| format!("missing required '{}'", key))
}

fn optional_params(args: &JsonValue) -> Result<Vec<JsonValue>, String> {
    let candidate = args
        .get("params")
        .cloned()
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("params").cloned())
        });

    match candidate {
        None => Ok(Vec::new()),
        Some(JsonValue::Array(items)) => Ok(items),
        Some(_) => Err("params must be an array".to_string()),
    }
}

fn optional_u64(args: &JsonValue, key: &str) -> Option<u64> {
    args.get(key)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(key))
                .and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                })
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionStepMode {
    Query,
    Execute,
}

#[derive(Debug, Clone)]
struct TransactionStep {
    mode: TransactionStepMode,
    sql: String,
    params: Vec<JsonValue>,
}

fn parse_transaction_steps(args: &JsonValue) -> Result<Vec<TransactionStep>, String> {
    let raw_steps = args
        .get("steps")
        .cloned()
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("steps").cloned())
        })
        .ok_or_else(|| "missing required 'steps'".to_string())?;

    let list = raw_steps
        .as_array()
        .ok_or_else(|| "steps must be an array".to_string())?;

    if list.is_empty() {
        return Err("steps must contain at least one entry".to_string());
    }

    let mut out = Vec::with_capacity(list.len());
    for (idx, value) in list.iter().enumerate() {
        let obj = value
            .as_object()
            .ok_or_else(|| format!("step[{idx}] must be an object"))?;

        let sql = obj
            .get("sql")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
            .ok_or_else(|| format!("step[{idx}] missing required 'sql'"))?;

        let mode = match obj.get("mode").and_then(|v| v.as_str()) {
            Some("query") => TransactionStepMode::Query,
            Some("execute") | None => TransactionStepMode::Execute,
            Some(other) => {
                return Err(format!(
                    "step[{idx}] has invalid mode '{}', expected query|execute",
                    other
                ))
            }
        };

        let params = match obj.get("params") {
            None => Vec::new(),
            Some(JsonValue::Array(items)) => items.clone(),
            Some(_) => return Err(format!("step[{idx}] params must be an array")),
        };

        out.push(TransactionStep { mode, sql, params });
    }

    Ok(out)
}

fn bind_params<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>,
    params: &[JsonValue],
) -> Result<sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>, String> {
    for value in params {
        query = match value {
            JsonValue::Null => query.bind(Option::<String>::None),
            JsonValue::Bool(v) => query.bind(*v),
            JsonValue::Number(n) => {
                if let Some(v) = n.as_i64() {
                    query.bind(v)
                } else if let Some(v) = n.as_u64() {
                    if let Ok(as_i64) = i64::try_from(v) {
                        query.bind(as_i64)
                    } else {
                        query.bind(v as f64)
                    }
                } else if let Some(v) = n.as_f64() {
                    query.bind(v)
                } else {
                    return Err("unsupported numeric param representation".to_string());
                }
            }
            JsonValue::String(v) => query.bind(v.clone()),
            JsonValue::Array(_) | JsonValue::Object(_) => {
                return Err("only scalar params are supported (null/bool/number/string)".to_string())
            }
        };
    }

    Ok(query)
}

fn resolve_connection(connection: &str) -> Result<String, String> {
    if connection.contains("://") || connection.starts_with("sqlite:") {
        return Ok(connection.to_string());
    }

    let env_key = format!(
        "GRAPHEME_SQL_CONNECTION_{}",
        connection
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
            .collect::<String>()
    );

    if let Ok(url) = env::var(&env_key) {
        if !url.trim().is_empty() {
            return Ok(url);
        }
    }

    if let Ok(map_raw) = env::var("GRAPHEME_SQL_CONNECTIONS") {
        if let Ok(map_json) = serde_json::from_str::<JsonValue>(&map_raw) {
            if let Some(url) = map_json
                .get(connection)
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
            {
                return Ok(url);
            }
        }
    }

    Err(format!(
        "connection '{}' is unresolved; set {} or GRAPHEME_SQL_CONNECTIONS",
        connection, env_key
    ))
}

fn decode_row_value(row: &sqlx::any::AnyRow, idx: usize) -> JsonValue {
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map(JsonValue::from).unwrap_or(JsonValue::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v.map(JsonValue::from).unwrap_or(JsonValue::Null);
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return v.map(JsonValue::from).unwrap_or(JsonValue::Null);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(JsonValue::from).unwrap_or(JsonValue::Null);
    }

    JsonValue::Null
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

fn transaction_failure_payload(
    connection: &str,
    results: &[JsonValue],
    failed_step: usize,
    started: Instant,
    code: &str,
    message: &str,
) -> JsonValue {
    json!({
        "ok": false,
        "connection": connection,
        "committed": false,
        "failed_step": failed_step,
        "results": results,
        "elapsed_ms": started.elapsed().as_millis() as u64,
        "error": {
            "kind": "query_error",
            "code": code,
            "message": message,
            "retryable": false,
        }
    })
}

fn ensure_any_drivers() {
    SQLX_ANY_DRIVERS.call_once(sqlx::any::install_default_drivers);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sqlite_temp_connection(tag: &str) -> (String, std::path::PathBuf) {
        let mut path = std::env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        path.push(format!("grapheme-sql-{tag}-{ts}.db"));
        (format!("sqlite://{}?mode=rwc", path.display()), path)
    }

    #[test]
    fn health_accepts_direct_sqlite_url_connection() {
        let out = health(&json!({ "connection": "sqlite::memory:" }));
        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn query_returns_rows_for_basic_select() {
        let out = query(&json!({
            "connection": "sqlite::memory:",
            "sql": "select 1 as ok"
        }));
        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(out.get("row_count").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn execute_reports_rows_affected() {
        let out = execute(&json!({
            "connection": "sqlite::memory:",
            "sql": "create table if not exists t (id integer)"
        }));
        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert!(out.get("rows_affected").and_then(|v| v.as_u64()).is_some());
    }

    #[test]
    fn query_reports_unresolved_connection_id() {
        let out = query(&json!({
            "connection": "missing_conn",
            "sql": "select 1"
        }));
        assert_eq!(
            out.get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("sql_connection_unresolved")
        );
    }

    #[test]
    fn query_supports_scalar_positional_params() {
        let out = query(&json!({
            "connection": "sqlite::memory:",
            "sql": "select ?1 as n, ?2 as t, ?3 as b, ?4 as z",
            "params": [42, "hello", true, null]
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
        let rows = out
            .get("rows")
            .and_then(|v| v.as_array())
            .expect("rows should be present");
        assert_eq!(rows.len(), 1);

        let row = rows.first().and_then(|v| v.as_object()).expect("row object");
        assert_eq!(row.get("n").and_then(|v| v.as_i64()), Some(42));
        assert_eq!(row.get("t").and_then(|v| v.as_str()), Some("hello"));
        let b = row.get("b").cloned().unwrap_or(JsonValue::Null);
        assert!(matches!(b, JsonValue::Bool(true) | JsonValue::Number(_)));
        if let JsonValue::Number(n) = b {
            assert_eq!(n.as_i64(), Some(1));
        }
        assert_eq!(row.get("z"), Some(&JsonValue::Null));
    }

    #[test]
    fn execute_supports_positional_params() {
        let out = execute(&json!({
            "connection": "sqlite::memory:",
            "sql": "select ?1",
            "params": [7]
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn query_rejects_non_array_params() {
        let out = query(&json!({
            "connection": "sqlite::memory:",
            "sql": "select 1",
            "params": {"a": 1}
        }));

        assert_eq!(
            out.get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("sql_params_invalid")
        );
    }

    #[test]
    fn query_enforces_max_rows_limit() {
        let out = query(&json!({
            "connection": "sqlite::memory:",
            "sql": "select 1 as n union all select 2 as n",
            "max_rows": 1
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            out.get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("sql_query_failed")
        );
        assert!(out
            .get("error")
            .and_then(|v| v.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("exceeds max_rows"));
    }

    #[test]
    fn query_enforces_max_payload_bytes_limit() {
        let out = query(&json!({
            "connection": "sqlite::memory:",
            "sql": "select 'abcdefghijklmnopqrstuvwxyz' as payload",
            "max_payload_bytes": 8
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            out.get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("sql_query_failed")
        );
        assert!(out
            .get("error")
            .and_then(|v| v.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("exceeds max_payload_bytes"));
    }

    #[test]
    fn query_handles_high_row_count_when_within_limit() {
        let out = query(&json!({
            "connection": "sqlite::memory:",
            "sql": "with recursive cnt(x) as (select 1 union all select x + 1 from cnt where x < 128) select x from cnt",
            "max_rows": 128
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(out.get("row_count").and_then(|v| v.as_u64()), Some(128));
    }

    #[test]
    fn query_handles_high_payload_near_limit_boundary() {
        let out = query(&json!({
            "connection": "sqlite::memory:",
            "sql": "with recursive cnt(x) as (select 1 union all select x + 1 from cnt where x < 64) select x, 'aaaaaaaaaaaaaaaa' as payload from cnt",
            "max_rows": 64,
            "max_payload_bytes": 4096
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(out.get("row_count").and_then(|v| v.as_u64()), Some(64));
    }

    #[test]
    fn transaction_runs_execute_and_query_steps() {
        let out = transaction(&json!({
            "connection": "sqlite::memory:",
            "steps": [
                {
                    "sql": "create table if not exists t (id integer, label text)",
                    "mode": "execute"
                },
                {
                    "sql": "insert into t (id, label) values (?1, ?2)",
                    "mode": "execute",
                    "params": [1, "ok"]
                },
                {
                    "sql": "select label from t where id = ?1",
                    "mode": "query",
                    "params": [1]
                }
            ]
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(out.get("committed").and_then(|v| v.as_bool()), Some(true));
        let results = out
            .get("results")
            .and_then(|v| v.as_array())
            .expect("results array");
        assert_eq!(results.len(), 3);
        let query_result_rows = results[2]
            .get("rows")
            .and_then(|v| v.as_array())
            .expect("query rows");
        assert_eq!(query_result_rows.len(), 1);
    }

    #[test]
    fn transaction_rolls_back_on_step_failure() {
        let out = transaction(&json!({
            "connection": "sqlite::memory:",
            "steps": [
                {
                    "sql": "create table if not exists t (id integer)",
                    "mode": "execute"
                },
                {
                    "sql": "this is invalid sql",
                    "mode": "execute"
                }
            ]
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(out.get("committed").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(out.get("failed_step").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(
            out.get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("sql_transaction_step_failed")
        );
    }

    #[test]
    fn transaction_rollback_is_deterministic_for_persisted_connection() {
        let (connection, path) = sqlite_temp_connection("rollback-deterministic");

        let setup = execute(&json!({
            "connection": connection,
            "sql": "create table if not exists t (id integer, label text)"
        }));
        assert_eq!(setup.get("ok").and_then(|v| v.as_bool()), Some(true));

        let out = transaction(&json!({
            "connection": connection,
            "steps": [
                {
                    "sql": "insert into t (id, label) values (?1, ?2)",
                    "mode": "execute",
                    "params": [1, "should_rollback"]
                },
                {
                    "sql": "this is invalid sql",
                    "mode": "execute"
                }
            ]
        }));

        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(out.get("committed").and_then(|v| v.as_bool()), Some(false));

        let verify = query(&json!({
            "connection": connection,
            "sql": "select count(*) as count from t"
        }));
        assert_eq!(verify.get("ok").and_then(|v| v.as_bool()), Some(true));
        let rows = verify
            .get("rows")
            .and_then(|v| v.as_array())
            .expect("rows array");
        let count = rows
            .first()
            .and_then(|v| v.get("count"))
            .and_then(|v| v.as_i64());
        assert_eq!(count, Some(0));

        let _ = fs::remove_file(path);
    }
}
