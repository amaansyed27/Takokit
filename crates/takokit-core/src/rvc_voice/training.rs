use super::*;
use crate::RvcF0Method;

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
            sample_rate_hz: RVC_VERIFIED_SAMPLE_RATE_HZ,
            model_version: RVC_VERIFIED_MODEL_VERSION.to_string(),
            f0_enabled: true,
            f0_method: RvcF0Method::Rmvpe,
            device: RvcTrainingDevice::Auto,
            precision: RvcTrainingPrecision::Auto,
            cache_dataset_on_gpu: false,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(1..=1200).contains(&self.epochs) {
            return Err("epochs must be between 1 and 1200".into());
        }
        if !(1..=64).contains(&self.batch_size) {
            return Err("batch size must be between 1 and 64".into());
        }
        if self.save_every_epochs == 0 || self.save_every_epochs > self.epochs {
            return Err("checkpoint interval must be between 1 and total epochs".into());
        }
        if self.sample_rate_hz != RVC_VERIFIED_SAMPLE_RATE_HZ
            || self.model_version != RVC_VERIFIED_MODEL_VERSION
        {
            return Err("Slice 3 training is verified only for RVC v2 at 40 kHz".into());
        }
        if !self.f0_enabled || self.f0_method != RvcF0Method::Rmvpe {
            return Err("Slice 3 training requires the verified RMVPE F0 pipeline".into());
        }
        if self.device == RvcTrainingDevice::Cpu && self.precision == RvcTrainingPrecision::Fp16 {
            return Err(
                "FP16 is not supported for the verified CPU training path; use auto or fp32".into(),
            );
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
    [
        (
            RvcTrainingPreset::Quick,
            "Quick test",
            "20 epochs for validating samples, preparation, GPU/runtime compatibility, checkpoint creation, and conversion. Not a final-quality guarantee.",
        ),
        (
            RvcTrainingPreset::Balanced,
            "Balanced",
            "200 epochs. Recommended normal starting point for the verified RVC v2/40 kHz/RMVPE path.",
        ),
        (
            RvcTrainingPreset::HighQuality,
            "High quality",
            "400 epochs. An extended run with higher resource/time cost; extra training does not guarantee perceptual improvement.",
        ),
        (
            RvcTrainingPreset::Custom,
            "Custom",
            "Advanced epochs, batch size, checkpoint cadence, device, precision, and GPU-cache controls inside Takokit's verified v2/40 kHz/RMVPE envelope.",
        ),
    ]
    .into_iter()
    .map(|(id, label, description)| RvcTrainingPresetInfo {
        id,
        label: label.into(),
        description: description.into(),
        config: RvcTrainingConfig::preset(id),
    })
    .collect()
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
    ReadyToTrain,
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
    #[serde(default)]
    pub current_epoch: Option<u32>,
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
pub struct StartRvcTrainingRequest {
    pub preset: RvcTrainingPreset,
    pub custom: Option<RvcTrainingConfig>,
}

impl StartRvcTrainingRequest {
    pub fn resolve(&self) -> Result<RvcTrainingConfig, String> {
        let config = match self.preset {
            RvcTrainingPreset::Custom => self
                .custom
                .clone()
                .ok_or_else(|| "custom training requires an explicit configuration".to_string())?,
            preset => RvcTrainingConfig::preset(preset).expect("non-custom preset"),
        };
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_share_verified_contract() {
        for preset in [
            RvcTrainingPreset::Quick,
            RvcTrainingPreset::Balanced,
            RvcTrainingPreset::HighQuality,
        ] {
            let config = RvcTrainingConfig::preset(preset).unwrap();
            assert_eq!(config.sample_rate_hz, 40_000);
            assert_eq!(config.model_version, "v2");
            assert_eq!(config.f0_method, RvcF0Method::Rmvpe);
            config.validate().unwrap();
        }
    }

    #[test]
    fn unsupported_training_modes_are_rejected_at_domain_boundary() {
        let mut config = RvcTrainingConfig::preset(RvcTrainingPreset::Quick).unwrap();
        config.sample_rate_hz = 48_000;
        assert!(config.validate().is_err());
        config.sample_rate_hz = 40_000;
        config.model_version = "v1".into();
        assert!(config.validate().is_err());
        config.model_version = "v2".into();
        config.f0_method = RvcF0Method::Harvest;
        assert!(config.validate().is_err());
        config.f0_method = RvcF0Method::Rmvpe;
        config.device = RvcTrainingDevice::Cpu;
        config.precision = RvcTrainingPrecision::Fp16;
        assert!(config.validate().is_err());
    }

    #[test]
    fn custom_requires_explicit_config() {
        let request = StartRvcTrainingRequest {
            preset: RvcTrainingPreset::Custom,
            custom: None,
        };
        assert!(request.resolve().is_err());
    }
}
