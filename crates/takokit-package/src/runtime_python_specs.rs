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

#[derive(Debug, Clone, Copy)]
pub(crate) struct AdapterSpec {
    pub id: &'static str,
    pub model_family: &'static str,
    pub python: &'static str,
    pub packages: &'static [&'static str],
    pub script: Option<&'static str>,
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

pub(crate) const ADAPTER_SPECS: &[AdapterSpec] = &[
    AdapterSpec {
        id: "qwen3_tts",
        model_family: "qwen3-tts",
        python: "3.11",
        packages: &["qwen-tts==0.1.1", "soundfile"],
        script: Some(QWEN3_TTS_ADAPTER),
        note: "Qwen3-TTS speech generation using the official qwen-tts package.",
    },
    AdapterSpec {
        id: "chatterbox",
        model_family: "chatterbox",
        python: "3.11",
        packages: &["chatterbox-tts", "torchaudio"],
        script: Some(CHATTERBOX_ADAPTER),
        note: "Chatterbox TTS and reference-audio voice cloning through the official Python API.",
    },
    AdapterSpec {
        id: "f5_tts",
        model_family: "f5-tts",
        python: "3.11",
        packages: &["f5-tts"],
        script: Some(F5_TTS_ADAPTER),
        note: "F5-TTS inference and reference-audio voice transfer through the official API.",
    },
    AdapterSpec {
        id: "dia",
        model_family: "dia",
        python: "3.11",
        packages: &[
            "git+https://github.com/huggingface/transformers.git",
            "torch",
            "accelerate",
            "soundfile",
        ],
        script: Some(DIA_ADAPTER),
        note: "Dia text-to-dialogue generation through the official Transformers integration.",
    },
    AdapterSpec {
        id: "sensevoice",
        model_family: "sensevoice",
        python: "3.11",
        packages: &["torch", "torchaudio", "funasr", "modelscope"],
        script: Some(SENSEVOICE_ADAPTER),
        note: "SenseVoice multilingual transcription through the official FunASR API.",
    },
    AdapterSpec {
        id: "voxtral",
        model_family: "voxtral",
        python: "3.11",
        packages: &[
            "git+https://github.com/huggingface/transformers.git",
            "torch",
            "accelerate",
            "soundfile",
            "mistral-common[audio]",
        ],
        script: Some(VOXTRAL_ADAPTER),
        note: "Voxtral multilingual transcription through the official Transformers API.",
    },
    AdapterSpec {
        id: "nemo_asr",
        model_family: "canary",
        python: "3.12",
        packages: NEMO_PACKAGES,
        script: Some(NEMO_ASR_ADAPTER),
        note: "NVIDIA Canary transcription through the official NeMo ASR API.",
    },
    AdapterSpec {
        id: "nemo_asr",
        model_family: "parakeet",
        python: "3.12",
        packages: NEMO_PACKAGES,
        script: Some(NEMO_ASR_ADAPTER),
        note: "NVIDIA Parakeet transcription through the official NeMo ASR API.",
    },
    AdapterSpec {
        id: "hf_audio",
        model_family: "bark-small",
        python: "3.11",
        packages: HF_AUDIO_PACKAGES,
        script: Some(HF_AUDIO_ADAPTER),
        note: "Bark Small generation through the official Transformers BarkModel API.",
    },
    AdapterSpec {
        id: "hf_audio",
        model_family: "mms-tts-eng",
        python: "3.11",
        packages: HF_AUDIO_PACKAGES,
        script: Some(HF_AUDIO_ADAPTER),
        note: "MMS-TTS generation through the official Transformers VitsModel API.",
    },
    AdapterSpec {
        id: "hf_audio",
        model_family: "distil-whisper-large-v3",
        python: "3.11",
        packages: HF_AUDIO_PACKAGES,
        script: Some(HF_AUDIO_ADAPTER),
        note: "Distil-Whisper transcription through the official Transformers ASR pipeline.",
    },
    AdapterSpec {
        id: "hf_audio",
        model_family: "wav2vec2-base-960h",
        python: "3.11",
        packages: HF_AUDIO_PACKAGES,
        script: Some(HF_AUDIO_ADAPTER),
        note: "Wav2Vec2 transcription through the official Transformers ASR pipeline.",
    },
    AdapterSpec {
        id: "coqui_tts",
        model_family: "xtts-v2",
        python: "3.11",
        packages: COQUI_PACKAGES,
        script: Some(COQUI_TTS_ADAPTER),
        note: "XTTS v2 zero-shot cloning through the Coqui TTS API.",
    },
    AdapterSpec {
        id: "coqui_tts",
        model_family: "yourtts",
        python: "3.11",
        packages: COQUI_PACKAGES,
        script: Some(COQUI_TTS_ADAPTER),
        note: "YourTTS zero-shot cloning through the Coqui TTS API.",
    },
    AdapterSpec {
        id: "kyutai_tts",
        model_family: "kyutai-tts-1.6b",
        python: "3.12",
        packages: &["moshi==0.2.11", "torch", "sphn", "numpy"],
        script: Some(KYUTAI_TTS_ADAPTER),
        note: "Kyutai DSM streaming-capable TTS using the official Moshi PyTorch API.",
    },
    AdapterSpec {
        id: "cosyvoice2",
        model_family: "cosyvoice2",
        python: "3.11",
        packages: &[],
        script: None,
        note: "Reserved for the CosyVoice2 official runtime integration.",
    },
    AdapterSpec {
        id: "fish_speech",
        model_family: "fish-speech",
        python: "3.11",
        packages: &[],
        script: None,
        note: "Reserved for the Fish Speech official runtime integration.",
    },
    AdapterSpec {
        id: "openvoice",
        model_family: "openvoice",
        python: "3.11",
        packages: &[],
        script: None,
        note: "Reserved for the OpenVoice tone-color conversion integration.",
    },
    AdapterSpec {
        id: "gpt_sovits",
        model_family: "gpt-sovits",
        python: "3.11",
        packages: &[],
        script: None,
        note: "Reserved for the GPT-SoVITS inference and training integration.",
    },
    AdapterSpec {
        id: "rvc",
        model_family: "rvc",
        python: "3.11",
        packages: &[],
        script: None,
        note: "Reserved for the RVC voice-conversion integration.",
    },
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
