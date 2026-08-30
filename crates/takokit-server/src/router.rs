use anyhow::Context;
use axum::{
    body::Body,
    extract::DefaultBodyLimit,
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

use crate::{handlers, AppState};

const WORKSPACE_UPLOAD_BODY_LIMIT_BYTES: usize = 100 * 1024 * 1024;

pub fn server_router(state: AppState) -> Router {
    let legacy = Router::new()
        .route("/status", get(handlers::status))
        .route("/daemon/identity", get(handlers::daemon_identity))
        .route("/daemon/shutdown", post(handlers::daemon_shutdown))
        .route("/ps", get(handlers::ps))
        .route("/doctor", get(handlers::doctor))
        .route("/capabilities", get(handlers::capabilities))
        .route("/models/installed", get(handlers::installed_models))
        .route("/models/:id/plan", get(handlers::model_plan))
        .route("/models/:id/progress", get(handlers::model_pull_progress))
        .route("/models/pull", post(handlers::model_pull_with_progress))
        .route("/runners", get(handlers::runners))
        .route("/voices", get(handlers::voices))
        .route("/audio/conversions", post(handlers::convert_voice))
        .route("/voices/clone", post(handlers::clone_voice))
        .route("/sessions", get(handlers::sessions));

    Router::new()
        .route("/health", get(handlers::health))
        .route("/openapi.json", get(handlers::openapi))
        .route("/v1/models", get(handlers::openai_models))
        .route("/v1/models/:id", get(handlers::openai_model))
        .route("/v1/audio/speech", post(handlers::openai_speech))
        .route(
            "/v1/audio/transcriptions",
            post(handlers::openai_transcription).layer(DefaultBodyLimit::max(26 * 1024 * 1024)),
        )
        .nest("/v1", legacy)
        .nest("/api/v1", native_router())
        .nest_service("/gui", gui_service())
        .layer(middleware::from_fn(request_trace))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            local_security,
        ))
        .with_state(state)
}

async fn request_trace(request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let route = request.uri().path().to_string();
    let started = Instant::now();
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).expect("UUID is a valid header value"),
    );
    tracing::info!(
        request_id,
        %method,
        route,
        status = response.status().as_u16(),
        duration_ms = started.elapsed().as_millis() as u64,
        "local API request"
    );
    response
}

async fn local_security(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let is_loopback = state
        .config
        .host
        .parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_loopback());
    if !is_loopback {
        let expected = std::env::var("TAKOKIT_API_TOKEN").unwrap_or_default();
        let supplied = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .unwrap_or_default();
        if expected.len() < 24 || supplied != expected {
            return security_error(StatusCode::UNAUTHORIZED, "invalid_api_key");
        }
    } else {
        if let Some(host) = request
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
        {
            let name = host
                .split(':')
                .next()
                .unwrap_or(host)
                .trim_matches(['[', ']']);
            if !matches!(name, "127.0.0.1" | "::1" | "localhost") {
                return security_error(StatusCode::FORBIDDEN, "invalid_host");
            }
        }
        if let Some(origin) = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
        {
            let allowed = [
                format!("http://127.0.0.1:{}", state.config.port),
                format!("http://localhost:{}", state.config.port),
                format!("http://[::1]:{}", state.config.port),
            ];
            if !allowed.iter().any(|value| value == origin) {
                return security_error(StatusCode::FORBIDDEN, "origin_not_allowed");
            }
        }
    }
    next.run(request).await
}

fn security_error(status: StatusCode, code: &'static str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": {
                "message": "Request rejected by Takokit local API security policy.",
                "type": "invalid_request_error",
                "param": null,
                "code": code
            }
        })),
    )
        .into_response()
}

