use serde_json::{json, Value as JsonValue};
use websearch::{providers::DuckDuckGoProvider, web_search, SearchOptions};

pub fn search(args: &JsonValue) -> JsonValue {
    let request = SearchRequest::from_args(args);
    if request.query.trim().is_empty() {
        return json!({ "error": "missing required arg: query" });
    }

    let provider = request.provider.clone();

    if provider != "duckduckgo" {
        return json!({
            "error": format!("unsupported websearch provider '{}'; currently supported: duckduckgo", provider)
        });
    }

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
        let provider = DuckDuckGoProvider::new();
        web_search(SearchOptions {
            query: request.query.clone(),
            max_results: request.max_results,
            provider: Box::new(provider),
            ..Default::default()
        })
        .await
    });

    match search_result {
        Ok(results) => {
            let results_json = serde_json::to_value(results)
                .unwrap_or_else(|_| JsonValue::Array(Vec::new()));
            SearchResponse::success(request.query, provider, results_json).to_json()
        }
        Err(err) => SearchResponse::failure(request.query, provider, err.to_string()).to_json(),
    }
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

    let Some(provider) = provider_catalog()
        .into_iter()
        .find(|p| p.id == target)
    else {
        return json!({
            "error": format!("unknown provider '{}'", target),
            "available_providers": provider_catalog().into_iter().map(|p| p.id).collect::<Vec<_>>()
        });
    };

    json!({ "provider": provider.to_json() })
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
            note: "Native provider is wired and active.",
        },
        WebProvider {
            id: "google",
            status: "planned",
            supports_search: false,
            supports_research_flow: false,
            note: "Provider namespace is reserved; backend not yet wired.",
        },
        WebProvider {
            id: "xaviv",
            status: "planned",
            supports_search: false,
            supports_research_flow: false,
            note: "Provider namespace is reserved; backend not yet wired.",
        },
    ]
}

fn arg_text(args: &JsonValue, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| args.get("__input").and_then(|v| v.as_str()).map(ToOwned::to_owned))
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
            "provider": "google"
        }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn search_provider_alias_routes_semantics() {
        let out = search_provider(&json!({ "query": "rust" }), "xaviv");
        let err = out.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(err.contains("unsupported websearch provider 'xaviv'"));
    }

    #[test]
    fn providers_lists_known_catalog() {
        let out = providers();
        let providers = out
            .get("providers")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        assert!(providers.iter().any(|item| {
            item.get("id")
                .and_then(|v| v.as_str())
                .map(|id| id == "duckduckgo")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn capabilities_rejects_unknown_provider() {
        let out = capabilities(&json!({ "provider": "unknown" }));
        assert!(out.get("error").and_then(|v| v.as_str()).is_some());
    }
}
