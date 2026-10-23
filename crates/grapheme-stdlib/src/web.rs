//! Web search providers.
//!
//! `reqwest` 0.13 with rustls replaces the `websearch` 0.1.1 crate, which pinned
//! reqwest 0.11 and rustls 0.21 (`rustls-webpki` 0.101 / `webpki-roots` 0.25,
//! neither of which has a fix). Public ops and the result fields the old crate
//! serialized stay the same. Brave was already a direct Search API call.

use serde_json::{Value as JsonValue, json};
use std::env;
use std::time::Duration;

const SEARCH_TIMEOUT: Duration = Duration::from_secs(15);
const DUCKDUCKGO_HTML: &str = "https://html.duckduckgo.com/html";
const DUCKDUCKGO_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";

pub fn search(args: &JsonValue) -> JsonValue {
    let request = SearchRequest::from_args(args);
    if request.query.trim().is_empty() {
        return json!({ "error": "missing required arg: query" });
    }

    let provider = request.provider.clone();
    run_search(request.query, provider, request.max_results)
}

pub fn search_provider(args: &JsonValue, provider: &str) -> JsonValue {
    let request = SearchRequest::from_args(args).with_provider(provider);
    search(&request.to_json())
}

pub fn providers() -> JsonValue {
    let providers = provider_catalog()
        .into_iter()
        .map(|provider| provider.to_json())
        .collect::<Vec<_>>();

    json!({
        "count": providers.len(),
        "providers": providers,
    })
}

pub fn capabilities(args: &JsonValue) -> JsonValue {
    let Some(target) = args
        .get("provider")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
    else {
        return providers();
    };

    let Some(provider) = provider_catalog().into_iter().find(|p| p.id == target) else {
        return json!({
            "error": format!("unknown provider '{}'", target),
            "available_providers": provider_catalog().into_iter().map(|p| p.id).collect::<Vec<_>>()
        });
    };

    json!({ "provider": provider.to_json() })
}

enum ProviderAuth {
    DuckDuckGo,
    Google { api_key: String, cx: String },
    Exa { api_key: String },
    Tavily { api_key: String },
    Brave { api_key: String },
}

fn run_search(query: String, provider: String, max_results: Option<u32>) -> JsonValue {
    let auth = match provider_auth(&provider) {
        Ok(auth) => auth,
        Err(err) => return SearchResponse::failure(query, provider, err).to_json(),
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            return json!({ "error": format!("web search runtime init failed: {err}") });
        }
    };

    runtime.block_on(async move {
        let client = match search_client() {
            Ok(client) => client,
            Err(err) => {
                return SearchResponse::failure(query, provider, err).to_json();
            }
        };

        match dispatch_search(&client, &auth, &query, max_results).await {
            Ok(results) => {
                SearchResponse::success(query, provider, JsonValue::Array(results)).to_json()
            }
            Err(err) => SearchResponse::failure(query, provider, err).to_json(),
        }
    })
}

