use anyhow::{anyhow, Context};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{de::DeserializeOwned, Serialize};
use std::{thread, time::Duration};
use takokit_core::{DaemonIdentity, DaemonMode, RuntimeConfig};
use takokit_store::LocalStore;

use crate::workspace::{SESSION_ENV, WORKSPACE_ENV};

pub struct Client {
    base: String,
    workspace_header: Option<String>,
    session_header: Option<String>,
}

impl Client {
    pub fn ensure(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<Self> {
        let info = crate::daemon::ensure_running(store, config)?;
        let url = format!("{}/v1/daemon/identity", config.local_base_url());
        let identity: DaemonIdentity = ureq::get(&url)
            .call()
            .map_err(|error| request_error("GET", &url, error))?
            .into_json()
            .with_context(|| format!("decode daemon identity response from {url}"))?;
        if identity.mode != DaemonMode::Managed || identity.instance_id != Some(info.instance_id) {
            return Err(anyhow!("managed daemon identity verification failed"));
        }
        Ok(Self {
            base: config.local_base_url(),
            workspace_header: std::env::var(WORKSPACE_ENV)
                .ok()
                .map(|value| utf8_percent_encode(&value, NON_ALPHANUMERIC).to_string()),
            session_header: std::env::var(SESSION_ENV).ok(),
        })
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base, path);
        self.headers(ureq::get(&url))
            .call()
            .map_err(|error| request_error("GET", &url, error))?
            .into_json()
            .with_context(|| format!("decode Takokit daemon response from {path}"))
    }

    pub fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        self.post_with_attempts(path, body, 1)
    }

    pub fn post_retryable<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        self.post_with_attempts(path, body, 3)
    }

    pub fn delete(&self, path: &str) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base, path);
        self.headers(ureq::delete(&url))
            .call()
            .map_err(|error| request_error("DELETE", &url, error))?;
        Ok(())
    }

    fn post_with_attempts<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
        attempts: usize,
    ) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base, path);
        let payload = serde_json::to_value(body)?;
        let attempts = attempts.max(1);

        for attempt in 1..=attempts {
            let response = self
                .headers(ureq::post(&url))
                .send_json(payload.clone());
            match response {
                Ok(response) => {
                    return response
                        .into_json()
                        .with_context(|| format!("decode Takokit daemon response from {path}"));
                }
                Err(ureq::Error::Status(status, response)) => {
                    let body = response.into_string().unwrap_or_default();
                    let message = format_status_error(status, &body);
                    if attempt < attempts && retryable_status(status) {
                        thread::sleep(retry_delay(attempt));
                        continue;
                    }
                    return Err(anyhow!(message));
                }
                Err(ureq::Error::Transport(error)) => {
                    let message = format!(
                        "POST {url}: could not contact Takokit daemon: {error}"
                    );
                    if attempt < attempts {
                        thread::sleep(retry_delay(attempt));
                        continue;
                    }
                    return Err(anyhow!(message));
                }
            }
        }

        Err(anyhow!("POST {url} failed without a response"))
    }

    fn headers(&self, mut request: ureq::Request) -> ureq::Request {
        if let Some(workspace) = &self.workspace_header {
            request = request.set("X-Takokit-Workspace", workspace);
        }
        if let Some(session) = &self.session_header {
            request = request.set("X-Takokit-Session", session);
        }
        request
    }
}

fn request_error(method: &str, url: &str, error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(status, response) => {
            let body = response.into_string().unwrap_or_default();
            anyhow!(format_status_error(status, &body))
        }
        ureq::Error::Transport(error) => {
            anyhow!("{method} {url}: could not contact Takokit daemon: {error}")
        }
    }
}

fn retryable_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(500 * (1_u64 << attempt.saturating_sub(1).min(4)))
}

fn format_status_error(status: u16, body: &str) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let code = parsed
        .as_ref()
        .and_then(|value| value.pointer("/error/code"))
        .and_then(serde_json::Value::as_str);
    let message = parsed
        .as_ref()
        .and_then(|value| value.pointer("/error/message"))
        .and_then(serde_json::Value::as_str);
    let retryable = parsed
        .as_ref()
        .and_then(|value| value.pointer("/error/retryable"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or_else(|| retryable_status(status));

    let suffix = if retryable {
        " (temporary; Takokit retried automatically)"
    } else {
        ""
    };
    match (code, message) {
        (Some(code), Some(message)) => format!("{code}: {message}{suffix}"),
        (None, Some(message)) => format!("{message}{suffix}"),
        _ if !body.trim().is_empty() => {
            format!("daemon returned HTTP {status}: {}{suffix}", body.trim())
        }
        _ => format!("daemon returned HTTP {status} without an error message{suffix}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{format_status_error, retryable_status};

    #[test]
    fn structured_api_error_preserves_code_and_message() {
        let body = r#"{"error":{"code":"artifact_download_failed","message":"runner runtime: download failed","retryable":true}}"#;
        assert_eq!(
            format_status_error(502, body),
            "artifact_download_failed: runner runtime: download failed (temporary; Takokit retried automatically)"
        );
    }

    #[test]
    fn non_json_api_error_keeps_status_and_body() {
        assert_eq!(
            format_status_error(400, "bad request"),
            "daemon returned HTTP 400: bad request"
        );
    }

    #[test]
    fn retries_only_transient_http_statuses() {
        assert!(retryable_status(408));
        assert!(retryable_status(429));
        assert!(retryable_status(502));
        assert!(retryable_status(503));
        assert!(!retryable_status(400));
        assert!(!retryable_status(404));
    }
}
