use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::process::{Command, Stdio};

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub channel: Option<String>,
    pub automatic_checks: Option<bool>,
    pub automatic_download: Option<bool>,
}

pub async fn update_status() -> (StatusCode, Json<Value>) {
    command_response(vec!["update".into(), "status".into()]).await
}

pub async fn update_check() -> (StatusCode, Json<Value>) {
    command_response(vec!["update".into(), "check".into()]).await
}

pub async fn update_apply() -> (StatusCode, Json<Value>) {
    let status = match run_json_command(&["update", "status"]).await {
        Ok(status) => status,
        Err(error) => return cli_error_response(error),
    };
    if status.get("distribution_mode").and_then(Value::as_str) != Some("installed") {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "error": {
                    "code": "update_not_installed_distribution",
                    "message": "self-update is available only from an installed Takokit distribution"
                }
            })),
        );
    }

    match spawn_update_apply() {
        Ok(()) => (
            StatusCode::ACCEPTED,
            Json(json!({
                "accepted": true,
                "message": "Verified update installation was requested. Takokit will refuse active work and restart only through the updater helper."
            })),
        ),
        Err(error) => cli_error_response(error),
    }
}

pub async fn update_settings(
    Json(request): Json<UpdateSettingsRequest>,
) -> (StatusCode, Json<Value>) {
    if request.channel.is_none()
        && request.automatic_checks.is_none()
        && request.automatic_download.is_none()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "code": "invalid_request",
                    "message": "update settings request did not contain any changes"
                }
            })),
        );
    }
    if request
        .channel
        .as_deref()
        .is_some_and(|channel| !matches!(channel, "stable" | "preview"))
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "code": "invalid_request",
                    "message": "update channel must be stable or preview"
                }
            })),
        );
    }

    let result = tokio::task::spawn_blocking(move || -> Result<Value, String> {
        if let Some(channel) = request.channel.as_deref() {
            run_json_command_blocking(&["update", "channel", channel])?;
        }
        let mut configure = vec!["update".to_string(), "configure".to_string()];
        if let Some(enabled) = request.automatic_checks {
            configure.push("--automatic-checks".to_string());
            configure.push(if enabled { "on" } else { "off" }.to_string());
        }
        if let Some(enabled) = request.automatic_download {
            configure.push("--automatic-download".to_string());
            configure.push(if enabled { "on" } else { "off" }.to_string());
        }
        if configure.len() > 2 {
            let borrowed = configure.iter().map(String::as_str).collect::<Vec<_>>();
            run_json_command_blocking(&borrowed)?;
        }
        run_json_command_blocking(&["update", "status"])
    })
    .await;

    match result {
        Ok(Ok(value)) => (StatusCode::OK, Json(value)),
        Ok(Err(error)) => cli_error_response(error),
        Err(error) => cli_error_response(format!("update settings task failed: {error}")),
    }
}

async fn command_response(args: Vec<String>) -> (StatusCode, Json<Value>) {
    let result = tokio::task::spawn_blocking(move || {
        let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
        run_json_command_blocking(&borrowed)
    })
    .await;
    match result {
        Ok(Ok(value)) => (StatusCode::OK, Json(value)),
        Ok(Err(error)) => cli_error_response(error),
        Err(error) => cli_error_response(format!("update command task failed: {error}")),
    }
}

async fn run_json_command(args: &[&str]) -> Result<Value, String> {
    let args = args
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    tokio::task::spawn_blocking(move || {
        let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
        run_json_command_blocking(&borrowed)
    })
    .await
    .map_err(|error| format!("update command task failed: {error}"))?
}

fn run_json_command_blocking(args: &[&str]) -> Result<Value, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not locate the running Takokit executable: {error}"))?;
    let mut command = Command::new(executable);
    command.args(["--output", "json"]).args(args);
    hide_window(&mut command);
    let output = command
        .output()
        .map_err(|error| format!("could not launch Takokit update command: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Takokit update command returned invalid JSON: {error}"))
}

fn spawn_update_apply() -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not locate the running Takokit executable: {error}"))?;
    let mut command = Command::new(executable);
    command
        .args(["--output", "json", "update", "apply"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_window(&mut command);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not launch Takokit updater: {error}"))
}

fn cli_error_response(error: String) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({
            "error": {
                "code": "update_command_failed",
                "message": error
            }
        })),
    )
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn hide_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_payload_rejects_unknown_channel() {
        let request: UpdateSettingsRequest =
            serde_json::from_str(r#"{"channel":"nightly","automatic_checks":true}"#).unwrap();
        assert_eq!(request.channel.as_deref(), Some("nightly"));
        assert!(request.automatic_checks.unwrap());
    }
}