fn search_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(SEARCH_TIMEOUT)
        .user_agent(concat!("grapheme-stdlib/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("web search client init failed: {err}"))
}

async fn dispatch_search(
    client: &reqwest::Client,
    auth: &ProviderAuth,
    query: &str,
    max_results: Option<u32>,
) -> Result<Vec<JsonValue>, String> {
    match auth {
        ProviderAuth::DuckDuckGo => duckduckgo_search(client, query, max_results).await,
        ProviderAuth::Google { api_key, cx } => {
            google_search(client, query, api_key, cx, max_results).await
        }
        ProviderAuth::Exa { api_key } => exa_search(client, query, api_key, max_results).await,
        ProviderAuth::Tavily { api_key } => {
            tavily_search(client, query, api_key, max_results).await
        }
        ProviderAuth::Brave { api_key } => brave_search(client, query, api_key, max_results).await,
    }
}

fn provider_auth(provider_id: &str) -> Result<ProviderAuth, String> {
    match provider_id {
        "duckduckgo" => Ok(ProviderAuth::DuckDuckGo),
        "google" => {
            let api_key = require_env(
                "GOOGLE_API_KEY",
                "Configuration error: Google API key is required",
            )?;
            let cx = require_env(
                "GOOGLE_CX",
                "Configuration error: Google Search Engine ID (cx) is required",
            )?;
            Ok(ProviderAuth::Google { api_key, cx })
        }
        "xaviv" => {
            let api_key = match env::var("XAVIV_API_KEY").or_else(|_| env::var("EXA_API_KEY")) {
                Ok(key) if !key.is_empty() => key,
                Ok(_) => {
                    return Err("Configuration error: Exa API key is required".to_string());
                }
                Err(_) => {
                    return Err(
                        "missing XAVIV_API_KEY or EXA_API_KEY environment variable".to_string()
                    );
                }
            };
            Ok(ProviderAuth::Exa { api_key })
        }
        "tavily" => {
            let api_key = match env::var("TAVILY_API_KEY") {
                Ok(key) if !key.is_empty() => key,
                Ok(_) => {
                    return Err("Configuration error: Tavily API key is required".to_string());
                }
                Err(_) => {
                    return Err("missing TAVILY_API_KEY environment variable".to_string());
                }
            };
            if let Some(err) = tavily_key_format_error(&api_key) {
                return Err(err);
            }
            Ok(ProviderAuth::Tavily { api_key })
        }
        "brave" => {
            let api_key = match env::var("BRAVE_API_KEY") {
                Ok(key) if !key.is_empty() => key,
                _ => {
                    return Err("missing BRAVE_API_KEY environment variable".to_string());
                }
            };
            Ok(ProviderAuth::Brave { api_key })
        }
        other => Err(format!(
            "unsupported websearch provider '{other}'; supported: duckduckgo, google, xaviv, tavily, brave"
        )),
    }
}

fn require_env(key: &str, empty_message: &str) -> Result<String, String> {
    match env::var(key) {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) => Err(empty_message.to_string()),
        Err(_) => Err(format!("missing {key} environment variable")),
    }
}

fn tavily_key_format_error(api_key: &str) -> Option<String> {
    if api_key.starts_with("tvly-") {
        None
    } else {
        Some(
            "Configuration error: Invalid Tavily API key format. Keys should start with 'tvly-'"
                .to_string(),
        )
    }
}

async fn duckduckgo_search(
    client: &reqwest::Client,
    query: &str,
    max_results: Option<u32>,
) -> Result<Vec<JsonValue>, String> {
    let response = client
        .post(DUCKDUCKGO_HTML)
        .header("User-Agent", DUCKDUCKGO_USER_AGENT)
        .header("Referer", "https://html.duckduckgo.com/")
        .form(&[("q", query), ("b", ""), ("kl", "wt-wt")])
        .send()
        .await
        .map_err(|err| format!("duckduckgo request failed: {err}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| format!("duckduckgo request failed: {err}"))?;
    if !status.is_success() {
        return Err(format!(
            "duckduckgo API error {}: {}",
            status.as_u16(),
            body.trim()
        ));
    }

    parse_duckduckgo_html(&body, max_results.unwrap_or(10))
}

fn parse_duckduckgo_html(html: &str, max_results: u32) -> Result<Vec<JsonValue>, String> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let result_selector = Selector::parse("h2.result__title a")
        .map_err(|err| format!("duckduckgo result selector failed: {err}"))?;
    let snippet_selector = Selector::parse(".result__snippet")
        .map_err(|err| format!("duckduckgo snippet selector failed: {err}"))?;

    let result_links: Vec<_> = document.select(&result_selector).collect();
    let result_snippets: Vec<_> = document.select(&snippet_selector).collect();
    let mut results = Vec::new();

    for (index, link_element) in result_links.iter().enumerate() {
        if results.len() >= max_results as usize {
            break;
        }
        let Some(href) = link_element.value().attr("href") else {
            continue;
        };
        if href.contains("duckduckgo.com") || href.contains("google.com/search") {
            continue;
        }

        let url = normalize_url(href);
        let title = normalize_text(&link_element.inner_html());
        let snippet = result_snippets
            .get(index)
            .map(|snippet_elem| normalize_text(&snippet_elem.inner_html()));

        results.push(search_hit(
            url.clone(),
            title,
            snippet,
            extract_domain(&url),
            None,
            "duckduckgo",
            None,
        ));
    }

    Ok(results)
}

async fn google_search(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
    cx: &str,
    max_results: Option<u32>,
) -> Result<Vec<JsonValue>, String> {
    let url = google_search_url(api_key, cx, query, max_results)?;
    let payload = get_json(client, &url, "google").await?;
    let items = payload
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let url = item
            .get("link")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let snippet = item
            .get("snippet")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);
        let domain = item
            .get("displayLink")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
            .or_else(|| extract_domain(&url));
        let published_date = item
            .get("pagemap")
            .and_then(|v| v.get("metatags"))
            .and_then(|v| v.as_array())
            .and_then(|tags| tags.first())
            .and_then(|meta| {
                meta.get("article:published_time")
                    .or_else(|| meta.get("date"))
                    .or_else(|| meta.get("og:updated_time"))
            })
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);

        results.push(search_hit(
            url,
            title,
            snippet,
            domain,
            published_date,
            "google",
            Some(item),
        ));
    }
    Ok(results)
}

