use takokit_core::{ModelCapability, ModelInfo, ModelRuntime, VoiceInfo};

#[derive(Debug, Clone)]
pub struct ModelRegistry {
    models: Vec<ModelInfo>,
    voices: Vec<VoiceInfo>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self {
            models: built_in_models(),
            voices: vec![VoiceInfo {
                id: "default".to_string(),
                name: "Default mock voice".to_string(),
                source: "takokit-mock".to_string(),
                model_id: Some("mock-tts".to_string()),
                consent_required: false,
            }],
        }
    }
}

impl ModelRegistry {
    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    pub fn voices(&self) -> &[VoiceInfo] {
        &self.voices
    }
}

pub fn built_in_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "mock-tts".to_string(),
            name: "Mock TTS".to_string(),
            version: "0.1.0".to_string(),
            summary: "Deterministic test WAV generator for API and CLI scaffolding.".to_string(),
            license: "internal-test".to_string(),
            runtime: ModelRuntime::NativeRust,
            backend: "native_rust".to_string(),
            runner: "takokit-mock".to_string(),
            hardware_notes: "CPU, no model weights".to_string(),
            capabilities: vec![ModelCapability::TextToSpeech],
            installed: true,
            runner_installed: true,
            execution_status: "ready".to_string(),
        },
        model(
            "kokoro",
            "Kokoro",
            "Fast local TTS",
            "unknown",
            ModelRuntime::Python,
            vec![ModelCapability::TextToSpeech],
        ),
        model(
            "whisper",
            "Whisper",
            "Transcription",
            "unknown",
            ModelRuntime::WhisperCpp,
            vec![ModelCapability::SpeechToText],
        ),
        model(
            "chatterbox",
            "Chatterbox",
            "Voice cloning",
            "unknown",
            ModelRuntime::Python,
            vec![
                ModelCapability::TextToSpeech,
                ModelCapability::VoiceCloning,
                ModelCapability::LiveAudio,
            ],
        ),
        model(
            "gpt-sovits",
            "GPT-SoVITS",
            "Few-shot voice training",
            "unknown",
            ModelRuntime::Python,
            vec![
                ModelCapability::TextToSpeech,
                ModelCapability::VoiceCloning,
                ModelCapability::LiveAudio,
            ],
        ),
        model(
            "qwen3-tts",
            "Qwen3-TTS",
            "Voice design and streaming",
            "unknown",
            ModelRuntime::Python,
            vec![ModelCapability::TextToSpeech, ModelCapability::LiveAudio],
        ),
        model(
            "rvc",
            "RVC",
            "Voice conversion",
            "unknown",
            ModelRuntime::Python,
            vec![ModelCapability::VoiceCloning],
        ),
        model(
            "piper",
            "Piper",
            "Lightweight offline voices",
            "unknown",
            ModelRuntime::Onnx,
            vec![ModelCapability::TextToSpeech],
        ),
    ]
}

fn model(
    id: &str,
    name: &str,
    summary: &str,
    license: &str,
    runtime: ModelRuntime,
    capabilities: Vec<ModelCapability>,
) -> ModelInfo {
    ModelInfo {
        id: id.to_string(),
        name: name.to_string(),
        version: "0.1.0".to_string(),
        summary: summary.to_string(),
        license: license.to_string(),
        runtime,
        backend: "registry".to_string(),
        runner: "unresolved".to_string(),
        hardware_notes: "runner contract pending".to_string(),
        capabilities,
        installed: false,
        runner_installed: false,
        execution_status: "runner not installed or not implemented".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_required_initial_models() {
        let registry = ModelRegistry::default();
        let ids: Vec<_> = registry
            .models()
            .iter()
            .map(|model| model.id.as_str())
            .collect();

        for required in [
            "kokoro",
            "whisper",
            "chatterbox",
            "gpt-sovits",
            "qwen3-tts",
            "rvc",
            "piper",
        ] {
            assert!(ids.contains(&required), "missing {required}");
        }
    }
}
