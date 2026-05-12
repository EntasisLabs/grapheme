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

fn ensure_any_drivers() {
    SQLX_ANY_DRIVERS.call_once(sqlx::any::install_default_drivers);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
