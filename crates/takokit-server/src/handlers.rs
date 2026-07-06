use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use takokit_core::{
    CloneVoiceRequest, HealthResponse, ModelsResponse, SpeechRequest, TakokitError,
    TrainVoiceRequest, TranscriptionRequest, VoicesResponse,
};
use takokit_models::TextToSpeechEngine;

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
    Json(ModelsResponse {
        data: state.registry.models().to_vec(),
    })
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
