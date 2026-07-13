use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionTask {
    TextToSpeech,
    SpeechToText,
    VoiceCloning,
    VoiceTraining,
    VoiceConversion,
    ModelInstall,
    RunnerInstall,
    Diagnostics,
    System,
}

impl SessionTask {
    pub fn label(self) -> &'static str {
        match self {
            Self::TextToSpeech => "Text to speech",
            Self::SpeechToText => "Speech to text",
            Self::VoiceCloning => "Voice cloning",
            Self::VoiceTraining => "Voice training",
            Self::VoiceConversion => "Voice conversion",
            Self::ModelInstall => "Model install",
            Self::RunnerInstall => "Runner install",
            Self::Diagnostics => "Diagnostics",
            Self::System => "System",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventState {
    Started,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionEvent {
    pub id: Uuid,
    pub session_id: Uuid,
    pub timestamp: u64,
    pub task: SessionTask,
    pub state: SessionEventState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewSessionEvent {
    pub task: SessionTask,
    pub state: SessionEventState,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub source_path: Option<PathBuf>,
    #[serde(default)]
    pub output_path: Option<PathBuf>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSummary {
    pub id: Uuid,
    pub title: String,
    pub workspace_root: PathBuf,
    pub created_at: u64,
    pub updated_at: u64,
    pub event_count: usize,
    pub output_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_task: Option<SessionTask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionRecord {
    pub summary: SessionSummary,
    pub events: Vec<SessionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionOpenRequest {
    pub workspace: PathBuf,
    #[serde(default)]
    pub session_id: Option<Uuid>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionOpenResponse {
    pub data: SessionRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionsResponse {
    pub data: Vec<SessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDetailResponse {
    pub data: SessionRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDeleteResponse {
    pub id: Uuid,
    pub removed: bool,
}
