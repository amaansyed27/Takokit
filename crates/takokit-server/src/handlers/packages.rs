use super::*;

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

#[derive(serde::Deserialize)]
pub struct AdapterInstallRequest {
    pub adapter: String,
}

pub async fn adapters(
    State(state): State<AppState>,
) -> Result<Json<RunnersResponse<takokit_package::AdapterRecord>>, ApiError> {
    Ok(Json(RunnersResponse {
        data: python_adapter_records(state.store.root())
            .map_err(Into::into)
            .map_err(ApiError)?,
    }))
}

pub async fn adapter(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunnerDetailResponse<takokit_package::AdapterRecord>>, ApiError> {
    Ok(Json(RunnerDetailResponse {
        data: python_adapter_record(state.store.root(), &id.replace('-', "_"))
            .map_err(Into::into)
            .map_err(ApiError)?,
    }))
}

pub async fn install_adapter(
    State(state): State<AppState>,
    Json(request): Json<AdapterInstallRequest>,
) -> Result<Json<RunnerDetailResponse<takokit_package::AdapterRecord>>, ApiError> {
    let id = request.adapter.replace('-', "_");
    let record = install_python_adapter(state.store.root(), &id)
        .map_err(Into::into)
        .map_err(ApiError)?;
    Ok(Json(RunnerDetailResponse { data: record }))
}

pub async fn adapter_doctor(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunnerDetailResponse<serde_json::Value>>, ApiError> {
    let id = id.replace('-', "_");
    let record = python_adapter_record(state.store.root(), &id)
        .map_err(Into::into)
        .map_err(ApiError)?;
    let path = state.store.python_managed_adapters_dir().join(&id);
    Ok(Json(RunnerDetailResponse {
        data: serde_json::json!({ "id": record.id, "state": record.state, "note": record.notes, "logs_path": path.join("install.log"), "adapter_path": path }),
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
) -> Result<Json<ModelInstallReport>, ApiError> {
    let report = install_model_complete(
        &state.package_registry,
        &state.installed_registry,
        state.store.root(),
        &request.model,
        InstallModelOptions {
            metadata_only: request.metadata_only,
        },
    )
    .map_err(Into::into)
    .map_err(ApiError)?;
    Ok(Json(report))
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
