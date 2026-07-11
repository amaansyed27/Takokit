use super::*;
use axum::body::{to_bytes, Body};
use http::{Request, StatusCode};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;
use tokio::sync::oneshot;
use tower::ServiceExt;
use uuid::Uuid;

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
async fn managed_shutdown_requires_matching_identity_and_ps_tracks_execution() {
    let root = tempfile::tempdir().unwrap();
    let config = RuntimeConfig::local(root.path().to_path_buf());
    let instance_id = Uuid::new_v4();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let identity = takokit_core::DaemonIdentity {
        instance_id: Some(instance_id),
        mode: takokit_core::DaemonMode::Managed,
        pid: 42,
        executable: root.path().join("takokit"),
        storage_root: root.path().to_path_buf(),
        host: config.host.clone(),
        port: config.port,
        started_at: 1,
        log_path: None,
    };
    let state = AppState::new(config, LocalStore::new(root.path().to_path_buf()))
        .managed(identity, shutdown_tx);
    let guard = state
        .register_execution("mock-tts".into(), "text_to_speech")
        .await;
    let app = server_router(state.clone());
    let ps = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/ps")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(ps.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/daemon/shutdown")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"instance_id":"{}"}}"#,
                    Uuid::new_v4()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::BAD_REQUEST);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut shutdown_rx)
            .await
            .is_err()
    );
    let accepted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/daemon/shutdown")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"instance_id":"{}"}}"#,
                    instance_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    assert!(shutdown_rx.await.is_ok());
    drop(guard);
    tokio::task::yield_now().await;
    let ps = app
        .oneshot(
            Request::builder()
                .uri("/v1/ps")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(ps.into_body(), 1024 * 1024).await.unwrap();
    assert!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["data"]
            .as_array()
            .unwrap()
            .is_empty()
    );
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
async fn metadata_only_pull_and_runner_contract_are_offline_before_speech() {
    let root = std::env::temp_dir().join("takokit-server-speech-runner-resolution-test");
    let _ = std::fs::remove_dir_all(&root);
    let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
    let app = server_router(state);

    let pull_response = app
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
    assert_eq!(pull_response.status(), StatusCode::OK);

    let runner_response = app
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
    assert_eq!(runner_response.status(), StatusCode::OK);

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
