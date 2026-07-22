//! Managed-Python adapter definitions used by model planning and installation.

const QWEN3_TTS_ADAPTER: &str = include_str!("../../../runners/python/qwen3_tts_adapter.py");
const CHATTERBOX_ADAPTER: &str = include_str!("../../../runners/python/chatterbox_adapter.py");
const F5_TTS_ADAPTER: &str = include_str!("../../../runners/python/f5_tts_adapter.py");
const DIA_ADAPTER: &str = include_str!("../../../runners/python/dia_adapter.py");
const SENSEVOICE_ADAPTER: &str = include_str!("../../../runners/python/sensevoice_adapter.py");
const VOXTRAL_ADAPTER: &str = include_str!("../../../runners/python/voxtral_adapter.py");
const NEMO_ASR_ADAPTER: &str = include_str!("../../../runners/python/nemo_asr_adapter.py");
const HF_AUDIO_ADAPTER: &str = include_str!("../../../runners/python/hf_audio_adapter.py");
const COQUI_TTS_ADAPTER: &str = include_str!("../../../runners/python/coqui_tts_adapter.py");
const KYUTAI_TTS_ADAPTER: &str = include_str!("../../../runners/python/kyutai_tts_adapter.py");
const PIPER_ADAPTER: &str = include_str!("../../../runners/python/piper_adapter.py");
const COSYVOICE2_ADAPTER: &str = include_str!("../../../runners/python/cosyvoice2_adapter.py");
const FISH_SPEECH_ADAPTER: &str = include_str!("../../../runners/python/fish_speech_adapter.py");
const OPENVOICE_ADAPTER: &str = include_str!("../../../runners/python/openvoice_adapter.py");
const GPT_SOVITS_ADAPTER: &str = include_str!("../../../runners/python/gpt_sovits_adapter.py");
const RVC_ADAPTER: &str = include_str!("../../../runners/python/rvc_adapter.py");
const QWEN_OMNI_ADAPTER: &str = include_str!("../../../runners/python/qwen_omni_adapter.py");

#[derive(Debug, Clone, Copy)]
pub(crate) struct AdapterSourceSpec {
    pub repository: &'static str,
    pub revision: &'static str,
    pub recursive: bool,
    pub requirement_files: &'static [&'static str],
    pub editable: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AdapterSpec {
    pub id: &'static str,
    pub model_family: &'static str,
    pub python: &'static str,
    pub packages: &'static [&'static str],
    pub script: Option<&'static str>,
    pub source: Option<AdapterSourceSpec>,
    pub note: &'static str,
}

#[path = "runtime_python_specs/catalog.rs"]
mod catalog;
pub(crate) use catalog::ADAPTER_SPECS;

pub(crate) fn adapter_spec(id: &str) -> Option<&'static AdapterSpec> {
    ADAPTER_SPECS.iter().find(|spec| spec.id == id)
}

pub fn adapter_for_model(model_id: &str) -> Option<&'static str> {
    ADAPTER_SPECS
        .iter()
        .find(|spec| spec.model_family == model_id)
        .map(|spec| spec.id)
}

pub(crate) fn model_prefetch_required(model_id: &str) -> bool {
    matches!(
        model_id,
        "bark-small"
            | "canary"
            | "parakeet"
            | "dia"
            | "distil-whisper-large-v3"
            | "f5-tts"
            | "kyutai-tts-1.6b"
            | "mms-tts-eng"
            | "sensevoice"
            | "voxtral"
            | "wav2vec2-base-960h"
            | "xtts-v2"
            | "yourtts"
    )
}
