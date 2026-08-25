from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    if old not in text:
        raise RuntimeError(f"expected block missing in {path}")
    file.write_text(text.replace(old, new, 1), encoding="utf-8")


replace(
    "crates/takokit-models/src/rvc_voice_service/packages.rs",
    """    let signature = Signature::from_slice(
        &hex::decode(&signature.signature_hex).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let fingerprint = hex::encode(Sha256::digest(verifying.as_bytes()));
    Ok(fingerprint == signature.signer_fingerprint
        && verifying.verify(manifest, &signature).is_ok())""",
    """    let signer_fingerprint = signature.signer_fingerprint.clone();
    let parsed_signature = Signature::from_slice(
        &hex::decode(&signature.signature_hex).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let fingerprint = hex::encode(Sha256::digest(verifying.as_bytes()));
    Ok(fingerprint == signer_fingerprint
        && verifying.verify(manifest, &parsed_signature).is_ok())""",
)

replace(
    "crates/takokit-server/src/handlers/rvc_voices.rs",
    "use axum::extract::{Path as AxumPath, Query};",
    "use axum::{extract::{Path as AxumPath, Query}, http::HeaderMap};",
)
replace(
    "crates/takokit-server/src/handlers/rvc_voices.rs",
    """pub async fn rvc_test_voice(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Json(request): Json<TestRvcVoiceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let output_dir = request
        .workspace_root
        .as_ref()
        .map(|root| root.join("outputs"))
        .unwrap_or_else(|| state.store.root().join("outputs"));""",
    """pub async fn rvc_test_voice(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    headers: HeaderMap,
    Json(request): Json<TestRvcVoiceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let output_dir = if request.workspace_root.is_some() {
        request.workspace_root.as_ref().unwrap().join("outputs")
    } else {
        crate::RequestWorkspace::from_headers(&headers, "RVC voice test")
            .map_err(ApiError)?
            .outputs_dir()
    };""",
)
