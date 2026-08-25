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
        .route("/v1/system/picker/audio", get(handlers::pick_audio_file))
        .route("/v1/system/picker/folder", get(handlers::pick_folder))
        .route(
            "/v1/system/picker/rvc",
            get(handlers::pick_rvc_artifact),
        )
        .route("/v1/system/audio", get(handlers::local_audio))
        .route("/v1/system/storage", get(handlers::storage_overview))
        .route("/v1/system/open", post(handlers::open_location))
        .route(
            "/v1/files",
            get(handlers::workspace_files).post(handlers::upload_workspace_file),
        )
        .route(
            "/v1/files/:id/content",
            get(handlers::workspace_file_content),
        )
        .route(
            "/v1/files/:id",
            axum::routing::delete(handlers::delete_workspace_file),
        )
        .route("/v1/test/launch", get(handlers::launch_test))
        .route("/v1/capabilities", get(handlers::capabilities))
        .route("/v1/models", get(handlers::models))
        .route("/v1/models/installed", get(handlers::installed_models))
        .route("/v1/library/models", get(handlers::library_models))
        .route("/v1/library/runners", get(handlers::library_runners))
        .route("/v1/models/:id/plan", get(handlers::model_plan))
        .route(
            "/v1/models/:id/progress",
            get(handlers::model_pull_progress),
        )
        .route(
            "/v1/models/:id",
            get(handlers::model).delete(handlers::remove_model),
        )
        .route("/v1/runners", get(handlers::runners))
        .route("/v1/adapters", get(handlers::adapters))
        .route("/v1/adapters/install", post(handlers::install_adapter))
        .route("/v1/adapters/:id/doctor", get(handlers::adapter_doctor))
        .route("/v1/adapters/:id", get(handlers::adapter))
        .route("/v1/models/pull", post(handlers::model_pull_with_progress))
        .route("/v1/runners/pull", post(handlers::pull_runner))
        .route("/v1/runners/install", post(handlers::install_runner))
        .route("/v1/runners/:id/doctor", get(handlers::runner_doctor))
        .route(
            "/v1/runners/:id",
            get(handlers::runner).delete(handlers::remove_runner),
        )
        .route("/v1/voices", get(handlers::voices))
        .route(
            "/v1/voices/rvc",
            get(handlers::rvc_voice_list).post(handlers::rvc_voice_create),
        )
        .route("/v1/voices/rvc/presets", get(handlers::rvc_voice_presets))
        .route("/v1/voices/rvc/import", post(handlers::rvc_import))
        .route(
            "/v1/voices/rvc/package/verify",
            post(handlers::rvc_package_verify),
        )
        .route(
            "/v1/voices/rvc/package/import",
            post(handlers::rvc_package_import),
        )
        .route(
            "/v1/voices/rvc/:voice",
            get(handlers::rvc_voice_show).delete(handlers::rvc_voice_remove),
        )
        .route(
            "/v1/voices/rvc/:voice/samples",
            get(handlers::rvc_sample_list).post(handlers::rvc_sample_add),
        )
        .route(
            "/v1/voices/rvc/:voice/samples/:sample",
            axum::routing::patch(handlers::rvc_sample_update).delete(handlers::rvc_sample_remove),
        )
        .route(
            "/v1/voices/rvc/:voice/dataset/inspect",
            post(handlers::rvc_dataset_inspect),
        )
        .route(
            "/v1/voices/rvc/:voice/dataset/prepared",
            axum::routing::delete(handlers::rvc_dataset_clear),
        )
        .route(
            "/v1/voices/rvc/:voice/preflight",
            post(handlers::rvc_preflight),
        )
        .route("/v1/voices/rvc/:voice/prepare", post(handlers::rvc_prepare))
        .route("/v1/voices/rvc/:voice/train", post(handlers::rvc_train))
        .route(
            "/v1/voices/rvc/:voice/train/recover",
            post(handlers::rvc_train_recover),
        )
        .route(
            "/v1/voices/rvc/:voice/train/status",
            get(handlers::rvc_train_status),
        )
        .route(
            "/v1/voices/rvc/:voice/train/logs",
            get(handlers::rvc_train_logs),
        )
        .route(
            "/v1/voices/rvc/:voice/train/cancel",
            post(handlers::rvc_train_cancel),
        )
        .route(
            "/v1/voices/rvc/:voice/checkpoints",
            get(handlers::rvc_checkpoints),
        )
        .route(
            "/v1/voices/rvc/:voice/checkpoint",
            post(handlers::rvc_select_checkpoint),
        )
        .route("/v1/voices/rvc/:voice/indexes", get(handlers::rvc_indexes))
        .route("/v1/voices/rvc/:voice/test", post(handlers::rvc_test_voice))
        .route("/v1/voices/rvc/:voice/export", post(handlers::rvc_export))
        .route("/v1/audio/speech", post(handlers::speech))
        .route("/v1/audio/transcriptions", post(handlers::transcriptions))
        .route("/v1/audio/conversions", post(handlers::convert_voice))
        .route("/v1/voices/clone", post(handlers::clone_voice))
        .route("/v1/voices/train", post(handlers::train_voice))
        .route(
            "/v1/voices/:id",
            axum::routing::delete(handlers::remove_voice),
        )
        .route("/v1/sessions/open", post(handlers::open_session))
        .route("/v1/sessions", get(handlers::sessions))
        .route(
            "/v1/sessions/:id",
            get(handlers::session).delete(handlers::remove_session),
        )
        .route(
            "/v1/sessions/:id/outputs/:filename",
            get(handlers::session_output),
        )
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
mod tests;
