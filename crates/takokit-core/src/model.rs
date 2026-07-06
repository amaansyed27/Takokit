use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    TextToSpeech,
    SpeechToText,
    VoiceCloning,
    VoiceTraining,
    VoiceConversion,
    Streaming,
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
    pub summary: String,
    pub license: String,
    pub runtime: ModelRuntime,
    pub capabilities: Vec<ModelCapability>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub source: String,
    pub model_id: Option<String>,
    pub consent_required: bool,
}
