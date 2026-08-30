use super::*;
use axum::body::{to_bytes, Body};
use http::{Request, StatusCode};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;
use tower::ServiceExt;

fn app() -> Router {
    let root = std::env::temp_dir().join(format!("takokit-openai-test-{}", Uuid::new_v4()));
    server_router(AppState::new(
        RuntimeConfig {
            host: "127.0.0.1".to_string(),
            port: 5050,
            storage_root: root.clone(),
        },
        LocalStore::new(root),
    ))
}

#[tokio::test]
async fn openai_models_has_sdk_compatible_list_shape_and_request_id() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["object"], "list");
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn openai_speech_rejects_unknown_fields_with_openai_error_envelope() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/audio/speech")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"model":"kokoro","input":"hello","voice":"default","surprise":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["type"], "invalid_request_error");
    assert_eq!(json["error"]["code"], "invalid_request");
    assert!(json["error"]["param"].is_null());
}

#[tokio::test]
async fn openai_transcription_rejects_truncated_wav_before_execution() {
    let boundary = "takokit-truncated-wav";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"broken.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFFbad\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nwhisper-tiny\r\n--{boundary}--\r\n"
    );
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/audio/transcriptions")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["type"], "invalid_request_error");
    assert_eq!(json["error"]["param"], "file");
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("malformed"));
}

#[tokio::test]
async fn hostile_host_and_origin_are_rejected_on_loopback() {
    for (header, value, code) in [
        ("host", "attacker.example", "invalid_host"),
        ("origin", "https://attacker.example", "origin_not_allowed"),
    ] {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/status")
                    .header(header, value)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], code);
    }
}

#[tokio::test]
async fn openapi_and_native_namespace_are_live() {
    let openapi = app()
        .clone()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(openapi.status(), StatusCode::OK);
    let body = to_bytes(openapi.into_body(), 1024 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["openapi"], "3.1.0");
    assert!(json["paths"]["/v1/audio/speech"].is_object());
    assert!(json["paths"]["/api/v1/models"].is_object());

    let native = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(native.status(), StatusCode::OK);
}

#[tokio::test]
async fn non_loopback_requires_the_configured_bearer_token() {
    let root = std::env::temp_dir().join(format!("takokit-network-test-{}", Uuid::new_v4()));
    let state = AppState::new(
        RuntimeConfig {
            host: "0.0.0.0".to_string(),
            port: 5050,
            storage_root: root.clone(),
        },
        LocalStore::new(root),
    );
    let token = "test-only-token-with-24-characters";
    std::env::set_var("TAKOKIT_API_TOKEN", token);
    let invalid = server_router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .header("authorization", "Bearer wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);

    let valid = server_router(state)
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    std::env::remove_var("TAKOKIT_API_TOKEN");
    assert_eq!(valid.status(), StatusCode::OK);
}