fn google_search_url(
    api_key: &str,
    cx: &str,
    query: &str,
    max_results: Option<u32>,
) -> Result<String, String> {
    let mut url = url::Url::parse("https://www.googleapis.com/customsearch/v1")
        .map_err(|err| format!("google url: {err}"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("key", api_key);
        pairs.append_pair("cx", cx);
        pairs.append_pair("q", query);
        if let Some(max_results) = max_results {
            // Custom Search returns at most 10 hits per request.
            pairs.append_pair("num", &max_results.min(10).to_string());
        }
        // websearch's SearchOptions default page is 1, which always sends start=1.
        pairs.append_pair("start", "1");
    }
    Ok(url.to_string())
}

async fn exa_search(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
    max_results: Option<u32>,
) -> Result<Vec<JsonValue>, String> {
    // Same body websearch 0.1.1's ExaProvider::new sent (keyword model, no page text).
    let body = json!({
        "query": query,
        "max_results": max_results,
        "model": "keyword",
        "include_contents": false,
    });
    let payload = post_json(client, "https://api.exa.ai/search", api_key, &body, "exa").await?;

    let items = payload
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let url = item
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let snippet = item
            .get("text")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);
        let published_date = item
            .get("publishedDate")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);

        let mut raw = serde_json::Map::new();
        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
            raw.insert("id".to_string(), JsonValue::String(id.to_string()));
        }
        if let Some(score) = item.get("score").cloned() {
            raw.insert("score".to_string(), score);
        }
        if let Some(author) = item.get("author").and_then(|v| v.as_str()) {
            raw.insert("author".to_string(), JsonValue::String(author.to_string()));
        }
        let raw = if raw.is_empty() {
            None
        } else {
            Some(JsonValue::Object(raw))
        };

        // Per-hit provider stays "exa": that is the value websearch serialized.
        // The response envelope still reports the Grapheme provider id "xaviv".
        results.push(search_hit(
            url.clone(),
            title,
            snippet,
            extract_domain(&url),
            published_date,
            "exa",
            raw,
        ));
    }
    Ok(results)
}

async fn tavily_search(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
    max_results: Option<u32>,
) -> Result<Vec<JsonValue>, String> {
    let max_results = max_results.unwrap_or(10).min(50);
    let body = json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "include_answer": true,
        "include_images": false,
        "include_raw_content": false,
        "max_results": max_results,
        "include_domains": JsonValue::Null,
        "exclude_domains": JsonValue::Null,
    });

    let response = client
        .post("https://api.tavily.com/search")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("tavily request failed: {err}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("tavily request failed: {err}"))?;
    if !status.is_success() {
        return Err(format!(
            "tavily API error {}: {}",
            status.as_u16(),
            text.trim()
        ));
    }
    let payload: JsonValue = serde_json::from_str(&text)
        .map_err(|err| format!("tavily response decode failed: {err}"))?;

    let items = payload
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let url = item
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let snippet = item
            .get("content")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);
        let published_date = item
            .get("published_date")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);
        results.push(search_hit(
            url.clone(),
            title,
            snippet,
            extract_domain(&url),
            published_date,
            "tavily",
            Some(item),
        ));
    }
    Ok(results)
}

