use super::*;

#[derive(Debug)]
pub struct ApiError(pub TakokitError);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self.0 {
            TakokitError::Resolution { code, .. } => match code {
                ErrorCode::CapabilityUnsupported => StatusCode::BAD_REQUEST,
                ErrorCode::ArtifactUrlMissing
                | ErrorCode::ArtifactChecksumMissing
                | ErrorCode::ArtifactInstallFailed
                | ErrorCode::ArtifactConfigInvalid => StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::ArtifactChecksumMismatch | ErrorCode::ArtifactDownloadFailed => {
                    StatusCode::BAD_GATEWAY
                }
                ErrorCode::ArtifactMissing | ErrorCode::ArtifactNotDownloaded => {
                    StatusCode::BAD_REQUEST
                }
                ErrorCode::ModelNotFound
                | ErrorCode::ModelNotInstalled
                | ErrorCode::RunnerNotFound => StatusCode::NOT_FOUND,
                ErrorCode::RunnerNotInstalled
                | ErrorCode::RunnerUnsupportedOnPlatform
                | ErrorCode::RuntimeNotReady
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
            TakokitError::Execution(_) => "execution_error",
            TakokitError::Audio(_) => "audio_error",
        };
        let retryable = matches!(
            &self.0,
            TakokitError::Resolution {
                code: ErrorCode::ArtifactDownloadFailed,
                ..
            }
        ) || matches!(
            status,
            StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT
        );
        let body = Json(serde_json::json!({
            "error": {
                "code": code,
                "message": self.0.to_string(),
                "retryable": retryable
            }
        }));

        (status, body).into_response()
    }
}
