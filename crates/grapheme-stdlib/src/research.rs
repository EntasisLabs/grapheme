use crate::{html, http, web};
use serde_json::{json, Value as JsonValue};

pub fn report(args: &JsonValue) -> JsonValue {
    let (materials_payload, options) = match build_materials_payload(args) {
        Ok(payload) => payload,
        Err(err) => return err,
    };

    let mut findings = Vec::new();
    let mut executive_points = Vec::new();

    for source in &materials_payload.sources {
        let highlights = if source.highlights.is_empty() && !source.snippet.trim().is_empty() {
            vec![normalize_report_line(&source.snippet)]
        } else {
            source.highlights.clone()
        };

        let key_point = highlights
            .first()
            .cloned()
            .unwrap_or_else(|| source.snippet.clone());

        if !key_point.is_empty() {
            findings.push(format!("- {}: {}", source.title, key_point));
            executive_points.push(key_point);
        }
    }

    let mut report_lines = vec![
        format!("Research report: {}", materials_payload.query),
        format!("Provider: {}", materials_payload.provider),
        format!("Sources analyzed: {}", materials_payload.sources.len()),
        "".to_string(),
        "Executive summary".to_string(),
    ];

    if executive_points.is_empty() {
        report_lines.push("- No strong findings extracted from fetched pages.".to_string());
    } else {
        for point in executive_points.into_iter().take(5) {
            report_lines.push(format!("- {}", point));
        }
    }

    report_lines.extend(["".to_string(), "Findings".to_string()]);

    if findings.is_empty() {
        report_lines.push("- No strong findings extracted from fetched pages.".to_string());
    } else {
        report_lines.extend(findings);
    }

    report_lines.push("".to_string());
    report_lines.push("Source details".to_string());
    for source in &materials_payload.sources {
        report_lines.push(format!("- {}", source.title));
        if !source.url.is_empty() {
            report_lines.push(format!("  url: {}", source.url));
        }
        if let Some(code) = source.status {
            report_lines.push(format!("  status: {}", code));
        }

        for h in source.highlights.iter().take(3) {
            report_lines.push(format!("  - {}", h));
        }
    }

    report_lines.push("".to_string());
    report_lines.push("Sources".to_string());
    for source in &materials_payload.sources {
        if !source.url.is_empty() {
            report_lines.push(format!("- {} ({})", source.title, source.url));
        }
    }

    let report_text = report_lines.join("\n");
    let summary = if report_text.chars().count() > options.report_chars {
        report_text
            .chars()
            .take(options.report_chars)
            .collect::<String>()
    } else {
        report_text
    };

    let materials_json = materials_payload.to_json(options.include_http_body);
    let sources_json = materials_payload
        .sources
        .iter()
        .map(|source| source.to_json(options.include_http_body))
        .collect::<Vec<_>>();

    ReportPayload {
        query: materials_payload.query,
        provider: materials_payload.provider,
        sources: sources_json,
        report: summary,
        materials: materials_json,
    }
    .to_json()
}

pub fn materials(args: &JsonValue) -> JsonValue {
    match build_materials_payload(args) {
        Ok((payload, options)) => payload.to_json(options.include_http_body),
        Err(err) => err,
    }
}

fn build_materials_payload(
    args: &JsonValue,
) -> Result<(MaterialsPayload, ResearchOptions), JsonValue> {
    let options = ResearchOptions::from_args(args);
    let search = web::search(args);
    if search.get("error").is_some() {
        return Err(search);
    }

    let query = search
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let provider = search
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("duckduckgo")
        .to_string();

    let hits = search
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|raw| SearchHit::from_json(&raw))
        .collect::<Vec<_>>();

    let mut sources = Vec::new();

    for (idx, hit) in hits.into_iter().enumerate() {
        if hit.url.is_empty() {
            sources.push(SourceRecord::missing_url(idx + 1, hit));
            continue;
        }

        let http_out = http::request("GET", &hit.url, None);
        let status = http_out.get("status").and_then(|v| v.as_u64());
        let body = http_out
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fetch_error = http_out
            .get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut md_args = json!({ "html": body });
        if let Some(options_json) = options.md_options.clone() {
            md_args["options"] = options_json;
        }

        let md_result = html::to_md(&md_args);
        let markdown = md_result
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let cleaned = html::clean_page_text_raw(&markdown, Some(options.per_source_chars));
        let conversion_error = md_result
            .get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let has_fetched_content = !cleaned.trim().is_empty() || !markdown.trim().is_empty();
        let can_use_fetched_content =
            fetch_error.is_none() && conversion_error.is_none() && has_fetched_content;

        let snippet_clean = normalize_report_line(&hit.snippet);
        let content_origin = if can_use_fetched_content {
            "fetched_page".to_string()
        } else {
            "snippet_fallback".to_string()
        };

        let content = if can_use_fetched_content {
            if !cleaned.trim().is_empty() {
                cleaned.clone()
            } else {
                markdown.clone()
            }
        } else {
            snippet_clean.clone()
        };

        let highlights = if can_use_fetched_content {
            extract_source_highlights(&content, &hit.snippet, 5, false)
        } else if snippet_clean.is_empty() {
            Vec::new()
        } else {
            vec![snippet_clean]
        };

        let md_result_payload = md_result.get("result").cloned();

        sources.push(SourceRecord::fetched(
            idx + 1,
            hit,
            status,
            fetch_error,
            conversion_error,
            content_origin,
            content,
            http_out,
            markdown,
            cleaned,
            highlights,
            md_result_payload,
        ));
    }

    Ok((
        MaterialsPayload {
            query,
            provider,
            sources,
        },
        options,
    ))
}

