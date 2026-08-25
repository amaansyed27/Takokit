use super::*;

const HF_AUDIO_PACKAGES: &[&str] = &[
    "torch",
    "torchaudio",
    "transformers",
    "accelerate",
    "huggingface_hub>=1.2",
    "soundfile",
    "scipy",
];
const COQUI_PACKAGES: &[&str] = &[
    "coqui-tts==0.27.5",
    "transformers==4.57.6",
    "torch",
    "torchaudio",
    "soundfile>=0.12",
];
const NEMO_PACKAGES: &[&str] = &[
    "torch",
    "Cython",
    "packaging",
    "huggingface_hub[hf_xet]",
    "numba==0.62.1",
    "nemo-toolkit[asr]==2.7.3",
];
const QWEN3_PACKAGES: &[&str] = &["qwen-tts==0.1.1", "soundfile"];
const CHATTERBOX_PACKAGES: &[&str] = &[
    "torch",
    "torchaudio",
    "setuptools==80.9.0",
    "numpy>=1.26",
    "librosa==0.11.0",
    "s3tokenizer",
    "transformers==4.46.3",
    "diffusers==0.29.0",
    "resemble-perth==1.0.1",
    "PyYAML>=6.0",
    "omegaconf==2.3.0",
    "conformer==0.3.2",
    "safetensors==0.5.3",
    "huggingface_hub[hf_xet]",
];
const QWEN_OMNI_PACKAGES: &[&str] = &[
    "transformers==4.52.3",
    "qwen-omni-utils",
    "accelerate",
    "soundfile",
    "librosa",
    "torch",
    "torchaudio",
    "torchvision",
];
const RVC_PACKAGES: &[&str] = &[
    "transformers>=4.49,<4.50",
    "numpy==1.26.4",
    "numba==0.58.1",
    "librosa>=0.10.1,<0.11",
    "praat-parselmouth>=0.4.3,<1",
    "pyworld>=0.3.4,<0.4",
    "torchcrepe>=0.0.22,<0.1",
    "av==12.0.0",
    "faiss-cpu>=1.7.4,<2",
    "python-dotenv>=1,<2",
    "pydub>=0.25,<1",
];

const COSYVOICE_SOURCE: AdapterSourceSpec = AdapterSourceSpec {
    repository: "https://github.com/FunAudioLLM/CosyVoice.git",
    revision: "074ca6dc9e80a2f424f1f74b48bdd7d3fea531cc",
    recursive: true,
    requirement_files: &["requirements.txt"],
    editable: false,
};
const FISH_SOURCE: AdapterSourceSpec = AdapterSourceSpec {
    repository: "https://github.com/fishaudio/fish-speech.git",
    revision: "e5e292632cb11e7a27b2b7487f58f612bc101e13",
    recursive: false,
    requirement_files: &[],
    editable: true,
};
const OPENVOICE_SOURCE: AdapterSourceSpec = AdapterSourceSpec {
    repository: "https://github.com/myshell-ai/OpenVoice.git",
    revision: "74a1d147b17a8c3092dd5430504bd83ef6c7eb23",
    recursive: false,
    requirement_files: &[],
    editable: false,
};
const GPT_SOVITS_SOURCE: AdapterSourceSpec = AdapterSourceSpec {
    repository: "https://github.com/RVC-Boss/GPT-SoVITS.git",
    revision: "be6a4f1e9d8a22d41b7d42c22df9d7ef36f225d2",
    recursive: true,
    requirement_files: &["requirements.txt"],
    editable: false,
};
const RVC_SOURCE: AdapterSourceSpec = AdapterSourceSpec {
    repository: "https://github.com/RVC-Project/Retrieval-based-Voice-Conversion.git",
    revision: "7b284a634667c34103eaaeed972b48ccdb4b893e",
    recursive: false,
    requirement_files: &[],
    editable: false,
};

macro_rules! adapter {
    ($id:literal, $family:literal, $python:literal, $packages:expr, $script:expr, $source:expr, $note:literal) => {
        AdapterSpec {
            id: $id,
            model_family: $family,
            python: $python,
            packages: $packages,
            no_deps_packages: &[],
            script: Some($script),
            source: $source,
            note: $note,
        }
    };
}

