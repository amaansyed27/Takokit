use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;
use takokit_core::{
    RvcTrainingConfig, RvcTrainingDevice, RvcTrainingPrecision, RvcTrainingPreset,
    StartRvcTrainingRequest,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum RvcTrainingPresetArg {
    Quick,
    Balanced,
    HighQuality,
    Custom,
}

impl From<RvcTrainingPresetArg> for RvcTrainingPreset {
    fn from(value: RvcTrainingPresetArg) -> Self {
        match value {
            RvcTrainingPresetArg::Quick => Self::Quick,
            RvcTrainingPresetArg::Balanced => Self::Balanced,
            RvcTrainingPresetArg::HighQuality => Self::HighQuality,
            RvcTrainingPresetArg::Custom => Self::Custom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum RvcTrainingDeviceArg {
    Auto,
    Cuda,
    Cpu,
}

impl From<RvcTrainingDeviceArg> for RvcTrainingDevice {
    fn from(value: RvcTrainingDeviceArg) -> Self {
        match value {
            RvcTrainingDeviceArg::Auto => Self::Auto,
            RvcTrainingDeviceArg::Cuda => Self::Cuda,
            RvcTrainingDeviceArg::Cpu => Self::Cpu,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum RvcTrainingPrecisionArg {
    Auto,
    Fp16,
    Fp32,
}

impl From<RvcTrainingPrecisionArg> for RvcTrainingPrecision {
    fn from(value: RvcTrainingPrecisionArg) -> Self {
        match value {
            RvcTrainingPrecisionArg::Auto => Self::Auto,
            RvcTrainingPrecisionArg::Fp16 => Self::Fp16,
            RvcTrainingPrecisionArg::Fp32 => Self::Fp32,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct RvcTrainingOptions {
    #[arg(long, value_enum, default_value = "balanced")]
    pub(crate) preset: RvcTrainingPresetArg,
    #[arg(long)]
    pub(crate) epochs: Option<u32>,
    #[arg(long)]
    pub(crate) batch_size: Option<u32>,
    #[arg(long)]
    pub(crate) save_every_epochs: Option<u32>,
    #[arg(long, value_enum, default_value = "auto")]
    pub(crate) device: RvcTrainingDeviceArg,
    #[arg(long, value_enum, default_value = "auto")]
    pub(crate) precision: RvcTrainingPrecisionArg,
    #[arg(long)]
    pub(crate) cache_dataset_on_gpu: bool,
}

impl RvcTrainingOptions {
    pub(crate) fn request(&self) -> anyhow::Result<StartRvcTrainingRequest> {
        let preset: RvcTrainingPreset = self.preset.into();
        if preset != RvcTrainingPreset::Custom {
            if self.epochs.is_some()
                || self.batch_size.is_some()
                || self.save_every_epochs.is_some()
                || self.device != RvcTrainingDeviceArg::Auto
                || self.precision != RvcTrainingPrecisionArg::Auto
                || self.cache_dataset_on_gpu
            {
                return Err(anyhow::anyhow!(
                    "advanced training overrides require --preset custom"
                ));
            }
            return Ok(StartRvcTrainingRequest {
                preset,
                custom: None,
            });
        }
        let mut config = RvcTrainingConfig::preset(RvcTrainingPreset::Balanced)
            .expect("balanced RVC training preset");
        config.preset = RvcTrainingPreset::Custom;
        config.epochs = self.epochs.unwrap_or(config.epochs);
        config.batch_size = self.batch_size.unwrap_or(config.batch_size);
        config.save_every_epochs = self
            .save_every_epochs
            .unwrap_or(config.save_every_epochs.min(config.epochs));
        config.device = self.device.into();
        config.precision = self.precision.into();
        config.cache_dataset_on_gpu = self.cache_dataset_on_gpu;
        config
            .validate()
            .map_err(|message| anyhow::anyhow!(message))?;
        Ok(StartRvcTrainingRequest {
            preset,
            custom: Some(config),
        })
    }

    pub(crate) fn config(&self) -> anyhow::Result<RvcTrainingConfig> {
        self.request()?
            .resolve()
            .map_err(|message| anyhow::anyhow!(message))
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum RvcSampleCommand {
    Add {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    List,
    Remove {
        sample: Uuid,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum RvcVoiceCommand {
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        consent: bool,
        #[arg(long)]
        consent_note: Option<String>,
    },
    List,
    Show {
        voice: String,
    },
    Samples {
        voice: String,
        #[command(subcommand)]
        command: RvcSampleCommand,
    },
    Inspect {
        voice: String,
    },
    Presets,
    Preflight {
        voice: String,
        #[command(flatten)]
        training: RvcTrainingOptions,
    },
    Prepare {
        voice: String,
        #[command(flatten)]
        training: RvcTrainingOptions,
    },
    Train {
        voice: String,
        #[command(flatten)]
        training: RvcTrainingOptions,
    },
    Status {
        voice: String,
    },
    Logs {
        voice: String,
        #[arg(long, default_value_t = 262_144)]
        max_bytes: usize,
    },
    Cancel {
        voice: String,
    },
    Recover {
        voice: String,
    },
    Checkpoints {
        voice: String,
    },
    Indexes {
        voice: String,
    },
    Activate {
        voice: String,
        checkpoint: Uuid,
        #[arg(long)]
        index: Option<Uuid>,
    },
    Test {
        voice: String,
        input: PathBuf,
    },
    Import {
        checkpoint: PathBuf,
        #[arg(long)]
        index: Option<PathBuf>,
        #[arg(long)]
        name: String,
        #[arg(long)]
        consent: bool,
        #[arg(long)]
        consent_note: Option<String>,
    },
    Export {
        voice: String,
        package: PathBuf,
        #[arg(long)]
        sign: bool,
        #[arg(long)]
        include_reference: bool,
    },
    Verify {
        package: PathBuf,
    },
    ImportPackage {
        package: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        consent: bool,
        #[arg(long)]
        consent_note: Option<String>,
    },
    Remove {
        voice: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_request_keeps_backend_owned_balanced_values() {
        let options = RvcTrainingOptions {
            preset: RvcTrainingPresetArg::Balanced,
            epochs: None,
            batch_size: None,
            save_every_epochs: None,
            device: RvcTrainingDeviceArg::Auto,
            precision: RvcTrainingPrecisionArg::Auto,
            cache_dataset_on_gpu: false,
        };
        let request = options.request().unwrap();
        assert_eq!(request.preset, RvcTrainingPreset::Balanced);
        assert!(request.custom.is_none());
    }

    #[test]
    fn overrides_require_custom_preset() {
        let options = RvcTrainingOptions {
            preset: RvcTrainingPresetArg::Quick,
            epochs: Some(40),
            batch_size: None,
            save_every_epochs: None,
            device: RvcTrainingDeviceArg::Auto,
            precision: RvcTrainingPrecisionArg::Auto,
            cache_dataset_on_gpu: false,
        };
        assert!(options.request().is_err());
    }

    #[test]
    fn custom_config_cannot_escape_verified_envelope() {
        let options = RvcTrainingOptions {
            preset: RvcTrainingPresetArg::Custom,
            epochs: Some(24),
            batch_size: Some(2),
            save_every_epochs: Some(6),
            device: RvcTrainingDeviceArg::Cpu,
            precision: RvcTrainingPrecisionArg::Fp32,
            cache_dataset_on_gpu: false,
        };
        let config = options.config().unwrap();
        assert_eq!(config.sample_rate_hz, 40_000);
        assert_eq!(config.model_version, "v2");
        assert_eq!(config.device, RvcTrainingDevice::Cpu);
    }
}
