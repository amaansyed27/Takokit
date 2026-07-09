use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    TextToSpeech,
    SpeechToText,
    VoiceCloning,
    LiveTranscription,
    LiveAudio,
}

pub type ModelCapability = CapabilityKind;

impl CapabilityKind {
    pub const ALL: [CapabilityKind; 5] = [
        CapabilityKind::TextToSpeech,
        CapabilityKind::SpeechToText,
        CapabilityKind::VoiceCloning,
        CapabilityKind::LiveTranscription,
        CapabilityKind::LiveAudio,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CapabilityKind::TextToSpeech => "TTS",
            CapabilityKind::SpeechToText => "STT",
            CapabilityKind::VoiceCloning => "Voice Cloning",
            CapabilityKind::LiveTranscription => "Live Transcription API",
            CapabilityKind::LiveAudio => "Live Audio API",
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            CapabilityKind::TextToSpeech => "Text input to speech or audio output.",
            CapabilityKind::SpeechToText => "Audio file or input to text transcript.",
            CapabilityKind::VoiceCloning => "Voice samples to a reusable local voice profile.",
            CapabilityKind::LiveTranscription => {
                "Local STT models exposed through an API for streaming or submitted audio."
            }
            CapabilityKind::LiveAudio => {
                "Compatible local voice models exposed through an API for speech output."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityInfo {
    pub id: CapabilityKind,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelRuntime {
    Python,
    Onnx,
    WhisperCpp,
    NativeRust,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub family: String,
    pub version: String,
    pub summary: String,
    pub license: String,
    pub license_warning: Option<String>,
    pub runtime: ModelRuntime,
    pub backend: String,
    pub runner: String,
    pub hardware_notes: String,
    pub artifact_count: usize,
    pub capabilities: Vec<ModelCapability>,
    pub installed: bool,
    pub runner_installed: bool,
    pub runner_runtime_state: String,
    pub lifecycle_state: String,
    pub executable: bool,
    pub missing: Vec<String>,
    pub next_command: String,
    pub execution_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub source: String,
    pub model_id: Option<String>,
    pub consent_required: bool,
}
