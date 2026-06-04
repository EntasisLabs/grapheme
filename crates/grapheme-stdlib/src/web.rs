use serde_json::{json, Value as JsonValue};
use std::env;
use std::time::Duration;
use websearch::{
    providers::{DuckDuckGoProvider, ExaProvider, GoogleProvider, TavilyProvider},
    types::SearchProvider,
    web_search, SearchOptions,
};

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

fn run_search(query: String, provider: String, max_results: Option<u32>) -> JsonValue {
    if provider == "brave" {
        return brave_search(query, max_results);
    }

    let provider_impl = match build_search_provider(&provider) {
        Ok(provider_impl) => provider_impl,
        Err(err) => {
            return SearchResponse::failure(query, provider, err).to_json();
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            return json!({ "error": format!("websearch runtime init failed: {err}") });
        }
    };

    let search_result = runtime.block_on(async {
        web_search(SearchOptions {
            query: query.clone(),
            max_results,
            provider: provider_impl,
            ..Default::default()
        })
        .await
    });

    match search_result {
        Ok(results) => {
            let results_json =
                serde_json::to_value(results).unwrap_or_else(|_| JsonValue::Array(Vec::new()));
            SearchResponse::success(query, provider, results_json).to_json()
        }
        Err(err) => SearchResponse::failure(query, provider, err.to_string()).to_json(),
    }
}

fn build_search_provider(
    provider_id: &str,
) -> Result<Box<dyn SearchProvider>, String> {
    match provider_id {
        "duckduckgo" => Ok(Box::new(DuckDuckGoProvider::new())),
        "google" => {
            let api_key = env::var("GOOGLE_API_KEY")
                .map_err(|_| "missing GOOGLE_API_KEY environment variable".to_string())?;
            let cx = env::var("GOOGLE_CX")
                .map_err(|_| "missing GOOGLE_CX environment variable".to_string())?;
            GoogleProvider::new(&api_key, &cx)
                .map_err(|err| err.to_string())
                .map(|provider| Box::new(provider) as Box<dyn SearchProvider>)
        }
        "xaviv" => {
            let api_key = env::var("XAVIV_API_KEY")
                .or_else(|_| env::var("EXA_API_KEY"))
                .map_err(|_| {
                    "missing XAVIV_API_KEY or EXA_API_KEY environment variable".to_string()
                })?;
            ExaProvider::new(&api_key)
                .map_err(|err| err.to_string())
                .map(|provider| Box::new(provider) as Box<dyn SearchProvider>)
        }
        "tavily" => {
            let api_key = env::var("TAVILY_API_KEY")
                .map_err(|_| "missing TAVILY_API_KEY environment variable".to_string())?;
            TavilyProvider::new(&api_key)
                .map_err(|err| err.to_string())
                .map(|provider| Box::new(provider) as Box<dyn SearchProvider>)
        }
        other => Err(format!(
            "unsupported websearch provider '{other}'; supported: duckduckgo, google, xaviv, tavily, brave"
        )),
    }
}

fn brave_search(query: String, max_results: Option<u32>) -> JsonValue {
    let provider = "brave".to_string();
    let api_key = match env::var("BRAVE_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            return SearchResponse::failure(
                query,
                provider,
                "missing BRAVE_API_KEY environment variable".to_string(),
            )
            .to_json();
        }
    };

    let count = max_results.unwrap_or(10).clamp(1, 20);
    let encoded_query = percent_encode_query(&query);
    let url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={encoded_query}&count={count}"
    );

    let mut request = ehttp::Request::get(&url);
    request
        .headers
        .insert("Accept", "application/json");
    request
        .headers
        .insert("X-Subscription-Token", api_key.as_str());
    request.timeout = Some(Duration::from_secs(15));

    let response = match ehttp::fetch_blocking(&request) {
        Ok(response) => response,
        Err(err) => {
            return SearchResponse::failure(query, provider, format!("brave request failed: {err}"))
                .to_json();
        }
    };

    if response.status >= 400 {
        let body = String::from_utf8_lossy(&response.bytes);
        return SearchResponse::failure(
            query,
            provider,
            format!("brave API error {}: {}", response.status, body.trim()),
        )
        .to_json();
    }

    let payload: JsonValue = match serde_json::from_slice(&response.bytes) {
        Ok(payload) => payload,
        Err(err) => {
            return SearchResponse::failure(
                query,
                provider,
                format!("brave response decode failed: {err}"),
            )
            .to_json();
        }
    };

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

    SearchResponse::success(query, provider, JsonValue::Array(results)).to_json()
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
