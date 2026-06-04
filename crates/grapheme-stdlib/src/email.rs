use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde_json::{json, Value as JsonValue};
use std::env;

pub fn send_provider(args: &JsonValue, provider: &str) -> JsonValue {
    let request = match EmailSendRequest::from_args(args, provider) {
        Ok(request) => request,
        Err(err) => return json!({ "accepted": false, "error": err }),
    };

    if request.to.is_empty() {
        return json!({ "accepted": false, "error": "missing email recipient: to" });
    }

    match send_with_lettre(&request) {
        Ok(()) => json!({
            "accepted": true,
            "provider": request.provider,
            "server": request.server_label(),
            "from": request.from,
            "to": request.to,
            "subject": request.subject,
        }),
        Err(err) => json!({
            "accepted": false,
            "provider": request.provider,
            "server": request.server_label(),
            "error": err,
        }),
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportSecurity {
    Plain,
    StartTls,
    Tls,
}

#[derive(Debug, Clone)]
struct EmailSendRequest {
    provider: String,
    to: String,
    from: String,
    subject: String,
    body: String,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    security: TransportSecurity,
}

impl EmailSendRequest {
    fn from_args(args: &JsonValue, provider: &str) -> Result<Self, String> {
        let provider = provider.to_ascii_lowercase();
        let defaults = provider_defaults(&provider);

        let host = arg_str(args, "host")
            .or_else(|| arg_str(args, "server"))
            .or_else(|| env::var(defaults.host_env).ok())
            .unwrap_or_else(|| defaults.host.to_string());
        let port = arg_u16(args, "port")
            .or_else(|| env::var(defaults.port_env).ok().and_then(|v| v.parse().ok()))
            .unwrap_or(defaults.port);
        let username = arg_str(args, "username")
            .or_else(|| env::var(defaults.username_env).ok());
        let password = arg_str(args, "password")
            .or_else(|| env::var(defaults.password_env).ok());
        let security = arg_security(args).unwrap_or(defaults.security);

        Ok(Self {
            provider,
            to: arg_str(args, "to").unwrap_or_default(),
            from: arg_str(args, "from")
                .or_else(|| env::var("EMAIL_FROM").ok())
                .unwrap_or_else(|| defaults.from.to_string()),
            subject: arg_str(args, "subject").unwrap_or_else(|| "(no subject)".to_string()),
            body: arg_str(args, "body").unwrap_or_else(|| "(empty body)".to_string()),
            host,
            port,
            username,
            password,
            security,
        })
    }

    fn server_label(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

struct ProviderDefaults {
    host: &'static str,
    port: u16,
    from: &'static str,
    security: TransportSecurity,
    host_env: &'static str,
    port_env: &'static str,
    username_env: &'static str,
    password_env: &'static str,
}

fn provider_defaults(provider: &str) -> ProviderDefaults {
    match provider {
        "gmail" => ProviderDefaults {
            host: "smtp.gmail.com",
            port: 587,
            from: "grapheme@localhost",
            security: TransportSecurity::StartTls,
            host_env: "GMAIL_SMTP_HOST",
            port_env: "GMAIL_SMTP_PORT",
            username_env: "GMAIL_USERNAME",
            password_env: "GMAIL_APP_PASSWORD",
        },
        _ => ProviderDefaults {
            host: "127.0.0.1",
            port: 25,
            from: "grapheme@localhost",
            security: TransportSecurity::Plain,
            host_env: "SMTP_HOST",
            port_env: "SMTP_PORT",
            username_env: "SMTP_USERNAME",
            password_env: "SMTP_PASSWORD",
        },
    }
}

fn send_with_lettre(request: &EmailSendRequest) -> Result<(), String> {
    let from: Mailbox = request
        .from
        .parse()
        .map_err(|err| format!("invalid from address: {err}"))?;
    let to: Mailbox = request
        .to
        .parse()
        .map_err(|err| format!("invalid to address: {err}"))?;

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(request.subject.clone())
        .header(ContentType::TEXT_PLAIN)
        .body(request.body.clone())
        .map_err(|err| format!("email build failed: {err}"))?;

    let mailer = build_mailer(request)?;

    mailer
        .send(&email)
        .map_err(|err| format!("email send failed: {err}"))?;
    Ok(())
}

fn build_mailer(request: &EmailSendRequest) -> Result<SmtpTransport, String> {
    let creds = match (&request.username, &request.password) {
        (Some(username), Some(password)) => {
            Some(Credentials::new(username.clone(), password.clone()))
        }
        _ => None,
    };

    let mailer = match request.security {
        TransportSecurity::StartTls => SmtpTransport::starttls_relay(&request.host)
            .map_err(|err| format!("smtp starttls relay failed: {err}"))?
            .port(request.port),
        TransportSecurity::Tls => SmtpTransport::relay(&request.host)
            .map_err(|err| format!("smtp relay failed: {err}"))?
            .port(request.port),
        TransportSecurity::Plain => SmtpTransport::builder_dangerous(&request.host).port(request.port),
    };

    Ok(if let Some(creds) = creds {
        mailer.credentials(creds).build()
    } else {
        mailer.build()
    })
}

#[derive(Debug, Clone)]
struct EmailProvider {
    id: &'static str,
    status: &'static str,
    note: &'static str,
}

impl EmailProvider {
    fn to_json(&self) -> JsonValue {
        json!({
            "id": self.id,
            "status": self.status,
            "note": self.note,
        })
    }
}

fn provider_catalog() -> Vec<EmailProvider> {
    vec![
        EmailProvider {
            id: "smtp",
            status: "available",
            note: "Generic SMTP via lettre; supports host/port/username/password and SMTP_* env vars.",
        },
        EmailProvider {
            id: "gmail",
            status: "available",
            note: "Gmail preset (smtp.gmail.com:587 STARTTLS); set GMAIL_USERNAME and GMAIL_APP_PASSWORD.",
        },
    ]
}

fn arg_str(args: &JsonValue, key: &str) -> Option<String> {
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
}

fn arg_u16(args: &JsonValue, key: &str) -> Option<u16> {
    args.get(key)
        .and_then(|v| v.as_u64().map(|n| n as u16))
        .or_else(|| {
            args.get("__input")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(key))
                .and_then(|v| v.as_u64().map(|n| n as u16))
        })
}

fn arg_security(args: &JsonValue) -> Option<TransportSecurity> {
    let raw = arg_str(args, "security")
        .or_else(|| arg_str(args, "tls"))
        .map(|s| s.to_ascii_lowercase())?;

    match raw.as_str() {
        "plain" | "none" | "false" | "0" => Some(TransportSecurity::Plain),
        "starttls" | "start_tls" | "tls" | "true" | "1" => Some(TransportSecurity::StartTls),
        "ssl" | "smtps" => Some(TransportSecurity::Tls),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn send_provider_requires_recipient() {
        let out = send_provider(&json!({ "subject": "hi" }), "smtp");
        assert_eq!(out.get("accepted").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn capabilities_lists_smtp_and_gmail() {
        let out = capabilities(&json!({}));
        let count = out.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(count, 2);
    }

    #[test]
    fn gmail_defaults_to_starttls_host() {
        let req = EmailSendRequest::from_args(&json!({ "to": "a@example.com" }), "gmail")
            .expect("gmail request should parse");
        assert_eq!(req.host, "smtp.gmail.com");
        assert_eq!(req.port, 587);
        assert_eq!(req.security, TransportSecurity::StartTls);
    }
}
