use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

mod package;
mod training;

pub use package::*;
pub use training::*;

pub const RVC_VOICE_SCHEMA_VERSION: u32 = 1;
pub const TAKOVOICE_SCHEMA_VERSION: u32 = 1;
pub const RVC_VERIFIED_SAMPLE_RATE_HZ: u32 = 40_000;
pub const RVC_VERIFIED_MODEL_VERSION: &str = "v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RvcVoiceProjectState {
    Created,
    CollectingSamples,
    ReadyForPreparation,
    Preprocessing,
    ExtractingF0,
    ExtractingFeatures,
    ReadyToTrain,
    Training,
    BuildingIndex,
    ValidatingArtifacts,
    Ready,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcVoiceProject {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    pub engine: String,
    pub state: RvcVoiceProjectState,
    pub imported: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub latest_job_id: Option<Uuid>,
    pub active_checkpoint_id: Option<Uuid>,
    pub active_index_id: Option<Uuid>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcVoiceConsent {
    pub voice_id: Uuid,
    pub affirmed: bool,
    pub note: Option<String>,
    pub recorded_at: u64,
    pub statement: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RvcSampleState {
    Imported,
    Inspected,
    Prepared,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RvcSampleWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RvcAudioInspection {
    pub duration_ms: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub codec: Option<String>,
    pub container: Option<String>,
    pub peak_dbfs: Option<f32>,
    pub rms_dbfs: Option<f32>,
    pub silence_ratio: Option<f32>,
    pub clipped_ratio: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcVoiceSample {
    pub id: Uuid,
    pub voice_id: Uuid,
    pub display_name: String,
    pub source_path: PathBuf,
    pub managed_path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
    pub imported_at: u64,
    pub included: bool,
    pub state: RvcSampleState,
    pub inspection: Option<RvcAudioInspection>,
    pub warnings: Vec<RvcSampleWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RvcDatasetInspection {
    pub voice_id: Uuid,
    pub sample_count: usize,
    pub included_sample_count: usize,
    pub usable_duration_ms: u64,
    pub warning_count: usize,
    pub duplicate_count: usize,
    pub ready_for_preparation: bool,
    pub warnings: Vec<RvcSampleWarning>,
    pub inspected_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcCheckpoint {
    pub id: Uuid,
    pub voice_id: Uuid,
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
    pub epoch: Option<u32>,
    pub sample_rate_hz: Option<u32>,
    pub model_version: Option<String>,
    pub f0: Option<bool>,
    pub created_at: u64,
    pub valid_for_inference: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcIndexArtifact {
    pub id: Uuid,
    pub voice_id: Uuid,
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
    pub checkpoint_id: Option<Uuid>,
    pub created_at: u64,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedRvcVoice {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub checkpoint_id: Uuid,
    pub index_id: Option<Uuid>,
    pub ready_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcVoiceDetail {
    pub project: RvcVoiceProject,
    pub samples: Vec<RvcVoiceSample>,
    pub dataset: RvcDatasetInspection,
    pub managed: Option<ManagedRvcVoice>,
    pub checkpoints: Vec<RvcCheckpoint>,
    pub indexes: Vec<RvcIndexArtifact>,
    pub active_job: Option<RvcTrainingJob>,
    pub conversion_target: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRvcVoiceRequest {
    pub name: String,
    pub consent_affirmed: bool,
    pub consent_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRvcSamplesRequest {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetRvcSampleIncludedRequest {
    pub included: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRvcVoiceRequest {
    pub checkpoint: PathBuf,
    pub index: Option<PathBuf>,
    pub name: String,
    pub consent_affirmed: bool,
    pub consent_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectRvcCheckpointRequest {
    pub checkpoint_id: Uuid,
    pub index_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRvcVoiceRequest {
    pub input: PathBuf,
    pub workspace_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveRvcVoiceRequest {
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcVoiceRemovalPreview {
    pub voice_id: Uuid,
    pub name: String,
    pub bytes: u64,
    pub files: usize,
    pub dry_run: bool,
    pub removed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_state_roundtrips() {
        let value = serde_json::to_string(&RvcVoiceProjectState::ExtractingFeatures).unwrap();
        assert_eq!(value, "\"extracting_features\"");
    }
}
