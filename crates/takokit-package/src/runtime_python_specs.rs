//! Managed-Python adapter definitions used by model planning and installation.

const QWEN3_TTS_ADAPTER: &str = include_str!("../../../runners/python/qwen3_tts_adapter.py");
const PIPER_ADAPTER: &str = include_str!("../../../runners/python/piper_adapter.py");
const CHATTERBOX_ADAPTER: &str = include_str!("../../../runners/python/chatterbox_adapter.py");
const F5_TTS_ADAPTER: &str = include_str!("../../../runners/python/f5_tts_adapter.py");
const DIA_ADAPTER: &str = include_str!("../../../runners/python/dia_adapter.py");
const SENSEVOICE_ADAPTER: &str = include_str!("../../../runners/python/sensevoice_adapter.py");
const VOXTRAL_ADAPTER: &str = include_str!("../../../runners/python/voxtral_adapter.py");
const NEMO_ASR_ADAPTER: &str = include_str!("../../../runners/python/nemo_asr_adapter.py");
const HF_AUDIO_ADAPTER: &str = include_str!("../../../runners/python/hf_audio_adapter.py");
const COQUI_TTS_ADAPTER: &str = include_str!("../../../runners/python/coqui_tts_adapter.py");
const KYUTAI_TTS_ADAPTER: &str = include_str!("../../../runners/python/kyutai_tts_adapter.py");
const COSYVOICE2_ADAPTER: &str = include_str!("../../../runners/python/cosyvoice2_adapter.py");
const FISH_SPEECH_ADAPTER: &str = include_str!("../../../runners/python/fish_speech_adapter.py");
const OPENVOICE_ADAPTER: &str = include_str!("../../../runners/python/openvoice_adapter.py");
const GPT_SOVITS_ADAPTER: &str = include_str!("../../../runners/python/gpt_sovits_adapter.py");
const RVC_ADAPTER: &str = include_str!("../../../runners/python/rvc_adapter.py");
const QWEN25_OMNI_ADAPTER: &str = include_str!("../../../runners/python/qwen25_omni_adapter.py");
const QWEN3_OMNI_ADAPTER: &str = include_str!("../../../runners/python/qwen3_omni_adapter.py");

#[derive(Debug, Clone, Copy)]
pub(crate) struct AdapterSpec {
    pub id: &'static str,
    pub model_family: &'static str,
    pub python: &'static str,
    pub packages: &'static [&'static str],
    pub script: Option<&'static str>,
    pub source_repository: Option<&'static str>,
    pub source_revision: Option<&'static str>,
    pub source_recursive: bool,
    pub requirements: &'static [&'static str],
    pub editable_source: bool,
    pub note: &'static str,
}

const HF_AUDIO_PACKAGES: &[&str] = &[
    "torch",
    "torchaudio",
    "transformers",
    "accelerate",
    "soundfile",
    "scipy",
];
const COQUI_PACKAGES: &[&str] = &["coqui-tts", "torch", "torchaudio"];
const NEMO_PACKAGES: &[&str] = &["torch", "nemo-toolkit[asr]"];
const QWEN_TTS_PACKAGES: &[&str] = &["qwen-tts==0.1.1", "soundfile"];

macro_rules! packaged_adapter {
    ($id:literal, $family:literal, $python:literal, $packages:expr, $script:expr, $note:literal) => {
        AdapterSpec {
            id: $id,
            model_family: $family,
            python: $python,
            packages: $packages,
            script: Some($script),
            source_repository: None,
            source_revision: None,
            source_recursive: false,
            requirements: &[],
            editable_source: false,
            note: $note,
        }
    };
}

