use serde_json::{json, Value as JsonValue};

pub fn to_md(args: &JsonValue) -> JsonValue {
    let request = match ToMdRequest::from_args(args) {
        Ok(request) => request,
        Err(err) => {
            return json!({
                "error": err,
                "text": "",
                "markdown": "",
            });
        }
    };

    match html_to_markdown_rs::convert(&request.html, request.options.clone()) {
        Ok(result) => {
            let markdown = result.content.as_deref().unwrap_or_default().to_string();
            let result_json = serde_json::to_value(&result).unwrap_or(JsonValue::Null);
            json!({
                "text": markdown.clone(),
                "markdown": markdown,
                "result": result_json,
                "used_options": request.options,
            })
        }
        Err(err) => json!({
            "error": format!("html to markdown conversion failed: {err}"),
            "text": "",
            "markdown": "",
        }),
    }
}

pub fn clean_text(args: &JsonValue) -> JsonValue {
    let request = CleanTextRequest::from_args(args);
    let cleaned = clean_page_text_raw(&request.text, request.max_chars);
    json!({
        "text": cleaned,
        "length": cleaned.chars().count(),
    })
}

pub fn clean_page_text_raw(raw: &str, max_chars: Option<usize>) -> String {
    let mut lines = Vec::new();
    let mut in_code = false;

    for original in raw.lines() {
        let line = original.trim();

        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || line.is_empty() {
            continue;
        }

        let mut candidate = line.to_string();
        for marker in [
            " const ",
            " window.",
            " document.",
            " function ",
            " @media ",
            " input[type=",
            " { font-family",
            "::-webkit",
            " appearance:",
            " let ",
            " var ",
        ] {
            if let Some(idx) = candidate.find(marker) {
                if idx > 20 {
                    candidate.truncate(idx);
                } else {
                    candidate.clear();
                }
                break;
            }
        }

        let candidate = candidate.trim();
        if candidate.is_empty() {
            continue;
        }

        let lower = candidate.to_lowercase();
        let noisy = lower.starts_with("skip to")
            || lower.contains("cookie")
            || lower.contains("privacy policy")
            || lower.contains("terms of service")
            || lower.starts_with("open menu")
            || lower.contains("keyboard shortcuts")
            || lower.starts_with("sign in")
            || lower.starts_with("sign up")
            || lower.starts_with("footer")
            || lower.starts_with("copyright")
            || candidate.starts_with("window.")
            || candidate.starts_with("document.")
            || candidate.starts_with("function ")
            || candidate.starts_with("const ")
            || candidate.starts_with("let ")
            || candidate.starts_with("var ")
            || candidate.starts_with("@media");

        if noisy {
            continue;
        }

        lines.push(candidate.to_string());
    }

    let cleaned = lines.join("\n");
    match max_chars {
        Some(limit) if cleaned.chars().count() > limit => cleaned.chars().take(limit).collect(),
        _ => cleaned,
    }
}

#[derive(Debug, Clone)]
struct ToMdRequest {
    html: String,
    options: Option<html_to_markdown_rs::ConversionOptions>,
}

impl ToMdRequest {
    fn from_args(args: &JsonValue) -> Result<Self, String> {
        Ok(Self {
            html: arg_text(args, "html"),
            options: parse_html_to_md_options(args)?,
        })
    }
}

#[derive(Debug, Clone)]
struct CleanTextRequest {
    text: String,
    max_chars: Option<usize>,
}

impl CleanTextRequest {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            text: arg_text(args, "text"),
            max_chars: arg_u64(args, "max_chars").map(|v| v as usize),
        }
    }
}

fn parse_html_to_md_options(
    args: &JsonValue,
) -> Result<Option<html_to_markdown_rs::ConversionOptions>, String> {
    let Some(raw) = args.get("options") else {
        return Ok(None);
    };

    if !raw.is_object() {
        return Err("html.to_md options must be an object".to_string());
    }

    serde_json::from_value::<html_to_markdown_rs::ConversionOptions>(raw.clone())
        .map(Some)
        .map_err(|err| format!("invalid html.to_md options: {err}"))
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default()
}

fn arg_u64(args: &JsonValue, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn clean_text_filters_script_noise() {
        let out = clean_text(&json!({
            "text": "const x = 1;\nKeyboard shortcuts\nReal content line"
        }));
        let text = out.get("text").and_then(|v| v.as_str()).unwrap_or("");
        assert!(text.contains("Real content line"));
        assert!(!text.contains("Keyboard shortcuts"));
        assert!(!text.contains("const x = 1"));
    }

    #[test]
    fn to_md_options_enable_document_structure() {
        let out = to_md(&json!({
            "html": "<h1>Title</h1><p>Hello</p>",
            "options": {
                "include_document_structure": true,
                "extract_metadata": true,
                "output_format": "markdown"
            }
        }));

        assert!(out.get("error").is_none());
        assert!(out.get("result").is_some());
        assert_eq!(
            out.get("used_options")
                .and_then(|v| v.get("include_document_structure"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn to_md_rejects_non_object_options() {
        let out = to_md(&json!({
            "html": "<p>hello</p>",
            "options": "strict"
        }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }
}