async fn brave_search(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
    max_results: Option<u32>,
) -> Result<Vec<JsonValue>, String> {
    let count = max_results.unwrap_or(10).clamp(1, 20);
    let encoded_query = percent_encode_query(query);
    let url =
        format!("https://api.search.brave.com/res/v1/web/search?q={encoded_query}&count={count}");

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("X-Subscription-Token", api_key)
        .send()
        .await
        .map_err(|err| format!("brave request failed: {err}"))?;

    let status = response.status().as_u16();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| format!("brave request failed: {err}"))?;
    if status >= 400 {
        let body = String::from_utf8_lossy(&bytes);
        return Err(format!("brave API error {status}: {}", body.trim()));
    }

    let payload: JsonValue = serde_json::from_slice(&bytes)
        .map_err(|err| format!("brave response decode failed: {err}"))?;

    let results = payload
        .get("web")
        .and_then(|v| v.get("results"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|item| {
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let domain = url
                .split("://")
                .nth(1)
                .and_then(|rest| rest.split('/').next())
                .map(ToOwned::to_owned);
            json!({
                "url": url,
                "title": item.get("title").and_then(|v| v.as_str()).unwrap_or_default(),
                "snippet": item.get("description").and_then(|v| v.as_str()),
                "domain": domain,
                "provider": "brave",
            })
        })
        .collect::<Vec<_>>();
    Ok(results)
}

async fn get_json(
    client: &reqwest::Client,
    url: &str,
    provider: &str,
) -> Result<JsonValue, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("{provider} request failed: {err}"))?;
    read_json(response, provider).await
}

async fn post_json(
    client: &reqwest::Client,
    url: &str,
    bearer: &str,
    body: &JsonValue,
    provider: &str,
) -> Result<JsonValue, String> {
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {bearer}"))
        .json(body)
        .send()
        .await
        .map_err(|err| format!("{provider} request failed: {err}"))?;
    read_json(response, provider).await
}

async fn read_json(response: reqwest::Response, provider: &str) -> Result<JsonValue, String> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("{provider} request failed: {err}"))?;
    if !status.is_success() {
        return Err(format!(
            "{provider} API error {}: {}",
            status.as_u16(),
            text.trim()
        ));
    }
    serde_json::from_str(&text).map_err(|err| format!("{provider} response decode failed: {err}"))
}

fn search_hit(
    url: String,
    title: String,
    snippet: Option<String>,
    domain: Option<String>,
    published_date: Option<String>,
    provider: &str,
    raw: Option<JsonValue>,
) -> JsonValue {
    let mut hit = json!({
        "url": url,
        "title": title,
        "snippet": snippet,
        "domain": domain,
        "published_date": published_date,
        "provider": provider,
    });
    if let Some(raw) = raw {
        hit["raw"] = raw;
    }
    hit
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("//") {
        format!("https:{url}")
    } else if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{url}")
    } else {
        url.to_string()
    }
}

fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(ToOwned::to_owned))
}

