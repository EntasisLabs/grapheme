use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Read, Write};

const ENVELOPE_SCHEMA: &str = "grapheme.host.result.envelope/v1";
const CHART_WIDTH: f64 = 640.0;
const CHART_HEIGHT: f64 = 360.0;
const PAD: f64 = 40.0;

#[derive(Debug, Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    args: Value,
}

fn print_json(value: &Value) {
    let serialized = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let mut stdout = io::stdout();
    let _ = stdout.write_all(serialized.as_bytes());
    let _ = stdout.flush();
}

fn envelope(data: Value) -> Value {
    json!({
        "data": data,
        "meta": {
            "schema": ENVELOPE_SCHEMA,
            "engine": "plot-rs-wasm",
            "version": "0.1.0",
        },
        "error": null,
    })
}

fn failure(message: impl Into<String>) -> Value {
    json!({
        "data": null,
        "meta": { "schema": ENVELOPE_SCHEMA, "engine": "plot-rs-wasm" },
        "error": message.into(),
    })
}

fn series_points(series: &Value) -> Result<Vec<Point>, String> {
    let Some(items) = series.as_array() else {
        return Err("series must be a non-empty array".to_string());
    };
    if items.is_empty() {
        return Err("series must be a non-empty array".to_string());
    }

    let mut points = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        if let Some(number) = item.as_f64() {
            points.push(Point {
                x: idx as f64,
                y: number,
            });
            continue;
        }
        if let Some(obj) = item.as_object() {
            let x = obj
                .get("x")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("series[{idx}] missing numeric x"))?;
            let y = obj
                .get("y")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("series[{idx}] missing numeric y"))?;
            points.push(Point { x, y });
            continue;
        }
        return Err(format!("series[{idx}] must be a number or {{x,y}} object"));
    }
    Ok(points)
}

fn scatter_points(points: &Value) -> Result<Vec<Point>, String> {
    let Some(items) = points.as_array() else {
        return Err("points must be a non-empty array".to_string());
    };
    if items.is_empty() {
        return Err("points must be a non-empty array".to_string());
    }

    let mut out = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        let Some(obj) = item.as_object() else {
            return Err(format!("points[{idx}] must be an object with x and y"));
        };
        let x = obj
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("points[{idx}] missing numeric x"))?;
        let y = obj
            .get("y")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("points[{idx}] missing numeric y"))?;
        out.push(Point { x, y });
    }
    Ok(out)
}

fn bounds(points: &[Point]) -> (f64, f64, f64, f64) {
    let mut min_x = points[0].x;
    let mut max_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_y = points[0].y;
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    (min_x, max_x, min_y, max_y)
}

fn map_x(value: f64, min_x: f64, max_x: f64) -> f64 {
    if (max_x - min_x).abs() < f64::EPSILON {
        return PAD + (CHART_WIDTH - 2.0 * PAD) / 2.0;
    }
    PAD + (value - min_x) / (max_x - min_x) * (CHART_WIDTH - 2.0 * PAD)
}

fn map_y(value: f64, min_y: f64, max_y: f64) -> f64 {
    if (max_y - min_y).abs() < f64::EPSILON {
        return CHART_HEIGHT / 2.0;
    }
    CHART_HEIGHT - PAD - (value - min_y) / (max_y - min_y) * (CHART_HEIGHT - 2.0 * PAD)
}

fn svg_header(title: Option<&str>) -> String {
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        CHART_WIDTH, CHART_HEIGHT, CHART_WIDTH, CHART_HEIGHT
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>");
    if let Some(title) = title {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"20\" font-family=\"sans-serif\" font-size=\"14\" fill=\"#111\">{}</text>",
            PAD,
            escape_xml(title)
        ));
    }
    svg
}

fn svg_footer() -> &'static str {
    "</svg>"
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn chart_envelope(op: &str, svg: String) -> Value {
    envelope(json!({
        "ok": true,
        "op": op,
        "format": "svg",
        "content": svg,
        "bytes_base64": STANDARD.encode(svg.as_bytes()),
    }))
}

fn op_line(args: &Value) -> Value {
    let Some(series) = args.get("series") else {
        return failure("missing required arg: series");
    };
    let title = args.get("title").and_then(|v| v.as_str());

    let points = match series_points(series) {
        Ok(points) => points,
        Err(err) => return failure(err),
    };

    let (min_x, max_x, min_y, max_y) = bounds(&points);
    let mut svg = svg_header(title);
    svg.push_str(&format!(
        "<polyline fill=\"none\" stroke=\"#2563eb\" stroke-width=\"2\" points=\"{}\"/>",
        points
            .iter()
            .map(|p| format!("{},{}", map_x(p.x, min_x, max_x), map_y(p.y, min_y, max_y)))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    svg.push_str(svg_footer());
    chart_envelope("plot.line", svg)
}

fn op_bar(args: &Value) -> Value {
    let Some(series) = args.get("series") else {
        return failure("missing required arg: series");
    };
    let title = args.get("title").and_then(|v| v.as_str());

    let points = match series_points(series) {
        Ok(points) => points,
        Err(err) => return failure(err),
    };

    let (min_x, max_x, min_y, max_y) = bounds(&points);
    let bar_width = ((CHART_WIDTH - 2.0 * PAD) / points.len() as f64 * 0.7).max(4.0);
    let mut svg = svg_header(title);

    for point in points {
        let x = map_x(point.x, min_x, max_x) - bar_width / 2.0;
        let y = map_y(point.y, min_y, max_y);
        let base = map_y(min_y, min_y, max_y);
        let height = (base - y).max(1.0);
        svg.push_str(&format!(
            "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{bar_width:.2}\" height=\"{height:.2}\" fill=\"#16a34a\"/>"
        ));
    }

    svg.push_str(svg_footer());
    chart_envelope("plot.bar", svg)
}

fn op_scatter(args: &Value) -> Value {
    let Some(points_value) = args.get("points") else {
        return failure("missing required arg: points");
    };
    let title = args.get("title").and_then(|v| v.as_str());

    let points = match scatter_points(points_value) {
        Ok(points) => points,
        Err(err) => return failure(err),
    };

    let (min_x, max_x, min_y, max_y) = bounds(&points);
    let mut svg = svg_header(title);
    for point in points {
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"4\" fill=\"#9333ea\"/>",
            map_x(point.x, min_x, max_x),
            map_y(point.y, min_y, max_y)
        ));
    }
    svg.push_str(svg_footer());
    chart_envelope("plot.scatter", svg)
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        print_json(&failure("failed to read stdin"));
        return;
    }

    let request: Request = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => {
            print_json(&failure("invalid request json"));
            return;
        }
    };

    let result = match request.op.as_str() {
        "line" => op_line(&request.args),
        "bar" => op_bar(&request.args),
        "scatter" => op_scatter(&request.args),
        other => failure(format!("unsupported plot op: {other}")),
    };

    print_json(&result);
}
