use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub sample_path: PathBuf,
    pub created_at: u64,
    pub consent_affirmed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consent_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateVoiceProfileRequest {
    pub sample_path: PathBuf,
    pub name: String,
    #[serde(default = "default_clone_model")]
    pub model: String,
    #[serde(default)]
    pub consent_affirmed: bool,
    #[serde(default)]
    pub consent_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloneVoiceResponse {
    pub data: VoiceProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceProfilesResponse {
    pub data: Vec<VoiceProfile>,
}

fn default_clone_model() -> String {
    "chatterbox".to_string()
}
