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
    initialize_runner_runtime, model_info_from_plan, plan_model, python_adapter_records,
    resolve_execution_plan, runner_runtime_layout, InstallModelOptions, LibraryModelManifest,
    LibraryRunnerManifest, ModelPlan, RunnerInfo, RunnerLifecycleState,
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

pub async fn doctor(
    State(state): State<AppState>,
) -> Json<RunnerDetailResponse<serde_json::Value>> {
    let mut checks = vec![
        doctor_check(
            "storage",
            "storage root",
            state.store.root().is_dir(),
            state.store.root().display().to_string(),
        ),
        doctor_check(
            "storage",
            "config.toml",
            state.store.config_path().is_file(),
            state.store.config_path().display().to_string(),
        ),
        doctor_check(
            "registry",
            "runtime model manifests",
            state.package_registry.models().is_ok(),
            "registry/models".to_string(),
        ),
        doctor_check(
            "registry",
            "runtime runner manifests",
            state.package_registry.runners().is_ok(),
            "registry/runners".to_string(),
        ),
        doctor_check(
            "registry",
            "library model manifests",
            state.package_registry.library_models().is_ok(),
            "registry/library/models".to_string(),
        ),
        doctor_check(
            "registry",
            "library runner manifests",
            state.package_registry.library_runners().is_ok(),
            "registry/library/runners".to_string(),
        ),
        doctor_check(
            "installed",
            "installed model records",
            state.installed_registry.installed_model_records().is_ok(),
            "manifests/installed-models".to_string(),
        ),
        doctor_check(
            "installed",
            "installed runner records",
            state.installed_registry.installed_runner_records().is_ok(),
            "manifests/installed-runners".to_string(),
        ),
        doctor_check(
            "gui",
            "GUI dist",
            crate::router::gui_dist_path().join("index.html").is_file(),
            crate::router::gui_dist_path().display().to_string(),
        ),
        doctor_check(
            "runner",
            "python-managed adapters",
            state.store.python_managed_adapters_dir().is_dir(),
            state
                .store
                .python_managed_adapters_dir()
                .display()
                .to_string(),
        ),
    ];
    for runner_id in [
        "takokit-whispercpp",
        "takokit-onnx",
        "takokit-python-managed",
    ] {
        let check = match state.package_registry.runner(runner_id) {
            Ok(manifest) => {
                let layout = runner_runtime_layout(state.store.root(), &manifest);
                match state.installed_registry.installed_runner_record(runner_id) {
                    Ok(record) => runner_doctor_check(
                        runner_id,
                        record.status,
                        format!("{}; logs: {}", record.note, layout.logs.display()),
                    ),
                    Err(_) => serde_json::json!({
                        "section": "runner",
                        "label": format!("{runner_id} runtime missing"),
                        "status": "warn",
                        "detail": format!("run: takokit runner pull {runner_id} && takokit runner install {runner_id}"),
                    }),
                }
            }
            Err(error) => serde_json::json!({
                "section": "runner",
                "label": format!("{runner_id} manifest"),
                "status": "fail",
                "detail": error.to_string(),
            }),
        };
        checks.push(check);
    }

    let executable_models = state
        .package_registry
        .models()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|model| {
            plan_model(
                &state.package_registry,
                &state.installed_registry,
                &model.id,
            )
            .ok()
        })
        .filter(|plan| plan.executable)
        .map(|plan| plan.model_id)
        .collect::<Vec<_>>();

    Json(RunnerDetailResponse {
        data: serde_json::json!({
            "storage_root": state.store.root(),
            "server": state.config.bind_addr(),
            "checks": checks,
            "executable_models": executable_models,
            "logs_path": state.store.logs_dir(),
        }),
    })
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

fn doctor_check(
    section: &'static str,
    label: &'static str,
    ok: bool,
    detail: String,
) -> serde_json::Value {
    serde_json::json!({
        "section": section,
        "label": label,
        "status": if ok { "ok" } else { "warn" },
        "detail": detail,
    })
}

fn runner_doctor_check(
    runner_id: &'static str,
    state: RunnerLifecycleState,
    detail: String,
) -> serde_json::Value {
    let (status, label) = match state {
        RunnerLifecycleState::Ready => ("ok", format!("{runner_id} ready")),
        RunnerLifecycleState::Failed => ("fail", format!("{runner_id} failed")),
        _ => ("warn", format!("{runner_id} state: {state}")),
    };
    serde_json::json!({
        "section": "runner",
        "label": label,
        "status": status,
        "detail": detail,
    })
}