#[derive(Debug, Clone)]
struct SearchHit {
    title: String,
    url: String,
    snippet: String,
}

impl SearchHit {
    fn from_json(value: &JsonValue) -> Self {
        Self {
            title: value
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("(untitled)")
                .to_string(),
            url: value
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            snippet: value
                .get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct SourceRecord {
    source_id: String,
    title: String,
    url: String,
    snippet: String,
    status: Option<u64>,
    ok: bool,
    fetch_error: Option<String>,
    conversion_error: Option<String>,
    content_origin: String,
    content: String,
    http_payload: JsonValue,
    markdown: String,
    clean_text: String,
    highlights: Vec<String>,
    citation: String,
    md_result: Option<JsonValue>,
}

impl SourceRecord {
    fn missing_url(index: usize, hit: SearchHit) -> Self {
        let snippet_clean = normalize_report_line(&hit.snippet);
        let highlights = if snippet_clean.is_empty() {
            Vec::new()
        } else {
            vec![snippet_clean.clone()]
        };
        let content = highlights.first().cloned().unwrap_or_default();

        Self {
            source_id: format!("s{index}"),
            title: hit.title.clone(),
            url: hit.url,
            snippet: hit.snippet,
            status: None,
            ok: false,
            fetch_error: Some("missing url".to_string()),
            conversion_error: None,
            content_origin: "snippet_fallback".to_string(),
            content,
            http_payload: JsonValue::Null,
            markdown: String::new(),
            clean_text: String::new(),
            highlights,
            citation: format!("[s{index}] {}", hit.title),
            md_result: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fetched(
        index: usize,
        hit: SearchHit,
        status: Option<u64>,
        fetch_error: Option<String>,
        conversion_error: Option<String>,
        content_origin: String,
        content: String,
        http_payload: JsonValue,
        markdown: String,
        clean_text: String,
        highlights: Vec<String>,
        md_result: Option<JsonValue>,
    ) -> Self {
        Self {
            source_id: format!("s{index}"),
            title: hit.title.clone(),
            url: hit.url,
            snippet: hit.snippet,
            status,
            ok: fetch_error.is_none(),
            fetch_error,
            conversion_error,
            content_origin,
            content,
            http_payload,
            markdown,
            clean_text,
            highlights,
            citation: format!("[s{index}] {}", hit.title),
            md_result,
        }
    }

    fn to_json(&self, include_http_body: bool) -> JsonValue {
        let http_json = if include_http_body {
            self.http_payload.clone()
        } else if self.url.is_empty() {
            JsonValue::Null
        } else {
            json!({
                "status": self.status,
                "url": self.url,
                "status_line": self
                    .status
                    .map(|code| format!("HTTP {}", code))
                    .unwrap_or_else(|| "HTTP".to_string()),
            })
        };

        let mut out = json!({
            "source_id": self.source_id,
            "title": self.title,
            "url": self.url,
            "snippet": self.snippet,
            "status": self.status,
            "ok": self.ok,
            "fetch_error": self.fetch_error,
            "conversion_error": self.conversion_error,
            "content_origin": self.content_origin,
            "content": self.content,
            "http": http_json,
            "markdown": self.markdown,
            "clean_text": self.clean_text,
            "highlights": self.highlights,
            "citation": self.citation,
        });

        if let Some(md_result) = &self.md_result {
            out["md_result"] = md_result.clone();
        }

        out
    }
}

#[derive(Debug, Clone)]
struct MaterialsPayload {
    query: String,
    provider: String,
    sources: Vec<SourceRecord>,
}

impl MaterialsPayload {
    fn to_json(&self, include_http_body: bool) -> JsonValue {
        let sources = self
            .sources
            .iter()
            .map(|source| source.to_json(include_http_body))
            .collect::<Vec<_>>();

        json!({
            "query": self.query,
            "provider": self.provider,
            "count": sources.len(),
            "sources": sources,
        })
    }
}

#[derive(Debug, Clone)]
struct ReportPayload {
    query: String,
    provider: String,
    sources: Vec<JsonValue>,
    report: String,
    materials: JsonValue,
}

impl ReportPayload {
    fn to_json(&self) -> JsonValue {
        json!({
            "query": self.query,
            "provider": self.provider,
            "count": self.sources.len(),
            "sources": self.sources,
            "report": self.report,
            "materials": self.materials,
        })
    }
}

pub fn extract_source_highlights(
    text: &str,
    snippet: &str,
    max_points: usize,
    allow_snippet_fallback: bool,
) -> Vec<String> {
    let mut points = Vec::new();

    for line in text.lines().map(str::trim) {
        if line.len() < 35 || line.len() > 220 {
            continue;
        }
        if is_weak_fact_line(line) {
            continue;
        }

        let normalized = normalize_report_line(line);
        if normalized.is_empty() {
            continue;
        }

        if !points
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&normalized))
        {
            points.push(normalized);
        }

        if points.len() >= max_points {
            break;
        }
    }

    if allow_snippet_fallback && points.is_empty() && !snippet.trim().is_empty() {
        let normalized_snippet = normalize_report_line(snippet.trim());
        if !normalized_snippet.is_empty() {
            points.push(normalized_snippet);
        }
    }

    points
}

pub fn normalize_report_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_tag = false;

    for ch in line.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    out = out
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_weak_fact_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.starts_with("canonical:")
        || lower.starts_with("meta-")
        || lower.starts_with("og:")
        || lower.starts_with("twitter:")
        || lower.starts_with("keywords:")
        || lower.starts_with("description:")
        || lower.starts_with("author:")
        || lower.starts_with("published")
        || lower.starts_with("updated")
        || lower.starts_with("share this")
        || lower.contains("application/ld+json")
        || lower.contains("schema.org")
        || lower.contains("cookie")
        || lower.contains("privacy")
        || lower.contains("terms")
        || lower.contains("robots")
        || lower.contains("viewport")
        || lower.contains("favicon")
        || lower.contains(": http://")
        || lower.contains(": https://")
}

fn arg_u64(args: &JsonValue, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
    })
}

