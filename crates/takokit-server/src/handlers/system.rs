use super::*;
use axum::http::HeaderMap;
use serde::Deserialize;
use std::path::Path;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "takokit".to_string(),
    })
}

pub async fn status(State(state): State<AppState>) -> Json<takokit_core::RuntimeStatus> {
    Json(state.status())
}

pub async fn daemon_identity(State(state): State<AppState>) -> Json<DaemonBuildIdentity> {
    Json(DaemonBuildIdentity {
        identity: state.daemon_identity.clone(),
        build_id: state.build_id.clone(),
    })
}

pub async fn daemon_shutdown(
    State(state): State<AppState>,
    Json(request): Json<DaemonShutdownRequest>,
) -> Result<StatusCode, ApiError> {
    if state.daemon_identity.mode != DaemonMode::Managed
        || state.daemon_identity.instance_id != Some(request.instance_id)
    {
        return Err(ApiError(TakokitError::InvalidRequest(
            "managed daemon identity does not match".to_string(),
        )));
    }
    if let Some(sender) = state.shutdown.lock().await.take() {
        let _ = sender.send(());
    }
    Ok(StatusCode::ACCEPTED)
}

pub async fn ps(State(state): State<AppState>) -> Json<RunnersResponse<ProcessInfo>> {
    Json(RunnersResponse {
        data: state.executions.lock().await.values().cloned().collect(),
    })
}

pub async fn pick_audio_file(headers: HeaderMap) -> Result<Json<serde_json::Value>, ApiError> {
    let store = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    let initial_dir = store.workspace_root().to_path_buf();
    let selected =
        tokio::task::spawn_blocking(move || crate::native_picker::pick_audio_file(&initial_dir))
            .await
            .map_err(|error| {
                ApiError(TakokitError::Execution(format!(
                    "audio picker task failed: {error}"
                )))
            })?
            .map_err(|error| ApiError(TakokitError::Execution(error)))?;

    Ok(Json(serde_json::json!({
        "path": selected.map(|path| path.display().to_string())
    })))
}

pub async fn pick_folder(headers: HeaderMap) -> Result<Json<serde_json::Value>, ApiError> {
    let store = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    let initial_dir = store.workspace_root().to_path_buf();
    let selected =
        tokio::task::spawn_blocking(move || crate::native_picker::pick_folder(&initial_dir))
            .await
            .map_err(|error| {
                ApiError(TakokitError::Execution(format!(
                    "folder picker task failed: {error}"
                )))
            })?
            .map_err(|error| ApiError(TakokitError::Execution(error)))?;

    Ok(Json(serde_json::json!({
        "path": selected.map(|path| path.display().to_string())
    })))
}

pub async fn storage_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RunnerDetailResponse<serde_json::Value>>, ApiError> {
    let workspace = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    let storage_root = state.store.root().to_path_buf();
    let workspace_root = workspace.workspace_root().to_path_buf();
    let workspace_bytes = directory_size(&workspace_root.join(".tako"));

    let paths = vec![
        ("models", "Models", state.store.models_dir()),
        ("runners", "Runners", state.store.runners_dir()),
        ("voices", "Voices", state.store.voices_dir()),
        ("blobs", "Shared blobs", state.store.blobs_dir()),
        ("cache", "Cache", state.store.cache_dir()),
        ("logs", "Logs", state.store.logs_dir()),
        ("manifests", "Manifests", state.store.manifests_dir()),
        ("workspace", "Workspace data", workspace_root.join(".tako")),
    ];

    let entries = paths
        .into_iter()
        .map(|(id, label, path)| {
            let bytes = directory_size(&path);
            let exists = path.exists();
            serde_json::json!({
                "id": id,
                "label": label,
                "path": path,
                "bytes": bytes,
                "exists": exists,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(RunnerDetailResponse {
        data: serde_json::json!({
            "storage_root": storage_root,
            "workspace_root": workspace_root,
            "total_bytes": directory_size(state.store.root()),
            "workspace_bytes": workspace_bytes,
            "entries": entries,
        }),
    }))
}

#[derive(Debug, Deserialize)]
pub struct OpenLocationRequest {
    pub target: String,
}

pub async fn open_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<OpenLocationRequest>,
) -> Result<StatusCode, ApiError> {
    let workspace = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    let path = match request.target.as_str() {
        "storage" => state.store.root().to_path_buf(),
        "workspace" => workspace.workspace_root().to_path_buf(),
        "logs" => state.store.logs_dir(),
        "voices" => state.store.voices_dir(),
        _ => {
            return Err(ApiError(TakokitError::InvalidRequest(
                "unsupported storage location".to_string(),
            )))
        }
    };

    let path_for_task = path.clone();
    tokio::task::spawn_blocking(move || open_path(&path_for_task))
        .await
        .map_err(|error| {
            ApiError(TakokitError::Execution(format!(
                "open location task failed: {error}"
            )))
        })?
        .map_err(|error| ApiError(TakokitError::Execution(error)))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn doctor(
    State(state): State<AppState>,
) -> Json<RunnerDetailResponse<serde_json::Value>> {
    let mut checks = vec![
        doctor_check(
            "daemon",
            "build identifier",
            !state.build_id.trim().is_empty(),
            state.build_id.clone(),
        ),
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
            "build_id": state.build_id,
            "daemon": state.daemon_identity,
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

fn directory_size(path: &Path) -> u64 {
    if path.is_file() {
        return std::fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| directory_size(&entry.path()))
        .sum()
}

#[cfg(windows)]
fn open_path(path: &Path) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not open {}: {error}", path.display()))
}

#[cfg(target_os = "macos")]
fn open_path(path: &Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not open {}: {error}", path.display()))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_path(path: &Path) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not open {}: {error}", path.display()))
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
