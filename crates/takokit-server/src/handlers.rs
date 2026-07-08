use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use takokit_core::{
    CloneVoiceRequest, HealthResponse, ModelDetailResponse, ModelsResponse, PullModelRequest,
    PullModelResponse, RunnersResponse, SpeechRequest, TakokitError, TrainVoiceRequest,
    TranscriptionRequest, VoicesResponse,
};
use takokit_models::TextToSpeechEngine;
use takokit_package::RunnerInfo;

use crate::AppState;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "takokit".to_string(),
    })
}

pub async fn status(State(state): State<AppState>) -> Json<takokit_core::RuntimeStatus> {
    Json(state.status())
}

pub async fn models(State(state): State<AppState>) -> Json<ModelsResponse> {
    let models = state
        .package_registry
        .models()
        .unwrap_or_default()
        .into_iter()
        .map(|model| {
            let installed = state.installed_registry.is_model_installed(&model.id);
            model.to_model_info(installed)
        })
        .collect();

    Json(ModelsResponse {
        data: models,
    })
}

pub async fn model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ModelDetailResponse>, ApiError> {
    let manifest = state.package_registry.model(&id).map_err(Into::into).map_err(ApiError)?;
    let installed = state.installed_registry.is_model_installed(&manifest.id);

    Ok(Json(ModelDetailResponse {
        data: manifest.to_model_info(installed),
    }))
}

pub async fn runners(State(state): State<AppState>) -> Json<RunnersResponse<RunnerInfo>> {
    let runners = state
        .package_registry
        .runners()
        .unwrap_or_default()
        .into_iter()
        .map(|runner| runner.to_runner_info(false))
        .collect();

    Json(RunnersResponse { data: runners })
}

pub async fn pull_model(
    State(state): State<AppState>,
    Json(request): Json<PullModelRequest>,
) -> Result<Json<PullModelResponse>, ApiError> {
    let manifest = state
        .package_registry
        .model(&request.model)
        .map_err(Into::into)
        .map_err(ApiError)?;
    let report = state
        .installed_registry
        .install_model(&manifest)
        .map_err(Into::into)
        .map_err(ApiError)?;

    Ok(Json(PullModelResponse {
        id: report.id,
        installed: report.installed,
        manifest_path: report.manifest_path,
        note: report.note,
    }))
}

pub async fn remove_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .installed_registry
        .remove_model(&id)
        .map_err(Into::into)
        .map_err(ApiError)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn voices(State(state): State<AppState>) -> Json<VoicesResponse> {
    Json(VoicesResponse {
        data: state.registry.voices().to_vec(),
    })
}

pub async fn speech(
    State(state): State<AppState>,
    Json(request): Json<SpeechRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if request.model != "mock-tts" {
        return Err(ApiError(TakokitError::NotImplemented {
            feature: "real model speech inference",
            reason: "model packages can be registered, but runners are not implemented yet; use mock-tts for the test WAV path",
        }));
    }

    let response = state
        .tts
        .synthesize(request, &state.store.outputs_dir())
        .await
        .map_err(ApiError)?;

    Ok((StatusCode::OK, Json(response)))
}

pub async fn transcriptions(
    Json(_request): Json<TranscriptionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError(TakokitError::NotImplemented {
        feature: "speech transcription",
        reason: "Whisper and whisper.cpp adapters are scaffolded but not wired yet",
    }))
}

pub async fn clone_voice(
    Json(_request): Json<CloneVoiceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError(TakokitError::NotImplemented {
        feature: "voice cloning",
        reason: "clone adapters require explicit model runner integration",
    }))
}

pub async fn train_voice(
    Json(_request): Json<TrainVoiceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError(TakokitError::NotImplemented {
        feature: "voice training",
        reason: "training jobs and dataset preparation are planned for a later phase",
    }))
}

#[derive(Debug)]
pub struct ApiError(pub TakokitError);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            TakokitError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            TakokitError::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,
            TakokitError::Model(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(serde_json::json!({
            "error": {
                "message": self.0.to_string()
            }
        }));

        (status, body).into_response()
    }
}
