use serde_json::{json, Value as JsonValue};
use std::time::Duration;

pub fn request(method: &str, url: &str, body: Option<&JsonValue>) -> JsonValue {
    let request = HttpRequest::from_parts(method, url, body.cloned());
    execute(&request).to_json()
}

#[derive(Debug, Clone)]
struct HttpRequest {
    method: String,
    url: String,
    body: Option<JsonValue>,
}

impl HttpRequest {
    fn from_parts(method: &str, url: &str, body: Option<JsonValue>) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            body,
        }
    }
}

#[derive(Debug, Clone)]
struct HttpResponse {
    method: String,
    url: String,
    status: Option<u16>,
    status_line: Option<String>,
    body: Option<String>,
    error: Option<String>,
}

impl HttpResponse {
    fn error(method: &str, url: &str, error: String) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            status: None,
            status_line: None,
            body: None,
            error: Some(error),
        }
    }

    fn to_json(&self) -> JsonValue {
        if let Some(error) = &self.error {
            return json!({
                "error": error,
                "method": self.method,
                "url": self.url,
            });
        }

        json!({
            "method": self.method,
            "url": self.url,
            "status": self.status,
            "status_line": self.status_line,
            "body": self.body,
        })
    }
}

fn execute(request: &HttpRequest) -> HttpResponse {
    if !request.url.starts_with("http://") && !request.url.starts_with("https://") {
        return HttpResponse::error(
            &request.method,
            &request.url,
            "host http adapter supports only http:// and https:// URLs".to_string(),
        );
    }

    let payload = request
        .body
        .as_ref()
        .map(|b| serde_json::to_vec(b).unwrap_or_else(|_| b"null".to_vec()))
        .unwrap_or_default();

    let mut ehttp_request = if request.method == "POST" {
        let mut req = ehttp::Request::post(&request.url, payload);
        req.headers.insert("Content-Type", "application/json");
        req
    } else {
        ehttp::Request::get(&request.url)
    };
    ehttp_request.timeout = Some(Duration::from_secs(15));

    let response = match ehttp::fetch_blocking(&ehttp_request) {
        Ok(resp) => resp,
        Err(err) => {
            return HttpResponse::error(
                &request.method,
                &request.url,
                format!("request failed: {err}"),
            );
        }
    };

    let response_body = String::from_utf8_lossy(&response.bytes).to_string();
    let status_line = format!("HTTP {} {}", response.status, response.status_text);

    HttpResponse {
        method: request.method.clone(),
        url: request.url.clone(),
        status: Some(response.status),
        status_line: Some(status_line),
        body: Some(response_body),
        error: None,
    }
}
