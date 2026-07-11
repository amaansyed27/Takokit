use super::*;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "takokit".to_string(),
    })
}

pub async fn status(State(state): State<AppState>) -> Json<takokit_core::RuntimeStatus> {
    Json(state.status())
}

pub async fn daemon_identity(State(state): State<AppState>) -> Json<DaemonIdentity> {
    Json(state.daemon_identity.clone())
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
