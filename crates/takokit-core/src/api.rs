use crate::{ModelInfo, VoiceInfo};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonIdentity {
    pub instance_id: Option<Uuid>,
    pub mode: DaemonMode,
    pub pid: u32,
    pub executable: PathBuf,
    pub storage_root: PathBuf,
    pub host: String,
    pub port: u16,
    pub started_at: u64,
    pub log_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DaemonMode {
    Managed,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonShutdownRequest {
    pub instance_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessInfo {
    pub execution_id: Uuid,
    pub model: String,
    pub task: String,
    pub started_at: u64,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub ok: bool,
    pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub service: String,
    pub version: String,
    pub server: String,
    pub storage_root: PathBuf,
    pub installed_models: usize,
    pub voices: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilitiesResponse {
    pub data: Vec<crate::CapabilityInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelsResponse {
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelDetailResponse {
    pub data: ModelInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnersResponse<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerDetailResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullModelRequest {
    pub model: String,
    #[serde(default)]
    pub metadata_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRunnerRequest {
    pub runner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallStepState {
    NotRequested,
    AlreadyReady,
    Installed,
    Repaired,
    MetadataOnly,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallStep {
    pub state: InstallStepState,
    pub newly_installed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInstallReport {
    pub model_id: String,
    pub required_runner: String,
    pub required_adapter: Option<String>,
    pub artifacts: InstallStep,
    pub runner_contract: InstallStep,
    pub runner_runtime: InstallStep,
    pub adapter: Option<InstallStep>,
    pub executable: bool,
    pub missing: Vec<String>,
    pub logs_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullModelResponse {
    pub id: String,
    pub installed: bool,
    pub manifest_path: PathBuf,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoicesResponse {
    pub data: Vec<VoiceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeechRequest {
    pub model: String,
    pub input: String,
    pub voice: Option<String>,
    pub response_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechResponse {
    pub id: Uuid,
    pub model: String,
    pub voice: Option<String>,
    pub engine: String,
    pub output_path: PathBuf,
    pub content_type: String,
    pub bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptionRequest {
    pub file_path: PathBuf,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptionResponse {
    pub id: Uuid,
    pub model: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloneVoiceRequest {
    pub sample_path: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrainVoiceRequest {
    pub samples_path: PathBuf,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_request_matches_openai_compatible_shape() {
        let request = SpeechRequest {
            model: "kokoro".to_string(),
            input: "Hello from Takokit".to_string(),
            voice: Some("default".to_string()),
            response_format: Some("wav".to_string()),
        };

        let json = serde_json::to_value(request).expect("serializes");

        assert_eq!(json["model"], "kokoro");
        assert_eq!(json["input"], "Hello from Takokit");
        assert_eq!(json["voice"], "default");
        assert_eq!(json["response_format"], "wav");
    }

    #[test]
    fn pull_model_request_keeps_metadata_only_optional() {
        let request: PullModelRequest =
            serde_json::from_str(r#"{"model":"piper-lessac"}"#).expect("pull request");

        assert_eq!(request.model, "piper-lessac");
        assert!(!request.metadata_only);
    }

    #[test]
    fn model_install_report_serializes_typed_steps() {
        let report = ModelInstallReport {
            model_id: "fixture-model".into(),
            required_runner: "fixture-runner".into(),
            required_adapter: None,
            artifacts: InstallStep {
                state: InstallStepState::AlreadyReady,
                newly_installed: false,
                detail: "verified".into(),
            },
            runner_contract: InstallStep {
                state: InstallStepState::NotRequested,
                newly_installed: false,
                detail: "fixture".into(),
            },
            runner_runtime: InstallStep {
                state: InstallStepState::NotRequested,
                newly_installed: false,
                detail: "fixture".into(),
            },
            adapter: None,
            executable: false,
            missing: vec!["runner runtime".into()],
            logs_path: PathBuf::from("logs"),
        };
        let json = serde_json::to_value(report).expect("serialize report");
        assert_eq!(json["artifacts"]["state"], "already-ready");
        for key in [
            "model_id",
            "required_runner",
            "required_adapter",
            "artifacts",
            "runner_contract",
            "runner_runtime",
            "adapter",
            "executable",
            "missing",
            "logs_path",
        ] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
    }
}
