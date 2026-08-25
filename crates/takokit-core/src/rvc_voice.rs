use crate::RvcF0Method;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub const RVC_VOICE_SCHEMA_VERSION: u32 = 1;
pub const TAKOVOICE_SCHEMA_VERSION: u32 = 1;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RvcTrainingPreset {
    Quick,
    Balanced,
    HighQuality,
    Custom,
}

impl RvcTrainingPreset {
    pub const ALL: [Self; 4] = [Self::Quick, Self::Balanced, Self::HighQuality, Self::Custom];

    pub fn id(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Balanced => "balanced",
            Self::HighQuality => "high-quality",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RvcTrainingDevice {
    Auto,
    Cuda,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RvcTrainingPrecision {
    Auto,
    Fp16,
    Fp32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcTrainingConfig {
    pub preset: RvcTrainingPreset,
    pub epochs: u32,
    pub batch_size: u32,
    pub save_every_epochs: u32,
    pub sample_rate_hz: u32,
    pub model_version: String,
    pub f0_enabled: bool,
    pub f0_method: RvcF0Method,
    pub device: RvcTrainingDevice,
    pub precision: RvcTrainingPrecision,
    pub cache_dataset_on_gpu: bool,
}

impl RvcTrainingConfig {
    pub fn preset(preset: RvcTrainingPreset) -> Option<Self> {
        let (epochs, batch_size, save_every_epochs) = match preset {
            RvcTrainingPreset::Quick => (20, 4, 5),
            RvcTrainingPreset::Balanced => (200, 8, 10),
            RvcTrainingPreset::HighQuality => (400, 8, 20),
            RvcTrainingPreset::Custom => return None,
        };
        Some(Self {
            preset,
            epochs,
            batch_size,
            save_every_epochs,
            sample_rate_hz: 40_000,
            model_version: "v2".to_string(),
            f0_enabled: true,
            f0_method: RvcF0Method::Rmvpe,
            device: RvcTrainingDevice::Auto,
            precision: RvcTrainingPrecision::Auto,
            cache_dataset_on_gpu: false,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.epochs == 0 || self.epochs > 1200 {
            return Err("epochs must be between 1 and 1200".to_string());
        }
        if self.batch_size == 0 {
            return Err("batch size must be at least 1".to_string());
        }
        if self.save_every_epochs == 0 {
            return Err("checkpoint interval must be at least 1 epoch".to_string());
        }
        if !matches!(self.sample_rate_hz, 32_000 | 40_000 | 48_000) {
            return Err("RVC sample rate must be 32000, 40000, or 48000 Hz".to_string());
        }
        if !matches!(self.model_version.as_str(), "v1" | "v2") {
            return Err("RVC model version must be v1 or v2".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcTrainingPresetInfo {
    pub id: RvcTrainingPreset,
    pub label: String,
    pub description: String,
    pub config: Option<RvcTrainingConfig>,
}

pub fn rvc_training_presets() -> Vec<RvcTrainingPresetInfo> {
    vec![
        RvcTrainingPresetInfo { id: RvcTrainingPreset::Quick, label: "Quick test".into(), description: "A short 20-epoch run for validating the dataset, runtime, and training path. Not a production-quality claim.".into(), config: RvcTrainingConfig::preset(RvcTrainingPreset::Quick) },
        RvcTrainingPresetInfo { id: RvcTrainingPreset::Balanced, label: "Balanced".into(), description: "A 200-epoch general starting point using the documented RVC v2/40k training path.".into(), config: RvcTrainingConfig::preset(RvcTrainingPreset::Balanced) },
        RvcTrainingPresetInfo { id: RvcTrainingPreset::HighQuality, label: "High quality".into(), description: "An extended 400-epoch run for clean, sufficiently long datasets. More training does not guarantee better perceptual quality.".into(), config: RvcTrainingConfig::preset(RvcTrainingPreset::HighQuality) },
        RvcTrainingPresetInfo { id: RvcTrainingPreset::Custom, label: "Custom".into(), description: "Advanced RVC parameters supplied explicitly by the user.".into(), config: None },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RvcPreflightClass {
    Recommended,
    Possible,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcHardwarePreflight {
    pub class: RvcPreflightClass,
    pub cpu: String,
    pub gpu: Option<String>,
    pub backend: String,
    pub vram_bytes: Option<u64>,
    pub system_ram_bytes: Option<u64>,
    pub available_disk_bytes: u64,
    pub dataset_duration_ms: u64,
    pub selected_preset: RvcTrainingPreset,
    pub resolved_device: RvcTrainingDevice,
    pub resolved_precision: RvcTrainingPrecision,
    pub resource_category: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RvcTrainingJobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RvcTrainingStage {
    ValidateSamples,
    Preprocess,
    ExtractF0,
    ExtractFeatures,
    Train,
    BuildIndex,
    ValidateArtifacts,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcTrainingJob {
    pub id: Uuid,
    pub voice_id: Uuid,
    pub config: RvcTrainingConfig,
    pub status: RvcTrainingJobStatus,
    pub stage: RvcTrainingStage,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub owner_pid: Option<u32>,
    pub child_pid: Option<u32>,
    pub log_path: PathBuf,
    pub checkpoint_ids: Vec<Uuid>,
    pub failure: Option<String>,
    pub cancellation_requested: bool,
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

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRvcVoiceRequest {
    pub name: String,
    pub consent_affirmed: bool,
    pub consent_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddRvcSamplesRequest {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetRvcSampleIncludedRequest {
    pub included: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartRvcTrainingRequest {
    pub preset: RvcTrainingPreset,
    pub custom: Option<RvcTrainingConfig>,
}

impl StartRvcTrainingRequest {
    pub fn resolve(&self) -> Result<RvcTrainingConfig, String> {
        let config = match self.preset {
            RvcTrainingPreset::Custom => self.custom.clone().ok_or_else(|| "custom training requires an explicit configuration".to_string())?,
            preset => RvcTrainingConfig::preset(preset).expect("non-custom preset"),
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportRvcVoiceRequest {
    pub checkpoint: PathBuf,
    pub index: Option<PathBuf>,
    pub name: String,
    pub consent_affirmed: bool,
    pub consent_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SelectRvcCheckpointRequest {
    pub checkpoint_id: Uuid,
    pub index_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportRvcVoiceRequest {
    pub output: PathBuf,
    #[serde(default)]
    pub sign: bool,
    #[serde(default)]
    pub include_reference: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VerifyRvcPackageRequest {
    pub package: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TestRvcVoiceRequest {
    pub input: PathBuf,
    pub workspace_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcPackageVerification {
    pub schema_version: u32,
    pub package_path: PathBuf,
    pub signed: bool,
    pub signature_valid: Option<bool>,
    pub signer_fingerprint: Option<String>,
    pub hashes_valid: bool,
    pub voice_name: Option<String>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_real_rvc_parameters() {
        let quick = RvcTrainingConfig::preset(RvcTrainingPreset::Quick).unwrap();
        let balanced = RvcTrainingConfig::preset(RvcTrainingPreset::Balanced).unwrap();
        let high = RvcTrainingConfig::preset(RvcTrainingPreset::HighQuality).unwrap();
        assert_eq!(quick.epochs, 20);
        assert_eq!(balanced.epochs, 200);
        assert!(high.epochs > balanced.epochs);
        for config in [quick, balanced, high] {
            assert_eq!(config.sample_rate_hz, 40_000);
            assert_eq!(config.model_version, "v2");
            assert_eq!(config.f0_method, RvcF0Method::Rmvpe);
            config.validate().unwrap();
        }
    }

    #[test]
    fn custom_requires_explicit_config() {
        let request = StartRvcTrainingRequest { preset: RvcTrainingPreset::Custom, custom: None };
        assert!(request.resolve().is_err());
    }

    #[test]
    fn project_state_roundtrips() {
        let value = serde_json::to_string(&RvcVoiceProjectState::ExtractingFeatures).unwrap();
        assert_eq!(value, "\"extracting_features\"");
    }
}
