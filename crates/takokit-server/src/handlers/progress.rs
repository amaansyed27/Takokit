use super::*;

pub async fn model_pull_with_progress(
    State(state): State<AppState>,
    Json(request): Json<PullModelRequest>,
) -> Result<Json<ModelInstallReport>, ApiError> {
    let package_registry = state.package_registry.clone();
    let installed_registry = state.installed_registry.clone();
    let takokit_root = state.store.root().to_path_buf();
    let model = request.model;
    let metadata_only = request.metadata_only;

    let report = tokio::task::spawn_blocking(move || {
        install_model_complete(
            &package_registry,
            &installed_registry,
            &takokit_root,
            &model,
            InstallModelOptions { metadata_only },
        )
    })
    .await
    .map_err(|error| ApiError(TakokitError::Execution(error.to_string())))?
    .map_err(Into::into)
    .map_err(ApiError)?;

    Ok(Json(report))
}

pub async fn model_pull_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<takokit_package::InstallProgress>, StatusCode> {
    takokit_package::read_model_progress(state.store.root(), &id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
