use super::*;

pub async fn model_pull_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<takokit_package::InstallProgress>, StatusCode> {
    takokit_package::read_model_progress(state.store.root(), &id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
