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
pub struct CloneVoiceResponse {
    pub data: VoiceProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceProfilesResponse {
    pub data: Vec<VoiceProfile>,
}
