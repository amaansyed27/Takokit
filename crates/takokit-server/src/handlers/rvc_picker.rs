use super::*;
use axum::{extract::Query, http::HeaderMap};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RvcPickerQuery {
    pub kind: String,
}

pub async fn pick_rvc_artifact(
    headers: HeaderMap,
    Query(query): Query<RvcPickerQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let store = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    let initial_dir = store.workspace_root().to_path_buf();
    let kind = query.kind;
    let selected = tokio::task::spawn_blocking(move || match kind.as_str() {
        "checkpoint" => crate::native_picker::pick_rvc_checkpoint(&initial_dir),
        "index" => crate::native_picker::pick_rvc_index(&initial_dir),
        "package" => crate::native_picker::pick_rvc_package(&initial_dir),
        _ => Err("RVC picker kind must be checkpoint, index, or package".to_string()),
    })
    .await
    .map_err(|error| {
        ApiError(TakokitError::Execution(format!(
            "RVC artifact picker task failed: {error}"
        )))
    })?
    .map_err(|error| ApiError(TakokitError::InvalidRequest(error)))?;

    Ok(Json(serde_json::json!({
        "path": selected.map(|path| path.display().to_string())
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_query_deserializes_known_kind() {
        let query: RvcPickerQuery = serde_json::from_value(serde_json::json!({
            "kind": "checkpoint"
        }))
        .unwrap();
        assert_eq!(query.kind, "checkpoint");
    }
}