fn native_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/status", get(handlers::status))
        .route("/daemon/identity", get(handlers::daemon_identity))
        .route("/daemon/shutdown", post(handlers::daemon_shutdown))
        .route("/ps", get(handlers::ps))
        .route("/doctor", get(handlers::doctor))
        .route("/system/picker/audio", get(handlers::pick_audio_file))
        .route("/system/picker/folder", get(handlers::pick_folder))
        .route("/system/picker/rvc", get(handlers::pick_rvc_artifact))
        .route("/system/audio", get(handlers::local_audio))
        .route("/system/storage", get(handlers::storage_overview))
        .route("/system/update", get(handlers::update_status))
        .route("/system/update/check", post(handlers::update_check))
        .route("/system/update/apply", post(handlers::update_apply))
        .route("/system/update/settings", post(handlers::update_settings))
        .route("/system/open", post(handlers::open_location))
        .route(
            "/files",
            get(handlers::workspace_files)
                .post(handlers::upload_workspace_file)
                .layer(DefaultBodyLimit::max(WORKSPACE_UPLOAD_BODY_LIMIT_BYTES)),
        )
        .route("/files/:id/content", get(handlers::workspace_file_content))
        .route(
            "/files/:id",
            axum::routing::delete(handlers::delete_workspace_file),
        )
        .route("/test/launch", get(handlers::launch_test))
        .route("/capabilities", get(handlers::capabilities))
        .route("/models", get(handlers::models))
        .route("/models/installed", get(handlers::installed_models))
        .route("/library/models", get(handlers::library_models))
        .route("/library/runners", get(handlers::library_runners))
        .route("/models/:id/plan", get(handlers::model_plan))
        .route("/models/:id/progress", get(handlers::model_pull_progress))
        .route(
            "/models/:id",
            get(handlers::model).delete(handlers::remove_model),
        )
        .route("/runners", get(handlers::runners))
        .route("/adapters", get(handlers::adapters))
        .route("/adapters/install", post(handlers::install_adapter))
        .route("/adapters/:id/doctor", get(handlers::adapter_doctor))
        .route("/adapters/:id", get(handlers::adapter))
        .route("/models/pull", post(handlers::model_pull_with_progress))
        .route("/runners/pull", post(handlers::pull_runner))
        .route("/runners/install", post(handlers::install_runner))
        .route("/runners/:id/doctor", get(handlers::runner_doctor))
        .route(
            "/runners/:id",
            get(handlers::runner).delete(handlers::remove_runner),
        )
        .route("/voices", get(handlers::voices))
        .route(
            "/voices/rvc",
            get(handlers::rvc_voice_list).post(handlers::rvc_voice_create),
        )
        .route("/voices/rvc/presets", get(handlers::rvc_voice_presets))
        .route("/voices/rvc/import", post(handlers::rvc_import))
        .route(
            "/voices/rvc/package/verify",
            post(handlers::rvc_package_verify),
        )
        .route(
            "/voices/rvc/package/import",
            post(handlers::rvc_package_import),
        )
        .route(
            "/voices/rvc/:voice",
            get(handlers::rvc_voice_show).delete(handlers::rvc_voice_remove),
        )
        .route(
            "/voices/rvc/:voice/samples",
            get(handlers::rvc_sample_list).post(handlers::rvc_sample_add),
        )
        .route(
            "/voices/rvc/:voice/samples/:sample",
            axum::routing::patch(handlers::rvc_sample_update).delete(handlers::rvc_sample_remove),
        )
        .route(
            "/voices/rvc/:voice/dataset/inspect",
            post(handlers::rvc_dataset_inspect),
        )
        .route(
            "/voices/rvc/:voice/dataset/prepared",
            axum::routing::delete(handlers::rvc_dataset_clear),
        )
        .route(
            "/voices/rvc/:voice/preflight",
            post(handlers::rvc_preflight),
        )
        .route("/voices/rvc/:voice/prepare", post(handlers::rvc_prepare))
        .route("/voices/rvc/:voice/train", post(handlers::rvc_train))
        .route(
            "/voices/rvc/:voice/train/recover",
            post(handlers::rvc_train_recover),
        )
        .route(
            "/voices/rvc/:voice/train/status",
            get(handlers::rvc_train_status),
        )
        .route(
            "/voices/rvc/:voice/train/logs",
            get(handlers::rvc_train_logs),
        )
        .route(
            "/voices/rvc/:voice/train/cancel",
            post(handlers::rvc_train_cancel),
        )
        .route(
            "/voices/rvc/:voice/checkpoints",
            get(handlers::rvc_checkpoints),
        )
        .route(
            "/voices/rvc/:voice/checkpoint",
            post(handlers::rvc_select_checkpoint),
        )
        .route("/voices/rvc/:voice/indexes", get(handlers::rvc_indexes))
        .route("/voices/rvc/:voice/test", post(handlers::rvc_test_voice))
        .route("/voices/rvc/:voice/export", post(handlers::rvc_export))
        .route("/audio/speech", post(handlers::speech))
        .route("/audio/transcriptions", post(handlers::transcriptions))
        .route("/audio/conversions", post(handlers::convert_voice))
        .route("/voices/clone", post(handlers::clone_voice))
        .route("/voices/train", post(handlers::train_voice))
        .route("/voices/:id", axum::routing::delete(handlers::remove_voice))
        .route("/sessions/open", post(handlers::open_session))
        .route("/sessions", get(handlers::sessions))
        .route(
            "/sessions/:id",
            get(handlers::session).delete(handlers::remove_session),
        )
        .route(
            "/sessions/:id/outputs/:filename",
            get(handlers::session_output),
        )
}

fn gui_service() -> ServeDir<ServeFile> {
    let dist = gui_dist_path();
    let index = dist.join("index.html");
    ServeDir::new(&dist)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(index))
}

pub fn gui_dist_path() -> std::path::PathBuf {
    std::env::var("TAKOKIT_GUI_DIST")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/gui/dist")
        })
}

pub async fn run_server(state: AppState) -> anyhow::Result<()> {
    validate_bind_security(&state)?;
    let bind_addr = state.config.bind_addr();
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind Takokit server at {bind_addr}"))?;
    tracing::info!(%bind_addr, "Takokit server listening");
    run_server_with_listener(state, listener, None).await?;
    Ok(())
}

fn validate_bind_security(state: &AppState) -> anyhow::Result<()> {
    let address = state
        .config
        .host
        .parse::<std::net::IpAddr>()
        .with_context(|| format!("invalid Takokit host {}", state.config.host))?;
    if !address.is_loopback()
        && std::env::var("TAKOKIT_API_TOKEN")
            .ok()
            .is_none_or(|token| token.trim().len() < 24)
    {
        anyhow::bail!(
            "non-loopback Takokit binding requires TAKOKIT_API_TOKEN with at least 24 characters"
        );
    }
    Ok(())
}

pub async fn run_server_with_listener(
    state: AppState,
    listener: TcpListener,
    shutdown: Option<oneshot::Receiver<()>>,
) -> anyhow::Result<()> {
    let server = axum::serve(listener, server_router(state));
    if let Some(shutdown) = shutdown {
        server
            .with_graceful_shutdown(async {
                let _ = shutdown.await;
            })
            .await?;
    } else {
        server
            .with_graceful_shutdown(async {
                if let Err(error) = tokio::signal::ctrl_c().await {
                    tracing::warn!(%error, "could not install Ctrl+C handler");
                }
            })
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
