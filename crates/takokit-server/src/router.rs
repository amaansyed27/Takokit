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
        .route("/v1/capabilities", get(handlers::capabilities))
        .route("/v1/models", get(handlers::models))
        .route(
            "/v1/models/:id",
            get(handlers::model).delete(handlers::remove_model),
        )
        .route("/v1/runners", get(handlers::runners))
        .route("/v1/models/pull", post(handlers::pull_model))
        .route("/v1/runners/pull", post(handlers::pull_runner))
        .route(
            "/v1/runners/:id",
            get(handlers::runner).delete(handlers::remove_runner),
        )
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
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/gui/dist")
        });
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
    use axum::body::{to_bytes, Body};
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

    #[tokio::test]
    async fn capabilities_route_returns_five_surfaces() {
        let root = std::env::temp_dir().join("takokit-server-capabilities-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let response = server_router(state)
            .oneshot(
                Request::builder()
                    .uri("/v1/capabilities")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["data"].as_array().unwrap().len(), 5);
        assert_eq!(json["data"][0]["label"], "TTS");
        assert_eq!(json["data"][3]["label"], "Live Transcription API");
    }

    #[tokio::test]
    async fn speech_route_returns_model_not_installed_before_runner_resolution() {
        let root = std::env::temp_dir().join("takokit-server-speech-resolution-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let response = server_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/audio/speech")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"kokoro","input":"Hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "model_not_installed");
        assert_eq!(json["error"]["message"], "model is not installed: kokoro");
    }

    #[tokio::test]
    async fn speech_route_returns_runner_not_installed_after_model_pull() {
        let root = std::env::temp_dir().join("takokit-server-speech-runner-resolution-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let app = server_router(state);

        let pull_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/models/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"kokoro"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pull_response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/audio/speech")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"kokoro","input":"Hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "runner_not_installed");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("kokoro supports TTS"));
    }

    #[tokio::test]
    async fn speech_route_returns_inference_not_implemented_after_model_and_runner_pull() {
        let root = std::env::temp_dir().join("takokit-server-speech-onnx-executor-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let app = server_router(state);

        let pull_model_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/models/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"kokoro"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pull_model_response.status(), StatusCode::OK);

        let pull_runner_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runners/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"runner":"takokit-onnx"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pull_runner_response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/audio/speech")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"kokoro","input":"Hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "inference_not_implemented");
        assert_eq!(
            json["error"]["message"],
            "ONNX runner contract resolved, but real ONNX execution is not implemented yet."
        );
    }

    #[tokio::test]
    async fn runner_lifecycle_routes_install_show_and_remove_runner_contract() {
        let root = std::env::temp_dir().join("takokit-server-runner-lifecycle-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let app = server_router(state);

        let pull_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runners/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"runner":"takokit-onnx"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pull_response.status(), StatusCode::OK);

        let show_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/runners/takokit-onnx")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(show_response.status(), StatusCode::OK);
        let body = to_bytes(show_response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["id"], "takokit-onnx");
        assert_eq!(json["data"]["installed"], true);

        let remove_response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/v1/runners/takokit-onnx")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(remove_response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn transcription_route_returns_unsupported_capability_error() {
        let root = std::env::temp_dir().join("takokit-server-transcription-resolution-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let app = server_router(state);

        let pull_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/models/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"kokoro"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pull_response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/audio/transcriptions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"kokoro","file_path":"audio.wav"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "capability_unsupported");
        assert_eq!(json["error"]["message"], "kokoro does not support STT.");
    }

    #[tokio::test]
    async fn transcription_route_returns_executor_not_implemented_after_model_and_runner_pull() {
        let root = std::env::temp_dir().join("takokit-server-transcription-executor-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let app = server_router(state);

        let pull_model_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/models/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"whisper-base"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pull_model_response.status(), StatusCode::OK);

        let pull_runner_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runners/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"runner":"takokit-whispercpp"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pull_runner_response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/audio/transcriptions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"whisper-base","file_path":"audio.wav"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "inference_not_implemented");
        assert_eq!(
            json["error"]["message"],
            "Runner takokit-whispercpp contract resolved, but transcription execution is not implemented yet."
        );
    }
}