pub(crate) const ADAPTER_SPECS: &[AdapterSpec] = &[
    packaged_adapter!(
        "qwen3_tts",
        "qwen3-tts",
        "3.11",
        QWEN_TTS_PACKAGES,
        QWEN3_TTS_ADAPTER,
        "Qwen3-TTS 0.6B CustomVoice generation."
    ),
    packaged_adapter!(
        "qwen3_tts",
        "qwen3-tts-0.6b-base",
        "3.11",
        QWEN_TTS_PACKAGES,
        QWEN3_TTS_ADAPTER,
        "Qwen3-TTS 0.6B zero-shot voice cloning."
    ),
    packaged_adapter!(
        "qwen3_tts",
        "qwen3-tts-1.7b-custom",
        "3.11",
        QWEN_TTS_PACKAGES,
        QWEN3_TTS_ADAPTER,
        "Qwen3-TTS 1.7B CustomVoice generation."
    ),
    packaged_adapter!(
        "qwen3_tts",
        "qwen3-tts-1.7b-base",
        "3.11",
        QWEN_TTS_PACKAGES,
        QWEN3_TTS_ADAPTER,
        "Qwen3-TTS 1.7B zero-shot voice cloning."
    ),
    packaged_adapter!(
        "qwen3_tts",
        "qwen3-tts-1.7b-voice-design",
        "3.11",
        QWEN_TTS_PACKAGES,
        QWEN3_TTS_ADAPTER,
        "Qwen3-TTS 1.7B natural-language VoiceDesign generation."
    ),
    packaged_adapter!(
        "piper_tts",
        "piper-lessac",
        "3.11",
        &["piper-tts"],
        PIPER_ADAPTER,
        "Piper CPU TTS through the official managed Python runtime."
    ),
    packaged_adapter!(
        "chatterbox",
        "chatterbox",
        "3.11",
        &["chatterbox-tts", "torchaudio"],
        CHATTERBOX_ADAPTER,
        "Chatterbox TTS and reference-audio voice cloning."
    ),
    packaged_adapter!(
        "f5_tts",
        "f5-tts",
        "3.11",
        &["f5-tts"],
        F5_TTS_ADAPTER,
        "F5-TTS inference and reference-audio voice transfer."
    ),
    packaged_adapter!(
        "dia",
        "dia",
        "3.11",
        &[
            "git+https://github.com/huggingface/transformers.git",
            "torch",
            "accelerate",
            "soundfile",
        ],
        DIA_ADAPTER,
        "Dia text-to-dialogue generation through Transformers."
    ),
    packaged_adapter!(
        "sensevoice",
        "sensevoice",
        "3.11",
        &["torch", "torchaudio", "funasr", "modelscope"],
        SENSEVOICE_ADAPTER,
        "SenseVoice multilingual transcription through FunASR."
    ),
    packaged_adapter!(
        "voxtral",
        "voxtral",
        "3.11",
        &[
            "transformers>=4.57",
            "torch",
            "accelerate",
            "soundfile",
            "mistral-common[audio]",
        ],
        VOXTRAL_ADAPTER,
        "Voxtral multilingual transcription through Transformers."
    ),
    packaged_adapter!(
        "nemo_asr",
        "canary",
        "3.12",
        NEMO_PACKAGES,
        NEMO_ASR_ADAPTER,
        "NVIDIA Canary transcription through NeMo."
    ),
    packaged_adapter!(
        "nemo_asr",
        "parakeet",
        "3.12",
        NEMO_PACKAGES,
        NEMO_ASR_ADAPTER,
        "NVIDIA Parakeet transcription through NeMo."
    ),
    packaged_adapter!(
        "hf_audio",
        "bark-small",
        "3.11",
        HF_AUDIO_PACKAGES,
        HF_AUDIO_ADAPTER,
        "Bark Small generation through Transformers."
    ),
    packaged_adapter!(
        "hf_audio",
        "mms-tts-eng",
        "3.11",
        HF_AUDIO_PACKAGES,
        HF_AUDIO_ADAPTER,
        "MMS-TTS generation through Transformers."
    ),
    packaged_adapter!(
        "hf_audio",
        "distil-whisper-large-v3",
        "3.11",
        HF_AUDIO_PACKAGES,
        HF_AUDIO_ADAPTER,
        "Distil-Whisper transcription through Transformers."
    ),
    packaged_adapter!(
        "hf_audio",
        "wav2vec2-base-960h",
        "3.11",
        HF_AUDIO_PACKAGES,
        HF_AUDIO_ADAPTER,
        "Wav2Vec2 transcription through Transformers."
    ),
    packaged_adapter!(
        "coqui_tts",
        "xtts-v2",
        "3.11",
        COQUI_PACKAGES,
        COQUI_TTS_ADAPTER,
        "XTTS v2 zero-shot cloning through Coqui TTS."
    ),
    packaged_adapter!(
        "coqui_tts",
        "yourtts",
        "3.11",
        COQUI_PACKAGES,
        COQUI_TTS_ADAPTER,
        "YourTTS zero-shot cloning through Coqui TTS."
    ),
    packaged_adapter!(
        "kyutai_tts",
        "kyutai-tts-1.6b",
        "3.12",
        &["moshi==0.2.11", "torch", "sphn", "numpy"],
        KYUTAI_TTS_ADAPTER,
        "Kyutai DSM TTS using the official Moshi API."
    ),
    AdapterSpec {
        id: "cosyvoice2",
        model_family: "cosyvoice2",
        python: "3.10",
        packages: &[],
        script: Some(COSYVOICE2_ADAPTER),
        source_repository: Some("https://github.com/FunAudioLLM/CosyVoice.git"),
        source_revision: Some("074ca6dc9e80a2f424f1f74b48bdd7d3fea531cc"),
        source_recursive: true,
        requirements: &["requirements.txt"],
        editable_source: false,
        note: "CosyVoice2 zero-shot TTS and voice conversion through the pinned official source.",
    },
    AdapterSpec {
        id: "fish_speech",
        model_family: "fish-speech",
        python: "3.10",
        packages: &["soundfile"],
        script: Some(FISH_SPEECH_ADAPTER),
        source_repository: Some("https://github.com/fishaudio/fish-speech.git"),
        source_revision: Some("e5e292632cb11e7a27b2b7487f58f612bc101e13"),
        source_recursive: false,
        requirements: &[],
        editable_source: true,
        note: "Fish Speech inference through the pinned official source tree.",
    },
    packaged_adapter!(
        "openvoice",
        "openvoice",
        "3.11",
        &[
            "git+https://github.com/myshell-ai/OpenVoice.git@74a1d147b17a8c3092dd5430504bd83ef6c7eb23",
            "git+https://github.com/myshell-ai/MeloTTS.git@209145371cff8fc3bd60d7be902ea69cbdb7965a",
            "torch",
            "torchaudio",
            "soundfile",
        ],
        OPENVOICE_ADAPTER,
        "OpenVoice V2 local tone-colour cloning."
    ),
    AdapterSpec {
        id: "gpt_sovits",
        model_family: "gpt-sovits",
        python: "3.10",
        packages: &[],
        script: Some(GPT_SOVITS_ADAPTER),
        source_repository: Some("https://github.com/RVC-Boss/GPT-SoVITS.git"),
        source_revision: Some("be6a4f1e9d8a22d41b7d42c22df9d7ef36f225d2"),
        source_recursive: false,
        requirements: &["extra-req.txt", "requirements.txt"],
        editable_source: false,
        note: "GPT-SoVITS inference and training through the pinned official source.",
    },
    packaged_adapter!(
        "rvc",
        "rvc",
        "3.10",
        &["git+https://github.com/RVC-Project/Retrieval-based-Voice-Conversion.git@7b284a634667c34103eaaeed972b48ccdb4b893e"],
        RVC_ADAPTER,
        "RVC local voice conversion and training."
    ),
    packaged_adapter!(
        "qwen25_omni",
        "qwen2-5-omni",
        "3.11",
        &[
            "transformers==4.52.3",
            "qwen-omni-utils",
            "accelerate",
            "torch",
            "torchaudio",
            "soundfile",
        ],
        QWEN25_OMNI_ADAPTER,
        "Qwen2.5-Omni local audio transcription."
    ),
    packaged_adapter!(
        "qwen3_omni",
        "qwen3-omni",
        "3.11",
        &[
            "transformers>=5.2.0",
            "qwen-omni-utils",
            "accelerate",
            "torch",
            "torchaudio",
            "soundfile",
        ],
        QWEN3_OMNI_ADAPTER,
        "Qwen3-Omni local audio transcription."
    ),
];

pub(crate) fn adapter_spec(id: &str) -> Option<&'static AdapterSpec> {
    ADAPTER_SPECS.iter().find(|spec| spec.id == id)
}

pub fn adapter_for_model(model_id: &str) -> Option<&'static str> {
    ADAPTER_SPECS
        .iter()
        .find(|spec| spec.model_family == model_id)
        .map(|spec| spec.id)
}
