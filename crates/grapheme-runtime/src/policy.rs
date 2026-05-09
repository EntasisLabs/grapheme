use serde_json::Value as JsonValue;

use crate::error::RuntimeError;
use crate::module_registry::ResolvedModuleCall;

#[derive(Debug, Clone, Default)]
pub struct PolicyGuard {
    pub allowed_http_domains: Vec<String>,
    pub allowed_tcp_targets: Vec<String>,
    pub allowed_smtp_domains: Vec<String>,
    pub allowed_secret_names: Vec<String>,
}

impl PolicyGuard {
    pub fn check(&self, call: &ResolvedModuleCall, args: &JsonValue) -> Result<(), RuntimeError> {
        match (call.module_id.as_str(), call.op.as_str()) {
            ("http", "get") | ("http", "post") => self.check_http(args),
            ("tcp", "connect") => self.check_tcp(args),
            ("smtp", "send_mail") => self.check_smtp(args),
            ("secrets", "get_secret_handle") | ("secrets", "sign_request") => {
                self.check_secrets(args)
            }
            _ => Ok(()),
        }
    }

    fn check_http(&self, args: &JsonValue) -> Result<(), RuntimeError> {
        if self.allowed_http_domains.is_empty() {
            return Ok(());
        }

        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::RuntimeError("policy: missing http url arg".to_string()))?;
        let host = extract_host(url)
            .ok_or_else(|| RuntimeError::RuntimeError(format!("policy: invalid http url '{url}'")))?;

        if self.allowed_http_domains.iter().any(|d| d == &host) {
            Ok(())
        } else {
            Err(RuntimeError::RuntimeError(format!(
                "policy: http domain '{}' is not allowed",
                host
            )))
        }
    }

    fn check_tcp(&self, args: &JsonValue) -> Result<(), RuntimeError> {
        if self.allowed_tcp_targets.is_empty() {
            return Ok(());
        }

        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::RuntimeError("policy: missing tcp target arg".to_string()))?;

        if self.allowed_tcp_targets.iter().any(|t| t == target) {
            Ok(())
        } else {
            Err(RuntimeError::RuntimeError(format!(
                "policy: tcp target '{}' is not allowed",
                target
            )))
        }
    }

    fn check_smtp(&self, args: &JsonValue) -> Result<(), RuntimeError> {
        if self.allowed_smtp_domains.is_empty() {
            return Ok(());
        }

        let to = args
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::RuntimeError("policy: missing smtp to arg".to_string()))?;
        let domain = to
            .split('@')
            .nth(1)
            .ok_or_else(|| RuntimeError::RuntimeError(format!("policy: invalid smtp recipient '{to}'")))?
            .to_string();

        if self.allowed_smtp_domains.iter().any(|d| d == &domain) {
            Ok(())
        } else {
            Err(RuntimeError::RuntimeError(format!(
                "policy: smtp domain '{}' is not allowed",
                domain
            )))
        }
    }

    fn check_secrets(&self, args: &JsonValue) -> Result<(), RuntimeError> {
        if self.allowed_secret_names.is_empty() {
            return Ok(());
        }

        let name = args
            .get("name")
            .or_else(|| args.get("secret"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::RuntimeError("policy: missing secrets name arg".to_string()))?;

        if self.allowed_secret_names.iter().any(|s| s == name) {
            Ok(())
        } else {
            Err(RuntimeError::RuntimeError(format!(
                "policy: secret '{}' is not allowed",
                name
            )))
        }
    }
}

fn extract_host(url: &str) -> Option<String> {
    let after_scheme = if let Some((_, rest)) = url.split_once("://") {
        rest
    } else {
        url
    };

    let host_port = after_scheme.split('/').next()?;
    let host = host_port.split(':').next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}