fn percent_encode_query(query: &str) -> String {
    query
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            b' ' => "+".to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[derive(Debug, Clone)]
struct SearchRequest {
    query: String,
    provider: String,
    max_results: Option<u32>,
}

#[derive(Debug, Clone)]
struct SearchResponse {
    query: String,
    provider: String,
    results: JsonValue,
    error: Option<String>,
}

impl SearchResponse {
    fn success(query: String, provider: String, results: JsonValue) -> Self {
        Self {
            query,
            provider,
            results,
            error: None,
        }
    }

    fn failure(query: String, provider: String, error: String) -> Self {
        Self {
            query,
            provider,
            results: JsonValue::Array(Vec::new()),
            error: Some(error),
        }
    }

    fn to_json(&self) -> JsonValue {
        let count = self
            .results
            .as_array()
            .map(|items| items.len())
            .unwrap_or(0);
        if let Some(error) = &self.error {
            json!({
                "query": self.query,
                "provider": self.provider,
                "error": error,
                "results": self.results,
            })
        } else {
            json!({
                "query": self.query,
                "provider": self.provider,
                "count": count,
                "results": self.results,
            })
        }
    }
}

impl SearchRequest {
    fn from_args(args: &JsonValue) -> Self {
        Self {
            query: arg_text(args, "query"),
            provider: args
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("duckduckgo")
                .to_ascii_lowercase(),
            max_results: args
                .get("max_results")
                .and_then(|v| v.as_u64())
                .map(|v| v.min(20) as u32),
        }
    }

    fn with_provider(mut self, provider: &str) -> Self {
        self.provider = provider.to_ascii_lowercase();
        self
    }

    fn to_json(&self) -> JsonValue {
        json!({
            "query": self.query,
            "provider": self.provider,
            "max_results": self.max_results,
        })
    }
}

#[derive(Debug, Clone)]
struct WebProvider {
    id: &'static str,
    status: &'static str,
    supports_search: bool,
    supports_research_flow: bool,
    note: &'static str,
}

impl WebProvider {
    fn to_json(&self) -> JsonValue {
        json!({
            "id": self.id,
            "status": self.status,
            "supports": {
                "search": self.supports_search,
                "research_materials": self.supports_research_flow,
                "research_report": self.supports_research_flow,
            },
            "note": self.note,
        })
    }
}

fn provider_catalog() -> Vec<WebProvider> {
    vec![
        WebProvider {
            id: "duckduckgo",
            status: "available",
            supports_search: true,
            supports_research_flow: true,
            note: "No API key required.",
        },
        WebProvider {
            id: "google",
            status: "available",
            supports_search: true,
            supports_research_flow: true,
            note: "Requires GOOGLE_API_KEY and GOOGLE_CX.",
        },
        WebProvider {
            id: "xaviv",
            status: "available",
            supports_search: true,
            supports_research_flow: true,
            note: "Experimental semantic search via Exa; set XAVIV_API_KEY or EXA_API_KEY.",
        },
        WebProvider {
            id: "tavily",
            status: "available",
            supports_search: true,
            supports_research_flow: true,
            note: "AI-oriented search; requires TAVILY_API_KEY (tvly- prefix).",
        },
        WebProvider {
            id: "brave",
            status: "available",
            supports_search: true,
            supports_research_flow: true,
            note: "Brave Search API; requires BRAVE_API_KEY.",
        },
    ]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn search_rejects_unsupported_provider() {
        let out = search(&json!({
            "query": "rust",
            "provider": "bing"
        }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn google_provider_requires_credentials() {
        let prior_key = env::var("GOOGLE_API_KEY").ok();
        let prior_cx = env::var("GOOGLE_CX").ok();
        unsafe {
            env::remove_var("GOOGLE_API_KEY");
            env::remove_var("GOOGLE_CX");
        }

        let out = search_provider(&json!({ "query": "rust" }), "google");
        let err = out.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(err.contains("GOOGLE_API_KEY"));

        restore_env("GOOGLE_API_KEY", prior_key);
        restore_env("GOOGLE_CX", prior_cx);
    }

    #[test]
    fn xaviv_provider_requires_api_key() {
        let prior_xaviv = env::var("XAVIV_API_KEY").ok();
        let prior_exa = env::var("EXA_API_KEY").ok();
        unsafe {
            env::remove_var("XAVIV_API_KEY");
            env::remove_var("EXA_API_KEY");
        }

        let out = search_provider(&json!({ "query": "rust" }), "xaviv");
        let err = out.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(err.contains("XAVIV_API_KEY") || err.contains("EXA_API_KEY"));

        restore_env("XAVIV_API_KEY", prior_xaviv);
        restore_env("EXA_API_KEY", prior_exa);
    }

    #[test]
    fn tavily_provider_requires_api_key() {
        let prior = env::var("TAVILY_API_KEY").ok();
        unsafe {
            env::remove_var("TAVILY_API_KEY");
        }

        let out = search_provider(&json!({ "query": "rust" }), "tavily");
        let err = out.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(err.contains("TAVILY_API_KEY"));

        restore_env("TAVILY_API_KEY", prior);
    }

    #[test]
    fn tavily_provider_rejects_key_without_prefix() {
        assert!(tavily_key_format_error("tvly-test").is_none());
        let err = tavily_key_format_error("invalid-key").unwrap_or_default();
        assert!(err.contains("tvly-"));
    }

    #[test]
    fn brave_provider_requires_api_key() {
        let prior = env::var("BRAVE_API_KEY").ok();
        unsafe {
            env::remove_var("BRAVE_API_KEY");
        }

        let out = search_provider(&json!({ "query": "rust" }), "brave");
        let err = out.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(err.contains("BRAVE_API_KEY"));

        restore_env("BRAVE_API_KEY", prior);
    }

    #[test]
    fn providers_lists_known_catalog() {
        let out = providers();
        let providers = out
            .get("providers")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let ids = providers
            .iter()
            .filter_map(|item| item.get("id").and_then(|v| v.as_str()))
            .collect::<Vec<_>>();
        for expected in ["duckduckgo", "google", "xaviv", "tavily", "brave"] {
            assert!(ids.contains(&expected), "missing provider {expected}");
        }
    }

    #[test]
    fn capabilities_rejects_unknown_provider() {
        let out = capabilities(&json!({ "provider": "unknown" }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn duckduckgo_html_parser_keeps_result_shape() {
        let html = r#"
            <html><body>
              <div class="result">
                <h2 class="result__title">
                  <a href="https://example.com/rust">Example <b>Rust</b></a>
                </h2>
                <a class="result__snippet" href="https://example.com/rust">A <b>snippet</b> here</a>
              </div>
              <div class="result">
                <h2 class="result__title">
                  <a href="https://duckduckgo.com/l/?q=skip">Skip me</a>
                </h2>
                <a class="result__snippet">ignored</a>
              </div>
              <div class="result">
                <h2 class="result__title">
                  <a href="//cdn.example.org/a">Protocol relative</a>
                </h2>
                <a class="result__snippet">second hit</a>
              </div>
            </body></html>
        "#;

        let hits = parse_duckduckgo_html(html, 10).expect("parse");
        assert_eq!(hits.len(), 2);

        assert_eq!(hits[0]["url"], "https://example.com/rust");
        assert_eq!(hits[0]["title"], "Example <b>Rust</b>");
        assert_eq!(hits[0]["snippet"], "A <b>snippet</b> here");
        assert_eq!(hits[0]["domain"], "example.com");
        assert_eq!(hits[0]["provider"], "duckduckgo");
        assert!(hits[0].get("published_date").is_some());
        assert!(hits[0].get("raw").is_none());

        assert_eq!(hits[1]["url"], "https://cdn.example.org/a");
        assert_eq!(hits[1]["snippet"], "second hit");
        assert_eq!(hits[1]["domain"], "cdn.example.org");

        let limited = parse_duckduckgo_html(html, 1).expect("parse");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0]["url"], "https://example.com/rust");
    }

    #[test]
    fn google_search_url_matches_websearch_defaults() {
        let url = google_search_url("key", "cx-id", "rust async", Some(5)).expect("url");
        assert!(url.contains("key=key"));
        assert!(url.contains("cx=cx-id"));
        assert!(url.contains("q=rust+async") || url.contains("q=rust%20async"));
        assert!(url.contains("num=5"));
        assert!(url.contains("start=1"));
    }

    fn restore_env(key: &str, value: Option<String>) {
        match value {
            Some(value) => unsafe {
                env::set_var(key, value);
            },
            None => unsafe {
                env::remove_var(key);
            },
        }
    }
}
