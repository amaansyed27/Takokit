use super::*;
use axum::body::{to_bytes, Body};
use http::{Request, StatusCode};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;
use tower::ServiceExt;

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
    assert_eq!(json["model_id"], "piper-lessac");
    assert_eq!(json["artifacts"]["state"], "metadata-only");
    assert_eq!(json["runner_runtime"]["state"], "not-requested");
    assert_eq!(json["executable"], false);
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
                .body(Body::from(r#"{"model":"kokoro","metadata_only":true}"#))
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

#[tokio::test]
async fn installed_models_route_excludes_catalog_and_metadata_only_entries() {
    let root = std::env::temp_dir().join(format!(
        "takokit-server-installed-models-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state = AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root));
    let app = server_router(state);

    let empty = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/models/installed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(empty.status(), StatusCode::OK);
    let body = to_bytes(empty.into_body(), 1024 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["kind"], "installed-models");
    assert!(json["data"].as_array().unwrap().is_empty());

    let metadata = app
        .clone()
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
    assert_eq!(metadata.status(), StatusCode::OK);

    let listed = app
        .oneshot(
            Request::builder()
                .uri("/v1/models/installed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(listed.into_body(), 1024 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["data"].as_array().unwrap().is_empty());
}
