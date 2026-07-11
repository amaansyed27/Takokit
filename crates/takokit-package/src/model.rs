//! Model manifests, artifacts, capabilities, and model-info conversion.

use crate::{
    planning::{license_warning, model_execution_status, model_task_label},
    *,
};
use serde::{Deserialize, Serialize};
use takokit_core::{CapabilityKind, ModelCapability, ModelInfo, ModelRuntime};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelManifest {
    pub id: String,
    pub name: String,
    pub family: String,
    pub version: String,
    pub kind: ModelKind,
    pub backend: ModelBackend,
    pub runner: String,
    #[serde(default)]
    pub required_adapter: Option<String>,
    pub license: String,
    pub description: String,
    pub capabilities: CapabilityManifest,
    pub hardware: HardwareManifest,
    pub artifacts: ArtifactManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelKind {
    Tts,
    Stt,
    VoiceClone,
    VoiceCloning,
    VoiceTrain,
    VoiceConvert,
    OmniAudio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelBackend {
    Native,
    Onnx,
    Whispercpp,
    PythonManaged,
    TransformersAudio,
    Nemo,
    External,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityManifest {
    #[serde(default, alias = "speak")]
    pub tts: bool,
    #[serde(default, alias = "transcribe")]
    pub stt: bool,
    #[serde(default, alias = "clone")]
    pub voice_cloning: bool,
    #[serde(default)]
    pub live_transcription: bool,
    #[serde(default)]
    pub live_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardwareManifest {
    pub cpu: bool,
    pub gpu: bool,
    pub min_ram: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactManifest {
    #[serde(default)]
    pub metadata_only: bool,
    #[serde(default)]
    pub weights: Vec<ArtifactEntry>,
    #[serde(default)]
    pub configs: Vec<ArtifactEntry>,
    #[serde(default)]
    pub voices: Vec<ArtifactEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub name: String,
    pub sha256: String,
    pub bytes: Option<u64>,
    pub url: Option<String>,
    #[serde(default)]
    pub role: ArtifactRole,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    Model,
    Config,
    Voice,
    Other,
}

impl Default for ArtifactRole {
    fn default() -> Self {
        Self::Model
    }
}

impl ModelManifest {
    pub fn supports(&self, capability: CapabilityKind) -> bool {
        self.capabilities.supports(capability)
    }

    pub fn to_model_info(&self, installed: bool, runner_installed: bool) -> ModelInfo {
        let fallback = ModelPlan {
            model_id: self.id.clone(),
            model_name: self.name.clone(),
            family: self.family.clone(),
            task: model_task_label(self),
            required_runner: self.runner.clone(),
            lifecycle_state: if installed {
                ModelLifecycleState::ArtifactsReady
            } else {
                ModelLifecycleState::MetadataOnly
            },
            artifact_state: if installed {
                ModelLifecycleState::ArtifactsReady
            } else {
                ModelLifecycleState::MetadataOnly
            },
            runner_contract_state: if runner_installed {
                RunnerLifecycleState::ContractInstalled
            } else {
                RunnerLifecycleState::RuntimeMissing
            },
            runner_runtime_state: if runner_installed {
                RunnerLifecycleState::ContractInstalled
            } else {
                RunnerLifecycleState::RuntimeMissing
            },
            executable: false,
            missing: if installed {
                vec![format!("runner contract: {}", self.runner)]
            } else {
                vec!["verified artifacts".to_string()]
            },
            next_command: if installed {
                format!("takokit runner pull {}", self.runner)
            } else {
                format!("takokit pull {}", self.id)
            },
        };
        self.to_model_info_from_plan(&fallback, installed, runner_installed)
    }

    pub fn to_model_info_from_plan(
        &self,
        plan: &ModelPlan,
        installed: bool,
        runner_installed: bool,
    ) -> ModelInfo {
        ModelInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            family: self.family.clone(),
            version: self.version.clone(),
            summary: self.description.clone(),
            license: self.license.clone(),
            license_warning: license_warning(&self.license),
            runtime: self.backend.to_model_runtime(),
            backend: self.backend.as_str().to_string(),
            runner: self.runner.clone(),
            hardware_notes: self.hardware.notes(),
            artifact_count: self.artifacts.all().count(),
            capabilities: self.capabilities.to_model_capabilities(),
            installed,
            runner_installed,
            runner_runtime_state: plan.runner_runtime_state.to_string(),
            lifecycle_state: plan.lifecycle_state.to_string(),
            executable: plan.executable,
            missing: plan.missing.clone(),
            next_command: plan.next_command.clone(),
            execution_status: model_execution_status(plan),
        }
    }
}

impl CapabilityManifest {
    pub fn supports(&self, capability: CapabilityKind) -> bool {
        match capability {
            CapabilityKind::TextToSpeech => self.tts,
            CapabilityKind::SpeechToText => self.stt,
            CapabilityKind::VoiceCloning => self.voice_cloning,
            CapabilityKind::LiveTranscription => self.live_transcription,
            CapabilityKind::LiveAudio => self.live_audio,
        }
    }

    pub fn to_model_capabilities(&self) -> Vec<ModelCapability> {
        let mut capabilities = Vec::new();
        if self.tts {
            capabilities.push(CapabilityKind::TextToSpeech);
        }
        if self.stt {
            capabilities.push(CapabilityKind::SpeechToText);
        }
        if self.voice_cloning {
            capabilities.push(CapabilityKind::VoiceCloning);
        }
        if self.live_transcription {
            capabilities.push(CapabilityKind::LiveTranscription);
        }
        if self.live_audio {
            capabilities.push(CapabilityKind::LiveAudio);
        }
        capabilities
    }
}

impl ArtifactManifest {
    pub fn all(&self) -> impl Iterator<Item = &ArtifactEntry> {
        self.weights
            .iter()
            .chain(self.configs.iter())
            .chain(self.voices.iter())
    }
}

impl HardwareManifest {
    fn notes(&self) -> String {
        let acceleration = match (self.cpu, self.gpu) {
            (true, true) => "CPU or GPU",
            (true, false) => "CPU",
            (false, true) => "GPU",
            (false, false) => "unspecified hardware",
        };
        match self.min_ram.as_deref() {
            Some(min_ram) => format!("{acceleration}, minimum RAM {min_ram}"),
            None => acceleration.to_string(),
        }
    }
}

impl ModelBackend {
    fn as_str(&self) -> &'static str {
        match self {
            ModelBackend::Native => "native",
            ModelBackend::Onnx => "onnx",
            ModelBackend::Whispercpp => "whispercpp",
            ModelBackend::PythonManaged => "python-managed",
            ModelBackend::TransformersAudio => "transformers-audio",
            ModelBackend::Nemo => "nemo",
            ModelBackend::External => "external",
        }
    }

    fn to_model_runtime(&self) -> ModelRuntime {
        match self {
            ModelBackend::Native => ModelRuntime::NativeRust,
            ModelBackend::Onnx => ModelRuntime::Onnx,
            ModelBackend::Whispercpp => ModelRuntime::WhisperCpp,
            ModelBackend::PythonManaged => ModelRuntime::Python,
            ModelBackend::TransformersAudio => ModelRuntime::External,
            ModelBackend::Nemo => ModelRuntime::External,
            ModelBackend::External => ModelRuntime::External,
        }
    }
}
