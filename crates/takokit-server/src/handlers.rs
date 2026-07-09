use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use takokit_core::{
    CapabilitiesResponse, CapabilityInfo, CapabilityKind, CloneVoiceRequest, ErrorCode,
    HealthResponse, ModelDetailResponse, ModelsResponse, PullModelRequest, PullModelResponse,
    PullRunnerRequest, RunnerDetailResponse, RunnersResponse, SpeechRequest, TakokitError,
    TrainVoiceRequest, TranscriptionRequest, VoicesResponse,
};
use takokit_models::{execute_speech, execute_transcription, TextToSpeechEngine};
use takokit_package::{
    initialize_runner_runtime, plan_model, resolve_execution_plan, InstallModelOptions,
    LibraryModelManifest, LibraryRunnerManifest, ModelPlan, RunnerInfo,
};

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

pub async fn capabilities() -> Json<CapabilitiesResponse> {
    Json(CapabilitiesResponse {
        data: CapabilityKind::ALL
            .into_iter()
            .map(|capability| CapabilityInfo {
                id: capability,
                label: capability.label().to_string(),
                description: capability.explanation().to_string(),
            })
            .collect(),
    })
}

pub async fn models(State(state): State<AppState>) -> Json<ModelsResponse> {
    let models = state
        .package_registry
        .models()
        .unwrap_or_default()
        .into_iter()
        .map(|model| {
            let installed = state.installed_registry.is_model_installed(&model.id);
            let runner_installed = state.installed_registry.is_runner_installed(&model.runner);
            model.to_model_info(installed, runner_installed)
        })
        .collect();

    Json(ModelsResponse { data: models })
}

pub async fn model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ModelDetailResponse>, ApiError> {
    let manifest = state
        .package_registry
        .model(&id)
        .map_err(Into::into)
        .map_err(ApiError)?;
    let installed = state.installed_registry.is_model_installed(&manifest.id);
    let runner_installed = state
        .installed_registry
        .is_runner_installed(&manifest.runner);

    Ok(Json(ModelDetailResponse {
        data: manifest.to_model_info(installed, runner_installed),
    }))
}

pub async fn model_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunnerDetailResponse<ModelPlan>>, ApiError> {
    let plan = plan_model(&state.package_registry, &state.installed_registry, &id)
        .map_err(Into::into)
        .map_err(ApiError)?;

    Ok(Json(RunnerDetailResponse { data: plan }))
}

pub async fn runners(State(state): State<AppState>) -> Json<RunnersResponse<RunnerInfo>> {
    let runners = state
        .package_registry
        .runners()
        .unwrap_or_default()
        .into_iter()
        .map(|runner| {
            if let Ok(record) = state.installed_registry.installed_runner_record(&runner.id) {
                runner.to_runner_info_with_state(true, record.status)
            } else {
                runner.to_runner_info(false)
            }
        })
        .collect();

    Json(RunnersResponse { data: runners })
}

pub async fn library_models(
    State(state): State<AppState>,
) -> Json<RunnersResponse<LibraryModelManifest>> {
    Json(RunnersResponse {
        data: state.package_registry.library_models().unwrap_or_default(),
    })
}

pub async fn library_runners(
    State(state): State<AppState>,
) -> Json<RunnersResponse<LibraryRunnerManifest>> {
    Json(RunnersResponse {
        data: state.package_registry.library_runners().unwrap_or_default(),
    })
}