pub async fn models(State(state): State<AppState>) -> Result<Json<ModelsResponse>, ApiError> {
    let manifests = state
        .package_registry
        .models()
        .map_err(Into::into)
        .map_err(ApiError)?;
    let models = manifests
        .into_iter()
        .map(|model| {
            model_info_from_plan(
                &state.package_registry,
                &state.installed_registry,
                &model.id,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
        .map_err(ApiError)?;

    Ok(Json(ModelsResponse { data: models }))
}

pub async fn model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ModelDetailResponse>, ApiError> {
    let info = model_info_from_plan(&state.package_registry, &state.installed_registry, &id)
        .map_err(Into::into)
        .map_err(ApiError)?;

    Ok(Json(ModelDetailResponse { data: info }))
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

pub async fn runners(
    State(state): State<AppState>,
) -> Result<Json<RunnersResponse<RunnerInfo>>, ApiError> {
    let runners = state
        .package_registry
        .runners()
        .map_err(Into::into)
        .map_err(ApiError)?
        .into_iter()
        .map(|runner| {
            if let Ok(record) = state.installed_registry.installed_runner_record(&runner.id) {
                runner.to_runner_info_with_state(true, record.status)
            } else {
                runner.to_runner_info(false)
            }
        })
        .collect();

    Ok(Json(RunnersResponse { data: runners }))
}

pub async fn runner_doctor(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunnerDetailResponse<serde_json::Value>>, ApiError> {
    let manifest = state
        .package_registry
        .runner(&id)
        .map_err(Into::into)
        .map_err(ApiError)?;
    let layout = runner_runtime_layout(state.store.root(), &manifest);
    let record = state.installed_registry.installed_runner_record(&id).ok();
    let adapters = if id == "takokit-python-managed" {
        python_adapter_records(state.store.root()).unwrap_or_default()
    } else {
        Vec::new()
    };
    let executable_models = state
        .package_registry
        .models()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|model| {
            plan_model(
                &state.package_registry,
                &state.installed_registry,
                &model.id,
            )
            .ok()
            .filter(|plan| plan.executable && plan.required_runner == id)
            .map(|plan| plan.model_id)
        })
        .collect::<Vec<_>>();

    Ok(Json(RunnerDetailResponse {
        data: serde_json::json!({
            "id": manifest.id,
            "name": manifest.name,
            "contract_installed": state.installed_registry.is_runner_installed(&id),
            "runtime_state": record
                .as_ref()
                .map(|record| record.status.to_string())
                .unwrap_or_else(|| "runtime-missing".to_string()),
            "note": record.as_ref().map(|record| record.note.clone()),
            "runtime_path": layout.root,
            "logs_path": layout.logs,
            "adapters": adapters,
            "onnx_session_capability": if manifest.id == "takokit-onnx" && record.as_ref().is_some_and(|item| item.status == RunnerLifecycleState::Ready) { Some("kokoro-onnx-ready") } else { None::<&str> },
            "piper_frontend_status": if manifest.id == "takokit-onnx" { Some("piper_text_frontend_not_implemented") } else { None::<&str> },
            "executable_models": executable_models,
        }),
    }))
}

pub async fn launch_test(
    State(state): State<AppState>,
) -> Json<RunnersResponse<serde_json::Value>> {
    let ids = [
        "piper-lessac",
        "kokoro",
        "whisper-base",
        "whisper-tiny",
        "qwen3-tts",
        "chatterbox",
        "f5-tts",
        "sensevoice",
        "parakeet",
        "canary",
        "openvoice",
        "rvc",
    ];
    let data = ids
        .into_iter()
        .map(
            |id| match plan_model(&state.package_registry, &state.installed_registry, id) {
                Ok(plan) => serde_json::json!({
                    "model": plan.model_id,
                    "task": plan.task,
                    "runner": plan.required_runner,
                    "lifecycle": plan.lifecycle_state,
                    "artifacts": plan.artifact_state,
                    "runner_runtime": plan.runner_runtime_state,
                    "executable": plan.executable,
                    "missing": plan.missing,
                    "next_command": plan.next_command,
                }),
                Err(error) => serde_json::json!({
                    "model": id,
                    "error": error.to_string(),
                }),
            },
        )
        .collect();

    Json(RunnersResponse { data })
}

pub async fn library_models(
    State(state): State<AppState>,
) -> Result<Json<RunnersResponse<LibraryModelManifest>>, ApiError> {
    Ok(Json(RunnersResponse {
        data: state
            .package_registry
            .library_models()
            .map_err(Into::into)
            .map_err(ApiError)?,
    }))
}

pub async fn library_runners(
    State(state): State<AppState>,
) -> Result<Json<RunnersResponse<LibraryRunnerManifest>>, ApiError> {
    Ok(Json(RunnersResponse {
        data: state
            .package_registry
            .library_runners()
            .map_err(Into::into)
            .map_err(ApiError)?,
    }))
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
                | ErrorCode::InferenceNotImplemented
                | ErrorCode::PiperTextFrontendNotImplemented => StatusCode::NOT_IMPLEMENTED,
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
