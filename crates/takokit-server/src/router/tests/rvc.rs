use super::*;
use axum::body::{to_bytes, Body};
use http::{Request, StatusCode};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;
use tower::ServiceExt;

fn test_state(temp: &tempfile::TempDir) -> AppState {
    let root = temp.path().join("takokit");
    AppState::new(RuntimeConfig::local(root.clone()), LocalStore::new(root))
}

async fn json_body(response: http::Response<Body>) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("response bytes");
    serde_json::from_slice(&body).expect("json response")
}

#[tokio::test]
async fn rvc_project_create_list_show_and_dry_run_remove_are_persistent() {
    let temp = tempfile::tempdir().unwrap();
    let app = server_router(test_state(&temp));

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/voices/rvc")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Studio Voice ü","consent_affirmed":true,"consent_note":"route test"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let created = json_body(create).await;
    assert_eq!(created["kind"], "rvc_voice");
    assert_eq!(created["data"]["name"], "Studio Voice ü");
    let id = created["data"]["id"].as_str().unwrap().to_string();

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/voices/rvc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let listed = json_body(list).await;
    assert!(listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == id));

    let show = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/voices/rvc/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(show.status(), StatusCode::OK);
    let detail = json_body(show).await;
    assert_eq!(detail["data"]["project"]["id"], id);
    assert_eq!(detail["data"]["dataset"]["sample_count"], 0);

    let preview = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/voices/rvc/{id}?dry_run=true"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(preview.status(), StatusCode::OK);
    let preview = json_body(preview).await;
    assert_eq!(preview["data"]["dry_run"], true);
    assert_eq!(preview["data"]["removed"], false);

    let still_there = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/voices/rvc/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(still_there.status(), StatusCode::OK);
}

#[tokio::test]
async fn imported_rvc_checkpoint_and_index_join_shared_voice_inventory() {
    let temp = tempfile::tempdir().unwrap();
    let checkpoint = temp.path().join("Imported Voice ü.pth");
    let index = temp.path().join("Imported Voice ü.index");
    std::fs::write(&checkpoint, b"checkpoint-fixture").unwrap();
    std::fs::write(&index, b"index-fixture").unwrap();
    let app = server_router(test_state(&temp));

    let request = serde_json::json!({
        "checkpoint": checkpoint,
        "index": index,
        "name": "Imported Voice ü",
        "consent_affirmed": true,
        "consent_note": "route fixture"
    });
    let imported = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/voices/rvc/import")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(imported.status(), StatusCode::CREATED);
    let imported = json_body(imported).await;
    let id = imported["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(imported["data"]["state"], "ready");

    let checkpoints = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/voices/rvc/{id}/checkpoints"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(checkpoints.status(), StatusCode::OK);
    let checkpoints = json_body(checkpoints).await;
    assert_eq!(checkpoints["data"].as_array().unwrap().len(), 1);
    assert_eq!(checkpoints["data"][0]["valid_for_inference"], true);

    let indexes = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/voices/rvc/{id}/indexes"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(indexes.status(), StatusCode::OK);
    let indexes = json_body(indexes).await;
    assert_eq!(indexes["data"].as_array().unwrap().len(), 1);
    assert_eq!(indexes["data"][0]["valid"], true);

    let voices = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/voices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(voices.status(), StatusCode::OK);
    let voices = json_body(voices).await;
    let managed = voices["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|voice| voice["id"] == id)
        .expect("managed RVC voice in shared inventory");
    assert_eq!(managed["source"], "managed-rvc");
    assert_eq!(managed["model_id"], "rvc");
}

#[tokio::test]
async fn rvc_presets_endpoint_exposes_backend_owned_product_presets() {
    let temp = tempfile::tempdir().unwrap();
    let response = server_router(test_state(&temp))
        .oneshot(
            Request::builder()
                .uri("/api/v1/voices/rvc/presets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    let ids = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|preset| preset["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["quick", "balanced", "high-quality", "custom"]);
}
