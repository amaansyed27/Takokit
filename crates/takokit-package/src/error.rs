//! Package-layer errors and their stable core error mapping.

use serde::{Deserialize, Serialize};
use takokit_core::{CapabilityKind, ErrorCode, TakokitError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("model installation failed during {stage:?}: {source}")]
    InstallStage {
        stage: InstallFailureStage,
        #[source]
        source: Box<PackageError>,
    },
    #[error("manifest IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("manifest encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("JSON metadata error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("model is not available in the local registry: {0}")]
    ModelNotFound(String),
    #[error("runner is not available in the local registry: {0}")]
    RunnerNotFound(String),
    #[error("model is not installed: {0}")]
    ModelNotInstalled(String),
    #[error("runner is not installed: {0}")]
    RunnerPackageNotInstalled(String),
    #[error("artifact URL missing for {model}: {artifact}")]
    ArtifactUrlMissing { model: String, artifact: String },
    #[error("artifact checksum missing for {model}: {artifact}")]
    ArtifactChecksumMissing { model: String, artifact: String },
    #[error("artifact download failed for {artifact}: {reason}")]
    ArtifactDownloadFailed { artifact: String, reason: String },
    #[error("artifact checksum mismatch for {artifact}: expected {expected}, got {actual}")]
    ArtifactChecksumMismatch {
        artifact: String,
        expected: String,
        actual: String,
    },
    #[error("artifact install failed for {artifact}: {reason}")]
    ArtifactInstallFailed { artifact: String, reason: String },
    #[error("{model} does not support {capability_label}.")]
    CapabilityUnsupported {
        model: String,
        capability: CapabilityKind,
        capability_label: &'static str,
    },
    #[error("{model} supports {capability_label}, but runner {runner} is not installed or not implemented yet.")]
    RunnerNotInstalled {
        model: String,
        runner: String,
        capability: CapabilityKind,
        capability_label: &'static str,
    },
    #[error(
        "{model} supports {capability_label}, but runner {runner} is not supported on {platform}."
    )]
    RunnerUnsupportedOnPlatform {
        model: String,
        runner: String,
        capability: CapabilityKind,
        capability_label: &'static str,
        platform: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallFailureStage {
    RunnerContract,
    RunnerRuntime,
    Adapter,
    Artifacts,
    Materialization,
    FinalVerification,
}

impl PackageError {
    pub(crate) fn at_stage(stage: InstallFailureStage, source: PackageError) -> Self {
        Self::InstallStage {
            stage,
            source: Box::new(source),
        }
    }
}

impl From<PackageError> for TakokitError {
    fn from(value: PackageError) -> Self {
        match value {
            PackageError::InstallStage { stage, source } => {
                add_install_stage(stage, TakokitError::from(*source))
            }
            PackageError::ModelNotFound(id) => TakokitError::Resolution {
                code: ErrorCode::ModelNotFound,
                message: format!("model is not available in the local registry: {id}"),
            },
            PackageError::RunnerNotFound(id) => TakokitError::Resolution {
                code: ErrorCode::RunnerNotFound,
                message: format!("runner is not available in the local registry: {id}"),
            },
            PackageError::ModelNotInstalled(id) => TakokitError::Resolution {
                code: ErrorCode::ModelNotInstalled,
                message: format!("model is not installed: {id}"),
            },
            PackageError::RunnerPackageNotInstalled(id) => TakokitError::Resolution {
                code: ErrorCode::RunnerNotInstalled,
                message: format!("runner is not installed: {id}"),
            },
            error @ PackageError::ArtifactUrlMissing { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactUrlMissing,
                message: error.to_string(),
            },
            error @ PackageError::ArtifactChecksumMissing { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactChecksumMissing,
                message: error.to_string(),
            },
            error @ PackageError::ArtifactDownloadFailed { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactDownloadFailed,
                message: error.to_string(),
            },
            error @ PackageError::ArtifactChecksumMismatch { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactChecksumMismatch,
                message: error.to_string(),
            },
            error @ PackageError::ArtifactInstallFailed { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactInstallFailed,
                message: error.to_string(),
            },
            error @ PackageError::CapabilityUnsupported { .. } => TakokitError::Resolution {
                code: ErrorCode::CapabilityUnsupported,
                message: error.to_string(),
            },
            error @ PackageError::RunnerNotInstalled { .. } => TakokitError::Resolution {
                code: ErrorCode::RunnerNotInstalled,
                message: error.to_string(),
            },
            error @ PackageError::RunnerUnsupportedOnPlatform { .. } => TakokitError::Resolution {
                code: ErrorCode::RunnerUnsupportedOnPlatform,
                message: error.to_string(),
            },
            PackageError::Io(error) => TakokitError::Storage(error.to_string()),
            PackageError::Toml(error) => TakokitError::Model(error.to_string()),
            PackageError::TomlSer(error) => TakokitError::Model(error.to_string()),
            PackageError::Json(error) => TakokitError::Model(error.to_string()),
        }
    }
}

fn add_install_stage(stage: InstallFailureStage, error: TakokitError) -> TakokitError {
    let prefix = format!("model installation failed during {stage:?}: ");
    match error {
        TakokitError::Resolution { code, message } => TakokitError::Resolution {
            code,
            message: format!("{prefix}{message}"),
        },
        TakokitError::InvalidRequest(message) => {
            TakokitError::InvalidRequest(format!("{prefix}{message}"))
        }
        TakokitError::Storage(message) => TakokitError::Storage(format!("{prefix}{message}")),
        TakokitError::Model(message) => TakokitError::Model(format!("{prefix}{message}")),
        TakokitError::Execution(message) => TakokitError::Execution(format!("{prefix}{message}")),
        TakokitError::Audio(message) => TakokitError::Audio(format!("{prefix}{message}")),
        TakokitError::NotImplemented { feature, reason } => TakokitError::Resolution {
            code: ErrorCode::ArtifactInstallFailed,
            message: format!("{prefix}{feature} is not implemented yet: {reason}"),
        },
    }
}

pub type PackageResult<T> = Result<T, PackageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_download_failure_preserves_retryable_error_code() {
        let error = PackageError::at_stage(
            InstallFailureStage::RunnerRuntime,
            PackageError::ArtifactDownloadFailed {
                artifact: "whisper-runtime".into(),
                reason: "connection reset".into(),
            },
        );

        assert!(matches!(
            TakokitError::from(error),
            TakokitError::Resolution {
                code: ErrorCode::ArtifactDownloadFailed,
                ..
            }
        ));
    }

    #[test]
    fn staged_install_failure_remains_install_failure() {
        let error = PackageError::at_stage(
            InstallFailureStage::FinalVerification,
            PackageError::ArtifactInstallFailed {
                artifact: "model".into(),
                reason: "verification failed".into(),
            },
        );

        assert!(matches!(
            TakokitError::from(error),
            TakokitError::Resolution {
                code: ErrorCode::ArtifactInstallFailed,
                ..
            }
        ));
    }
}
