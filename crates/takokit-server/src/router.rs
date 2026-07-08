use anyhow::Context;
use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};

use crate::{handlers, AppState};

pub fn server_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/v1/status", get(handlers::status))
        .route("/v1/models", get(handlers::models))
        .route("/v1/models/:id", get(handlers::model).delete(handlers::remove_model))
        .route("/v1/runners", get(handlers::runners))
        .route("/v1/models/pull", post(handlers::pull_model))
        .route("/v1/voices", get(handlers::voices))
        .route("/v1/audio/speech", post(handlers::speech))
        .route("/v1/audio/transcriptions", post(handlers::transcriptions))
        .route("/v1/voices/clone", post(handlers::clone_voice))
        .route("/v1/voices/train", post(handlers::train_voice))
        .with_state(state)
        .nest_service("/gui", gui_service())
}

fn gui_service() -> ServeDir<ServeFile> {
    let dist = std::env::var("TAKOKIT_GUI_DIST")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/gui/dist"));
    let index = dist.join("index.html");

    ServeDir::new(&dist)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(index))
}

pub async fn run_server(state: AppState) -> anyhow::Result<()> {
    let bind_addr = state.config.bind_addr();
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind Takokit server at {bind_addr}"))?;

    tracing::info!(%bind_addr, "Takokit server listening");
    axum::serve(listener, server_router(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::{Request, StatusCode};
    use takokit_core::RuntimeConfig;
    use takokit_store::LocalStore;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_route_returns_ok() {
        let root = std::env::temp_dir().join("takokit-server-health-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let response = server_router(state)
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