macro_rules! adapter_with_no_deps {
    ($id:literal, $family:literal, $python:literal, $packages:expr, $no_deps:expr, $script:expr, $source:expr, $note:literal) => {
        AdapterSpec {
            id: $id,
            model_family: $family,
            python: $python,
            packages: $packages,
            no_deps_packages: $no_deps,
            script: Some($script),
            source: $source,
            note: $note,
        }
    };
}

pub(crate) const ADAPTER_SPECS: &[AdapterSpec] = &[
    adapter!(
        "qwen3_tts",
        "qwen3-tts",
        "3.11",
        QWEN3_PACKAGES,
        QWEN3_TTS_ADAPTER,
        None,
        "Qwen3-TTS 0.6B CustomVoice."
    ),
    adapter!(
        "qwen3_tts",
        "qwen3-tts-0.6b-base",
        "3.11",
        QWEN3_PACKAGES,
        QWEN3_TTS_ADAPTER,
        None,
        "Qwen3-TTS 0.6B Base."
    ),
    adapter!(
        "qwen3_tts",
        "qwen3-tts-1.7b-custom",
        "3.11",
        QWEN3_PACKAGES,
        QWEN3_TTS_ADAPTER,
        None,
        "Qwen3-TTS 1.7B CustomVoice."
    ),
    adapter!(
        "qwen3_tts",
        "qwen3-tts-1.7b-base",
        "3.11",
        QWEN3_PACKAGES,
        QWEN3_TTS_ADAPTER,
        None,
        "Qwen3-TTS 1.7B Base."
    ),
    adapter!(
        "qwen3_tts",
        "qwen3-tts-1.7b-voice-design",
        "3.11",
        QWEN3_PACKAGES,
        QWEN3_TTS_ADAPTER,
        None,
        "Qwen3-TTS 1.7B VoiceDesign."
    ),
    adapter_with_no_deps!(
        "chatterbox",
        "chatterbox",
        "3.11",
        CHATTERBOX_PACKAGES,
        &["chatterbox-tts==0.1.2"],
        CHATTERBOX_ADAPTER,
        None,
        "Chatterbox TTS."
    ),
    adapter!(
        "f5_tts",
        "f5-tts",
        "3.11",
        &[
            "f5-tts==1.1.21",
            "huggingface_hub[hf_xet]",
            "soundfile>=0.12"
        ],
        F5_TTS_ADAPTER,
        None,
        "F5-TTS."
    ),
    adapter!(
        "dia",
        "dia",
        "3.11",
        &[
            "git+https://github.com/huggingface/transformers.git",
            "torch",
            "accelerate",
            "huggingface_hub[hf_xet]",
            "soundfile"
        ],
        DIA_ADAPTER,
        None,
        "Dia TTS."
    ),
    adapter!(
        "sensevoice",
        "sensevoice",
        "3.11",
        &["torch", "torchaudio", "funasr", "modelscope"],
        SENSEVOICE_ADAPTER,
        None,
        "SenseVoice ASR."
    ),
    adapter!(
        "voxtral",
        "voxtral",
        "3.11",
        &[
            "git+https://github.com/huggingface/transformers.git",
            "torch",
            "accelerate",
            "huggingface_hub[hf_xet]",
            "soundfile",
            "librosa",
            "mistral-common[audio]"
        ],
        VOXTRAL_ADAPTER,
        None,
        "Voxtral ASR."
    ),
    adapter!(
        "nemo_asr",
        "canary",
        "3.12",
        NEMO_PACKAGES,
        NEMO_ASR_ADAPTER,
        None,
        "Canary ASR."
    ),
    adapter!(
        "nemo_asr",
        "parakeet",
        "3.12",
        NEMO_PACKAGES,
        NEMO_ASR_ADAPTER,
        None,
        "Parakeet ASR."
    ),
    adapter!(
        "hf_audio",
        "bark-small",
        "3.11",
        HF_AUDIO_PACKAGES,
        HF_AUDIO_ADAPTER,
        None,
        "Bark TTS."
    ),
    adapter!(
        "hf_audio",
        "mms-tts-eng",
        "3.11",
        HF_AUDIO_PACKAGES,
        HF_AUDIO_ADAPTER,
        None,
        "MMS TTS."
    ),
    adapter!(
        "hf_audio",
        "distil-whisper-large-v3",
        "3.11",
        HF_AUDIO_PACKAGES,
        HF_AUDIO_ADAPTER,
        None,
        "Distil-Whisper ASR."
    ),
    adapter!(
        "hf_audio",
        "wav2vec2-base-960h",
        "3.11",
        HF_AUDIO_PACKAGES,
        HF_AUDIO_ADAPTER,
        None,
        "Wav2Vec2 ASR."
    ),
    adapter!(
        "coqui_tts",
        "xtts-v2",
        "3.11",
        COQUI_PACKAGES,
        COQUI_TTS_ADAPTER,
        None,
        "XTTS v2."
    ),
    adapter!(
        "coqui_tts",
        "yourtts",
        "3.11",
        COQUI_PACKAGES,
        COQUI_TTS_ADAPTER,
        None,
        "YourTTS."
    ),
    adapter!(
        "kyutai_tts",
        "kyutai-tts-1.6b",
        "3.12",
        &[
            "moshi==0.2.11",
            "torch",
            "sphn",
            "numpy",
            "huggingface_hub[hf_xet]"
        ],
        KYUTAI_TTS_ADAPTER,
        None,
        "Kyutai DSM TTS."
    ),
    adapter!(
        "piper",
        "piper-lessac",
        "3.11",
        &["piper-tts==1.5.0"],
        PIPER_ADAPTER,
        None,
        "Piper TTS."
    ),
    adapter!(
        "cosyvoice2",
        "cosyvoice2",
        "3.10",
        &[
            "torch>=2.13.0",
            "torchaudio>=2.11.0",
            "soundfile>=0.12",
            "setuptools==80.9.0"
        ],
        COSYVOICE2_ADAPTER,
        Some(COSYVOICE_SOURCE),
        "CosyVoice2."
    ),
    adapter!(
        "fish_speech",
        "fish-speech",
        "3.12",
        &["soundfile"],
        FISH_SPEECH_ADAPTER,
        Some(FISH_SOURCE),
        "Fish Speech S2 Pro."
    ),
    adapter!(
        "openvoice",
        "openvoice",
        "3.11",
        &[
            "torch",
            "torchaudio",
            "huggingface_hub",
            "soundfile",
            "setuptools==80.9.0",
            "unidic-lite",
            "wavmark==0.0.3",
            "git+https://github.com/myshell-ai/MeloTTS.git@209145371cff8fc3bd60d7be902ea69cbdb7965a"
        ],
        OPENVOICE_ADAPTER,
        Some(OPENVOICE_SOURCE),
        "OpenVoice V2."
    ),
    adapter!(
        "gpt_sovits",
        "gpt-sovits",
        "3.10",
        &["pyyaml", "soundfile>=0.12"],
        GPT_SOVITS_ADAPTER,
        Some(GPT_SOVITS_SOURCE),
        "GPT-SoVITS."
    ),
    adapter!(
        "rvc",
        "rvc",
        "3.11",
        RVC_PACKAGES,
        RVC_ADAPTER,
        Some(RVC_SOURCE),
        "RVC conversion using pinned source plus a pure-Python Transformers HuBERT compatibility layer; the upstream fairseq package is not compiled."
    ),
    adapter!(
        "qwen_omni",
        "qwen2-5-omni",
        "3.11",
        QWEN_OMNI_PACKAGES,
        QWEN_OMNI_ADAPTER,
        None,
        "Qwen2.5-Omni."
    ),
    adapter!(
        "qwen_omni",
        "qwen3-omni",
        "3.11",
        QWEN_OMNI_PACKAGES,
        QWEN_OMNI_ADAPTER,
        None,
        "Qwen3-Omni."
    ),
];