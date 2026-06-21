//! Portable JSON frame handles exchanged between `data.*` ops.

use serde_json::{json, Map, Value as JsonValue};
use std::collections::BTreeMap;

pub const FRAME_KIND: &str = "grapheme.data.frame/v1";

#[cfg(feature = "data")]
use polars::prelude::*;

pub fn frame_from_arg(args: &JsonValue) -> Option<&JsonValue> {
    args.get("frame")
        .or_else(|| args.get("__input"))
        .and_then(|value| {
            if is_frame(value) {
                Some(value)
            } else {
                value.get("frame").filter(|frame| is_frame(frame))
            }
        })
}

pub fn is_frame(value: &JsonValue) -> bool {
    value
        .get("kind")
        .and_then(|v| v.as_str())
        .is_some_and(|kind| kind == FRAME_KIND)
}

pub fn schema_from_frame(frame: &JsonValue) -> JsonValue {
    json!({
        "kind": FRAME_KIND,
        "row_count": frame.get("row_count").cloned().unwrap_or(json!(0)),
        "column_count": frame.get("column_count").cloned().unwrap_or(json!(0)),
        "columns": frame.get("columns").cloned().unwrap_or(json!([])),
        "source_path": frame.get("source_path").cloned().unwrap_or(JsonValue::Null),
    })
}

pub fn rows_from_frame(frame: &JsonValue) -> Option<&Vec<JsonValue>> {
    frame.get("rows").and_then(|v| v.as_array())
}

pub fn filter_frame(frame: &JsonValue, column: &str, eq: &JsonValue) -> Result<JsonValue, String> {
    let rows = rows_from_frame(frame).ok_or_else(|| "frame missing rows array".to_string())?;
    let mut filtered = Vec::new();

    for row in rows {
        let Some(obj) = row.as_object() else {
            continue;
        };
        if obj.get(column) == Some(eq) {
            filtered.push(row.clone());
        }
    }

    Ok(json!({
        "kind": FRAME_KIND,
        "source_path": frame.get("source_path").cloned().unwrap_or(JsonValue::Null),
        "row_count": filtered.len(),
        "column_count": frame.get("column_count").cloned().unwrap_or(json!(0)),
        "columns": frame.get("columns").cloned().unwrap_or(json!([])),
        "rows": filtered,
    }))
}

pub fn group_frame(frame: &JsonValue, by: &str) -> Result<JsonValue, String> {
    let rows = rows_from_frame(frame).ok_or_else(|| "frame missing rows array".to_string())?;
    let mut groups: BTreeMap<String, Vec<JsonValue>> = BTreeMap::new();

    for row in rows {
        let key = row
            .get(by)
            .map(json_scalar_key)
            .unwrap_or_else(|| "null".to_string());
        groups.entry(key).or_default().push(row.clone());
    }

    let grouped = groups
        .into_iter()
        .map(|(key, rows)| {
            json!({
                "key": key,
                "row_count": rows.len(),
                "rows": rows,
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "kind": "grapheme.data.group/v1",
        "by": by,
        "group_count": grouped.len(),
        "groups": grouped,
    }))
}

#[cfg(feature = "data")]
pub fn read_csv_path(path: &str, max_rows: usize) -> Result<JsonValue, String> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .with_infer_schema_length(Some(256))
        .try_into_reader_with_file_path(Some(path.into()))
        .map_err(|err| format!("csv open failed: {err}"))?
        .finish()
        .map_err(|err| format!("csv parse failed: {err}"))?;

    let df = if df.height() > max_rows {
        df.slice(0, max_rows)
    } else {
        df
    };

    dataframe_to_frame(&df, Some(path))
}

#[cfg(feature = "data")]
fn dataframe_to_frame(df: &DataFrame, source_path: Option<&str>) -> Result<JsonValue, String> {
    let columns = df
        .get_columns()
        .iter()
        .map(|column| {
            json!({
                "name": column.name().to_string(),
                "dtype": format!("{}", column.dtype()),
            })
        })
        .collect::<Vec<_>>();

    let mut rows = Vec::with_capacity(df.height());
    for row_idx in 0..df.height() {
        let mut obj = Map::new();
        for column in df.get_columns() {
            let name = column.name().to_string();
            obj.insert(name, column_value_to_json(column, row_idx)?);
        }
        rows.push(JsonValue::Object(obj));
    }

    Ok(json!({
        "kind": FRAME_KIND,
        "source_path": source_path,
        "row_count": df.height(),
        "column_count": df.width(),
        "columns": columns,
        "rows": rows,
    }))
}

#[cfg(feature = "data")]
fn column_value_to_json(column: &Column, row: usize) -> Result<JsonValue, String> {
    let series = column.as_materialized_series();
    let value = series
        .get(row)
        .map_err(|err| format!("read cell at row {row}: {err}"))?;

    Ok(match value {
        AnyValue::Null => JsonValue::Null,
        AnyValue::Boolean(v) => json!(v),
        AnyValue::Int8(v) => json!(v),
        AnyValue::Int16(v) => json!(v),
        AnyValue::Int32(v) => json!(v),
        AnyValue::Int64(v) => json!(v),
        AnyValue::UInt8(v) => json!(v),
        AnyValue::UInt16(v) => json!(v),
        AnyValue::UInt32(v) => json!(v),
        AnyValue::UInt64(v) => json!(v),
        AnyValue::Float32(v) => json!(v),
        AnyValue::Float64(v) => json!(v),
        AnyValue::String(v) => json!(v),
        other => JsonValue::String(other.to_string()),
    })
}

fn json_scalar_key(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(v) => v.to_string(),
        JsonValue::Number(v) => v.to_string(),
        JsonValue::String(v) => v.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn filter_frame_matches_column_value() {
        let frame = json!({
            "kind": FRAME_KIND,
            "row_count": 2,
            "column_count": 2,
            "columns": [{"name": "city", "dtype": "String"}],
            "rows": [
                {"city": "Paris", "name": "Ada"},
                {"city": "London", "name": "Grace"},
            ]
        });

        let filtered = filter_frame(&frame, "city", &json!("Paris")).expect("filter frame");
        assert_eq!(filtered.get("row_count").and_then(|v| v.as_u64()), Some(1));
    }

    #[cfg(feature = "data")]
    #[test]
    fn read_csv_path_parses_fixture() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        write!(
            file,
            "id,name\n1,Ada\n2,Grace\n"
        )
        .expect("write csv");

        let frame = read_csv_path(file.path().to_str().expect("utf8 path"), 100)
            .expect("read csv");
        assert_eq!(frame.get("row_count").and_then(|v| v.as_u64()), Some(2));
        assert!(is_frame(&frame));
    }
}
