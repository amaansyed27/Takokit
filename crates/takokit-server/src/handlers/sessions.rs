use super::*;
use crate::workspace::store_from_headers;
use axum::{
    body::Body,
    extract::Query,
    http::{header, HeaderMap, Response},
};
use serde::Deserialize;
use takokit_core::{
    SessionDeleteResponse, SessionDetailResponse, SessionOpenRequest, SessionOpenResponse,
    SessionsResponse,
};
use takokit_store::WorkspaceStore;
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
pub struct SessionSearchQuery {
    pub q: Option<String>,
}

pub async fn open_session(
    Json(request): Json<SessionOpenRequest>,
) -> Result<Json<SessionOpenResponse>, ApiError> {
    let store = WorkspaceStore::new(request.workspace);
    store.ensure_layout().map_err(ApiError)?;
    let record = store
        .open_session(request.session_id, request.title.as_deref())
        .map_err(ApiError)?;
    Ok(Json(SessionOpenResponse { data: record }))
}

pub async fn sessions(
    headers: HeaderMap,
    Query(query): Query<SessionSearchQuery>,
) -> Result<Json<SessionsResponse>, ApiError> {
    let store = store_from_headers(&headers).map_err(ApiError)?;
    let data = store
        .list_sessions(query.q.as_deref())
        .map_err(ApiError)?;
    Ok(Json(SessionsResponse { data }))
}

pub async fn session(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<SessionDetailResponse>, ApiError> {
    let store = store_from_headers(&headers).map_err(ApiError)?;
    let data = store.read_session(id).map_err(ApiError)?;
    Ok(Json(SessionDetailResponse { data }))
}

pub async fn remove_session(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<SessionDeleteResponse>, ApiError> {
    let store = store_from_headers(&headers).map_err(ApiError)?;
    let removed = store.remove_session(id).map_err(ApiError)?;
    Ok(Json(SessionDeleteResponse { id, removed }))
}

pub async fn session_output(
    Path((id, filename)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Result<Response<Body>, ApiError> {
    validate_filename(&filename)?;
    let store = store_from_headers(&headers).map_err(ApiError)?;
    let path = store.session_outputs_dir(id).join(&filename);
    let bytes = tokio::fs::read(&path).await.map_err(|error| {
        ApiError(TakokitError::Storage(format!(
            "could not read session output {}: {error}",
            path.display()
        )))
    })?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type(&filename))
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{filename}\""),
        )
        .body(Body::from(bytes))
        .map_err(|error| ApiError(TakokitError::Storage(error.to_string())))
}

fn validate_filename(filename: &str) -> Result<(), ApiError> {
    if filename.is_empty()
        || filename == "."
        || filename == ".."
        || filename.contains('/')
        || filename.contains('\\')
    {
        return Err(ApiError(TakokitError::InvalidRequest(
            "session output must be a single filename".to_string(),
        )));
    }
    Ok(())
}

fn content_type(filename: &str) -> &'static str {
    match std::path::Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("json") => "application/json",
        Some("txt") | Some("md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_filename_validation_blocks_path_traversal() {
        assert!(validate_filename("speech.wav").is_ok());
        assert!(validate_filename("../speech.wav").is_err());
        assert!(validate_filename("folder\\speech.wav").is_err());
    }
}