pub async fn runner(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunnerDetailResponse<RunnerInfo>>, ApiError> {
    let manifest = state
        .package_registry
        .runner(&id)
        .map_err(Into::into)
        .map_err(ApiError)?;
    let installed = state.installed_registry.is_runner_installed(&manifest.id);
    let info = if let Ok(record) = state
        .installed_registry
        .installed_runner_record(&manifest.id)
    {
        manifest.to_runner_info_with_state(true, record.status)
    } else {
        manifest.to_runner_info(installed)
    };

    Ok(Json(RunnerDetailResponse { data: info }))
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
        .install_model_with_options(
            &manifest,
            InstallModelOptions {
                metadata_only: request.metadata_only,
            },
        )
        .map_err(Into::into)
        .map_err(ApiError)?;

    Ok(Json(PullModelResponse {
        id: report.id,
        installed: report.installed,
        manifest_path: report.manifest_path,
        note: report.note,
    }))
}

pub async fn pull_runner(
    State(state): State<AppState>,
    Json(request): Json<PullRunnerRequest>,
) -> Result<Json<PullModelResponse>, ApiError> {
    let manifest = state
        .package_registry
        .runner(&request.runner)
        .map_err(Into::into)
        .map_err(ApiError)?;
    let report = state
        .installed_registry
        .install_runner(&manifest)
        .map_err(Into::into)
        .map_err(ApiError)?;

    Ok(Json(PullModelResponse {
        id: report.id,
        installed: report.installed,
        manifest_path: report.manifest_path,
        note: report.note,
    }))
}

pub async fn install_runner(
    State(state): State<AppState>,
    Json(request): Json<PullRunnerRequest>,
) -> Result<Json<PullModelResponse>, ApiError> {
    let manifest = state
        .package_registry
        .runner(&request.runner)
        .map_err(Into::into)
        .map_err(ApiError)?;
    let report =
        initialize_runner_runtime(state.store.root(), &state.installed_registry, &manifest)
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

pub async fn remove_runner(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .installed_registry
        .remove_runner(&id)
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
        let plan = resolve_execution_plan(
            &state.package_registry,
            &state.installed_registry,
            &request.model,
            CapabilityKind::TextToSpeech,
        )
        .map_err(Into::into)
        .map_err(ApiError)?;

        let response = execute_speech(&plan, request, &state.store.outputs_dir())
            .await
            .map_err(ApiError)?;
        return Ok((StatusCode::OK, Json(response)));
    }

    let response = state
        .tts
        .synthesize(request, &state.store.outputs_dir())
        .await
        .map_err(ApiError)?;

    Ok((StatusCode::OK, Json(response)))
}

pub async fn transcriptions(
    State(state): State<AppState>,
    Json(request): Json<TranscriptionRequest>,
) -> Result<Json<takokit_core::TranscriptionResponse>, ApiError> {
    let model = request
        .model
        .clone()
        .unwrap_or_else(|| "whisper-base".to_string());
    let plan = resolve_execution_plan(
        &state.package_registry,
        &state.installed_registry,
        &model,
        CapabilityKind::SpeechToText,
    )
    .map_err(Into::into)
    .map_err(ApiError)?;

    let response = execute_transcription(&plan, request)
        .await
        .map_err(ApiError)?;
    Ok(Json(response))
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
            TakokitError::Resolution { code, .. } => match code {
                ErrorCode::CapabilityUnsupported => StatusCode::BAD_REQUEST,
                ErrorCode::ArtifactUrlMissing
                | ErrorCode::ArtifactChecksumMissing
                | ErrorCode::ArtifactChecksumMismatch
                | ErrorCode::ArtifactInstallFailed
                | ErrorCode::ArtifactMissing
                | ErrorCode::ArtifactNotDownloaded
                | ErrorCode::ArtifactConfigInvalid => StatusCode::BAD_REQUEST,
                ErrorCode::ArtifactDownloadFailed => StatusCode::BAD_GATEWAY,
                ErrorCode::ModelNotFound
                | ErrorCode::ModelNotInstalled
                | ErrorCode::RunnerNotFound => StatusCode::NOT_FOUND,
                ErrorCode::RunnerNotInstalled
                | ErrorCode::RunnerUnsupportedOnPlatform
                | ErrorCode::InferenceNotImplemented => StatusCode::NOT_IMPLEMENTED,
            },
            TakokitError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            TakokitError::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,
            TakokitError::Model(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let code = match &self.0 {
            TakokitError::Resolution { code, .. } => code.as_str(),
            TakokitError::InvalidRequest(_) => "invalid_request",
            TakokitError::NotImplemented { .. } => "inference_not_implemented",
            TakokitError::Model(_) => "model_error",
            TakokitError::Storage(_) => "storage_error",
            TakokitError::Audio(_) => "audio_error",
        };
        let body = Json(serde_json::json!({
            "error": {
                "code": code,
                "message": self.0.to_string()
            }
        }));

        (status, body).into_response()
    }
}