fn arg_bool(args: &JsonValue, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| {
        v.as_bool().or_else(|| {
            v.as_str()
                .and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => Some(true),
                    "false" | "0" | "no" | "off" => Some(false),
                    _ => None,
                })
        })
    })
}

#[derive(Debug, Clone)]
struct ResearchOptions {
    per_source_chars: usize,
    report_chars: usize,
    include_http_body: bool,
    md_options: Option<JsonValue>,
}

impl ResearchOptions {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            per_source_chars: arg_u64(args, "per_source_chars")
                .map(|n| n as usize)
                .unwrap_or(5000),
            report_chars: arg_u64(args, "report_chars")
                .map(|n| n as usize)
                .unwrap_or(2500),
            include_http_body: arg_bool(args, "include_http_body").unwrap_or(false),
            md_options: args.get("md_options").cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_hit() -> SearchHit {
        SearchHit {
            title: "Example".to_string(),
            url: "https://example.com".to_string(),
            snippet: "Example snippet".to_string(),
        }
    }

    #[test]
    fn report_requires_query() {
        let out = report(&json!({}));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn materials_requires_query() {
        let out = materials(&json!({}));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn extract_highlights_skips_canonical_metadata_lines() {
        let text = "canonical: https://example.com/article\nEggs are versatile and can be boiled, poached, or scrambled in under 10 minutes.\nkeywords: egg, cooking";
        let highlights = extract_source_highlights(text, "fallback snippet", 3, true);
        assert!(!highlights.is_empty());
        assert!(highlights[0].contains("Eggs are versatile"));
        assert!(!highlights[0].to_lowercase().starts_with("canonical:"));
    }

    #[test]
    fn normalize_report_line_removes_html_markup() {
        let line = "Learn <b>how</b> to cook <em>eggs</em> &amp; serve warm.";
        let normalized = normalize_report_line(line);
        assert_eq!(normalized, "Learn how to cook eggs & serve warm.");
    }

    #[test]
    fn extract_highlights_normalizes_snippet_fallback_markup() {
        let highlights = extract_source_highlights("", "Learn <b>how</b> to cook eggs", 3, true);
        assert_eq!(highlights, vec!["Learn how to cook eggs"]);
    }

    #[test]
    fn extract_highlights_no_snippet_fallback_when_disabled() {
        let highlights = extract_source_highlights("", "fallback snippet", 3, false);
        assert!(highlights.is_empty());
    }

    #[test]
    fn source_record_missing_url_uses_snippet_fallback_shape() {
        let hit = SearchHit {
            title: "No URL".to_string(),
            url: String::new(),
            snippet: "Fallback <b>snippet</b> text".to_string(),
        };

        let record = SourceRecord::missing_url(1, hit);
        let out = record.to_json(false);

        assert_eq!(out.get("source_id").and_then(|v| v.as_str()), Some("s1"));
        assert_eq!(
            out.get("content_origin").and_then(|v| v.as_str()),
            Some("snippet_fallback")
        );
        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            out.get("fetch_error").and_then(|v| v.as_str()),
            Some("missing url")
        );
        assert!(out.get("http").map(|v| v.is_null()).unwrap_or(false));
        assert_eq!(
            out.get("highlights")
                .and_then(|v| v.as_array())
                .and_then(|v| v.first())
                .and_then(|v| v.as_str()),
            Some("Fallback snippet text")
        );
    }

    #[test]
    fn source_record_to_json_redacts_http_body_when_disabled() {
        let record = SourceRecord::fetched(
            2,
            sample_hit(),
            Some(200),
            None,
            None,
            "fetched_page".to_string(),
            "clean content".to_string(),
            json!({"status": 200, "url": "https://example.com", "body": "secret body"}),
            "markdown".to_string(),
            "clean text".to_string(),
            vec!["point".to_string()],
            Some(json!({"nodes": 3})),
        );

        let out = record.to_json(false);
        let http = out.get("http").expect("http payload must exist");

        assert_eq!(http.get("status").and_then(|v| v.as_u64()), Some(200));
        assert_eq!(
            http.get("status_line").and_then(|v| v.as_str()),
            Some("HTTP 200")
        );
        assert!(http.get("body").is_none());
        assert_eq!(
            out.get("md_result")
                .and_then(|v| v.get("nodes"))
                .and_then(|v| v.as_u64()),
            Some(3)
        );
    }

    #[test]
    fn source_record_to_json_includes_http_body_when_enabled() {
        let record = SourceRecord::fetched(
            3,
            sample_hit(),
            Some(200),
            None,
            None,
            "fetched_page".to_string(),
            "clean content".to_string(),
            json!({"status": 200, "url": "https://example.com", "body": "visible body"}),
            "markdown".to_string(),
            "clean text".to_string(),
            vec!["point".to_string()],
            None,
        );

        let out = record.to_json(true);
        assert_eq!(
            out.get("http")
                .and_then(|v| v.get("body"))
                .and_then(|v| v.as_str()),
            Some("visible body")
        );
    }

    #[test]
    fn materials_and_report_payloads_preserve_count_shape() {
        let source_json = SourceRecord::fetched(
            4,
            sample_hit(),
            Some(204),
            None,
            None,
            "fetched_page".to_string(),
            "content".to_string(),
            json!({"status": 204, "url": "https://example.com", "body": "body"}),
            "md".to_string(),
            "clean".to_string(),
            vec!["highlight".to_string()],
            None,
        )
        .to_json(false);

        let materials = MaterialsPayload {
            query: "q".to_string(),
            provider: "duckduckgo".to_string(),
            sources: vec![SourceRecord::missing_url(
                5,
                SearchHit {
                    title: "T".to_string(),
                    url: "".to_string(),
                    snippet: "S".to_string(),
                },
            )],
        }
        .to_json(false);

        assert_eq!(materials.get("count").and_then(|v| v.as_u64()), Some(1));

        let report = ReportPayload {
            query: "q".to_string(),
            provider: "duckduckgo".to_string(),
            sources: vec![source_json],
            report: "summary".to_string(),
            materials: materials.clone(),
        }
        .to_json();

        assert_eq!(report.get("count").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(
            report
                .get("materials")
                .and_then(|v| v.get("count"))
                .and_then(|v| v.as_u64()),
            Some(1)
        );
    }
}
