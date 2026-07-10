use anyhow::Context;
use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tower_http::services::{ServeDir, ServeFile};

use crate::{handlers, AppState};

pub fn server_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/v1/status", get(handlers::status))
        .route("/v1/daemon/identity", get(handlers::daemon_identity))
        .route("/v1/daemon/shutdown", post(handlers::daemon_shutdown))
        .route("/v1/ps", get(handlers::ps))
        .route("/v1/doctor", get(handlers::doctor))
        .route("/v1/test/launch", get(handlers::launch_test))
        .route("/v1/capabilities", get(handlers::capabilities))
        .route("/v1/models", get(handlers::models))
        .route("/v1/library/models", get(handlers::library_models))
        .route("/v1/library/runners", get(handlers::library_runners))
        .route("/v1/models/:id/plan", get(handlers::model_plan))
        .route(
            "/v1/models/:id",
            get(handlers::model).delete(handlers::remove_model),
        )
        .route("/v1/runners", get(handlers::runners))
        .route("/v1/adapters", get(handlers::adapters))
        .route("/v1/adapters/install", post(handlers::install_adapter))
        .route("/v1/adapters/:id/doctor", get(handlers::adapter_doctor))
        .route("/v1/adapters/:id", get(handlers::adapter))
        .route("/v1/models/pull", post(handlers::pull_model))
        .route("/v1/runners/pull", post(handlers::pull_runner))
        .route("/v1/runners/install", post(handlers::install_runner))
        .route("/v1/runners/:id/doctor", get(handlers::runner_doctor))
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
    let bind_addr = state.config.bind_addr();
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind Takokit server at {bind_addr}"))?;

    tracing::info!(%bind_addr, "Takokit server listening");
    run_server_with_listener(state, listener, None).await?;
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
        server.await?;
    }
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
        let state = AppState::new(
            RuntimeConfig::local(root.clone()),
            LocalStore::new(root.clone()),
        );
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
        let state = AppState::new(
            RuntimeConfig::local(root.clone()),
            LocalStore::new(root.clone()),
        );
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
    async fn adapters_route_is_available_without_claiming_adapter_readiness() {
        let root = std::env::temp_dir().join("takokit-server-adapters-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let response = server_router(state)
            .oneshot(
                Request::builder()
                    .uri("/v1/adapters")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["data"].as_array().is_some());
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
    async fn speech_route_requires_kokoro_artifacts_before_execution() {
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
                    .body(Body::from(r#"{"model":"kokoro","metadata_only":true}"#))
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

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "artifact_not_downloaded");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("kokoro-v1.0.int8.onnx"));
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
                    .body(Body::from(r#"{"runner":"takokit-transformers-audio"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pull_response.status(), StatusCode::OK);

        let show_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/runners/takokit-transformers-audio")
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
        assert_eq!(json["data"]["id"], "takokit-transformers-audio");
        assert_eq!(json["data"]["installed"], true);

        let install_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runners/install")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"runner":"takokit-transformers-audio"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(install_response.status(), StatusCode::OK);

        let show_installed_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/runners/takokit-transformers-audio")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(show_installed_response.status(), StatusCode::OK);
        let body = to_bytes(show_installed_response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["install_state"], "runtime-installed");

        let remove_response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/v1/runners/takokit-transformers-audio")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(remove_response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn library_routes_return_curated_model_and_runner_manifests() {
        let root = std::env::temp_dir().join("takokit-server-library-routes-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let app = server_router(state);

        let models_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/library/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(models_response.status(), StatusCode::OK);
        let body = to_bytes(models_response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|model| model["id"] == "qwen3-tts"));

        let runners_response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/library/runners")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(runners_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn model_plan_route_returns_honest_state_for_metadata_only_model() {
        let root = std::env::temp_dir().join("takokit-server-model-plan-route-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let response = server_router(state)
            .oneshot(
                Request::builder()
                    .uri("/v1/models/qwen3-tts/plan")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["data"]["model_id"], "qwen3-tts");
        assert_eq!(json["data"]["required_runner"], "takokit-python-managed");
        assert_eq!(json["data"]["artifact_state"], "metadata-only");
        assert_eq!(json["data"]["runner_runtime_state"], "runtime-missing");
        assert_eq!(json["data"]["executable"], false);
    }

    #[tokio::test]
    async fn diagnostics_routes_return_doctor_runner_doctor_and_launch_suite() {
        let root = std::env::temp_dir().join("takokit-server-diagnostics-routes-test");
        let state = AppState::new(
            RuntimeConfig::local(root.clone()),
            LocalStore::new(root.clone()),
        );
        let app = server_router(state);

        let doctor = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/doctor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(doctor.status(), StatusCode::OK);
        let body = to_bytes(doctor.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["storage_root"], root.display().to_string());
        assert!(json["data"]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["label"] == "runtime model manifests"));

        let runner = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/runners/takokit-whispercpp/doctor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(runner.status(), StatusCode::OK);
        let body = to_bytes(runner.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["id"], "takokit-whispercpp");
        assert_eq!(json["data"]["runtime_state"], "runtime-missing");

        let suite = app
            .oneshot(
                Request::builder()
                    .uri("/v1/test/launch")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(suite.status(), StatusCode::OK);
        let body = to_bytes(suite.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["model"] == "whisper-base"));
    }

    #[tokio::test]
    async fn models_route_includes_canonical_plan_summary_fields() {
        let root = std::env::temp_dir().join("takokit-server-models-plan-summary-test");
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let response = server_router(state)
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let whisper = json["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "whisper-base")
            .expect("whisper-base summary");

        assert_eq!(whisper["family"], "whisper");
        assert_eq!(whisper["lifecycle_state"], "metadata-only");
        assert_eq!(whisper["runner_runtime_state"], "runtime-missing");
        assert_eq!(whisper["executable"], false);
        assert_eq!(whisper["next_command"], "takokit pull whisper-base");
        assert!(whisper["missing"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "verified artifacts"));
    }

    #[tokio::test]
    async fn pull_model_route_supports_metadata_only_option_for_artifact_manifest() {
        let root = std::env::temp_dir().join(format!(
            "takokit-server-piper-pull-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let app = server_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/models/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"piper-lessac","metadata_only":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], "piper-lessac");
        assert_eq!(json["installed"], true);
        assert!(json["note"].as_str().unwrap().contains("metadata-only"));
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
    async fn transcription_route_reports_metadata_only_whisper_artifact_before_execution() {
        let root = std::env::temp_dir().join("takokit-server-transcription-executor-test");
        let audio_path = root.join("audio.wav");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&audio_path, b"not a real wav").unwrap();
        let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
        let app = server_router(state);

        let pull_model_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/models/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"whisper-base","metadata_only":true}"#,
                    ))
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
                    .body(Body::from(format!(
                        r#"{{"model":"whisper-base","file_path":"{}"}}"#,
                        audio_path.display().to_string().replace('\\', "\\\\")
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "artifact_not_downloaded");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("recorded but not downloaded"));
    }
}
