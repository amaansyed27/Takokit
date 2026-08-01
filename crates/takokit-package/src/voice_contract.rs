//! Model-facing voice-input contracts shared by CLI, API, and runners.

use crate::{runtime_model_id, ModelManifest};
use serde::{Deserialize, Serialize};
use takokit_core::{SpeechRequest, TakokitError, TakokitResult};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VoiceMode {
    ModelDefault,
    PresetSpeaker,
    ReferenceAudio,
    VoiceDesign,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InputRequirement {
    Unsupported,
    Optional,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceContract {
    pub model_id: String,
    pub runtime_model_id: String,
    pub modes: Vec<VoiceMode>,
    pub default_voice: Option<String>,
    pub preset_voices: Vec<String>,
    pub voice_input: InputRequirement,
    pub reference_text: InputRequirement,
    pub instruction: InputRequirement,
    pub notes: Vec<String>,
}

pub fn voice_contract_for_model(model: &ModelManifest) -> VoiceContract {
    let runtime = runtime_model_id(model);
    let mut contract = VoiceContract {
        model_id: model.id.clone(),
        runtime_model_id: runtime.to_string(),
        modes: vec![VoiceMode::ModelDefault],
        default_voice: None,
        preset_voices: Vec::new(),
        voice_input: InputRequirement::Optional,
        reference_text: InputRequirement::Optional,
        instruction: InputRequirement::Optional,
        notes: Vec::new(),
    };
    match runtime {
        "qwen3-tts" | "qwen3-tts-1.7b-custom" => {
            contract.modes = vec![VoiceMode::PresetSpeaker];
            contract.default_voice = Some("Ryan".to_string());
            contract.preset_voices = [
                "Vivian",
                "Serena",
                "Uncle_Fu",
                "Dylan",
                "Eric",
                "Ryan",
                "Aiden",
                "Ono_Anna",
                "Sohee",
            ]
            .into_iter()
            .map(str::to_string)
            .collect();
            contract.reference_text = InputRequirement::Unsupported;
            contract.instruction = InputRequirement::Optional;
            contract.notes.push(
                "Use --voice with an official preset speaker and --instruction for delivery control."
                    .to_string(),
            );
        }
        "qwen3-tts-0.6b-base" | "qwen3-tts-1.7b-base" => {
            contract.modes = vec![VoiceMode::ReferenceAudio];
            contract.voice_input = InputRequirement::Required;
            contract.reference_text = InputRequirement::Optional;
            contract.instruction = InputRequirement::Unsupported;
            contract.notes.push(
                "Use a consent-backed audio path or saved Takokit voice-profile ID.".to_string(),
            );
        }
        "qwen3-tts-1.7b-voice-design" => {
            contract.modes = vec![VoiceMode::VoiceDesign];
            contract.voice_input = InputRequirement::Unsupported;
            contract.reference_text = InputRequirement::Unsupported;
            contract.instruction = InputRequirement::Required;
        }
        "gpt-sovits" | "cosyvoice2" => {
            contract.modes = vec![VoiceMode::ReferenceAudio];
            contract.voice_input = InputRequirement::Required;
            contract.reference_text = InputRequirement::Required;
            contract.instruction = InputRequirement::Unsupported;
        }
        "f5-tts" => {
            contract.modes = vec![VoiceMode::ModelDefault, VoiceMode::ReferenceAudio];
            contract.voice_input = InputRequirement::Optional;
            contract.reference_text = InputRequirement::Optional;
            contract.instruction = InputRequirement::Unsupported;
        }
        "chatterbox" | "openvoice" | "xtts-v2" | "yourtts" => {
            contract.modes = vec![VoiceMode::ModelDefault, VoiceMode::ReferenceAudio];
            contract.voice_input = InputRequirement::Optional;
            contract.reference_text = InputRequirement::Optional;
            contract.instruction = InputRequirement::Optional;
        }
        _ if model.capabilities.voice_cloning => {
            contract.modes = vec![VoiceMode::ReferenceAudio];
            contract.notes.push(
                "The runner accepts a consent-backed audio path or saved voice-profile ID."
                    .to_string(),
            );
        }
        _ => {}
    }
    contract
}

pub fn validate_speech_request(
    model: &ModelManifest,
    request: &SpeechRequest,
) -> TakokitResult<()> {
    let contract = voice_contract_for_model(model);
    let voice_present = request
        .voice
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "default");
    let reference_text_present = request
        .reference_text
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let instruction_present = request
        .instruction
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());

    validate_requirement(&model.id, "--voice", contract.voice_input, voice_present)?;
    validate_requirement(
        &model.id,
        "--reference-text",
        contract.reference_text,
        reference_text_present,
    )?;
    validate_requirement(
        &model.id,
        "--instruction",
        contract.instruction,
        instruction_present,
    )?;

    if !contract.preset_voices.is_empty() && voice_present {
        let supplied = request.voice.as_deref().unwrap_or_default();
        if !contract
            .preset_voices
            .iter()
            .any(|preset| preset.eq_ignore_ascii_case(supplied))
        {
            return Err(TakokitError::InvalidRequest(format!(
                "{} does not provide preset voice {supplied}; supported voices: {}",
                model.id,
                contract.preset_voices.join(", ")
            )));
        }
    }
    Ok(())
}

fn validate_requirement(
    model: &str,
    option: &str,
    requirement: InputRequirement,
    present: bool,
) -> TakokitResult<()> {
    match (requirement, present) {
        (InputRequirement::Required, false) => Err(TakokitError::InvalidRequest(format!(
            "{model} requires {option}"
        ))),
        (InputRequirement::Unsupported, true) => Err(TakokitError::InvalidRequest(format!(
            "{model} does not support {option}"
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArtifactManifest, CapabilityManifest, HardwareManifest, ModelBackend, ModelKind};

    fn model(id: &str, family: &str, cloning: bool) -> ModelManifest {
        ModelManifest {
            id: id.to_string(),
            name: id.to_string(),
            family: family.to_string(),
            version: "1".to_string(),
            kind: ModelKind::Tts,
            backend: ModelBackend::PythonManaged,
            runner: "takokit-python-managed".to_string(),
            required_adapter: Some("qwen3_tts".to_string()),
            license: "test".to_string(),
            description: "test".to_string(),
            capabilities: CapabilityManifest {
                tts: true,
                voice_cloning: cloning,
                ..CapabilityManifest::default()
            },
            hardware: HardwareManifest {
                cpu: true,
                gpu: true,
                min_ram: None,
                min_vram: None,
            },
            source: None,
            artifacts: ArtifactManifest::default(),
        }
    }

    #[test]
    fn custom_qwen_base_inherits_reference_contract() {
        let model = model("local-qwen", "qwen3-tts-1.7b-base", true);
        let contract = voice_contract_for_model(&model);
        assert_eq!(contract.runtime_model_id, "qwen3-tts-1.7b-base");
        assert_eq!(contract.voice_input, InputRequirement::Required);
    }

    #[test]
    fn voice_design_requires_instruction() {
        let model = model(
            "qwen3-tts-1.7b-voice-design",
            "qwen3-tts-1.7b-voice-design",
            false,
        );
        let request = SpeechRequest {
            model: model.id.clone(),
            input: "hello".to_string(),
            voice: Some("default".to_string()),
            response_format: Some("wav".to_string()),
            language: None,
            instruction: None,
            reference_text: None,
        };
        assert!(validate_speech_request(&model, &request).is_err());
    }
}
