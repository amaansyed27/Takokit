use super::*;
use axum::body::Body;
use http::{Request, StatusCode};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;
use tower::ServiceExt;

#[tokio::test]
async fn workspace_upload_accepts_body_above_axum_default_limit() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    let home = root.path().join("home");
    std::fs::create_dir_all(&workspace).unwrap();

    let state = AppState::new(RuntimeConfig::local(home.clone()), LocalStore::new(home));
    let payload = vec![0x2a; 3 * 1024 * 1024];
    let encoded = crate::workspace::encoded_workspace_header(&workspace.to_string_lossy());

    let response = server_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files?name=long-sample.mp3")
                .header("content-type", "audio/mpeg")
                .header(crate::workspace::WORKSPACE_HEADER, encoded)
                .body(Body::from(payload.clone()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let saved = workspace
        .join(".tako")
        .join("files")
        .join("long-sample.mp3");
    assert_eq!(
        std::fs::metadata(saved).unwrap().len(),
        payload.len() as u64
    );
}
