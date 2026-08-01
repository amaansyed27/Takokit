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
    pub no_deps_packages: &'static [&'static str],
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

pub fn runtime_model_id(model: &crate::ModelManifest) -> &str {
    if adapter_for_model(&model.id).is_some() {
        return &model.id;
    }
    if let (Some(required), Some(base_adapter)) = (
        model.required_adapter.as_deref(),
        adapter_for_model(&model.family),
    ) {
        if required == base_adapter {
            return &model.family;
        }
    }
    &model.id
}

pub(crate) fn adapter_dependency_overrides(id: &str) -> &'static [&'static str] {
    match id {
        "rvc" => &["av==12.0.0"],
        _ => &[],
    }
}

pub(crate) fn sanitized_adapter_requirements(
    id: &str,
    target_os: &str,
    requirements: &str,
) -> Option<String> {
    if id != "cosyvoice2" || !matches!(target_os, "windows" | "macos") {
        return None;
    }

    let mut sanitized = String::with_capacity(requirements.len());
    for line in requirements.lines() {
        let trimmed = line.trim();
        if line
            .contains("aiinfra.pkgs.visualstudio.com/PublicPackages/_packaging/onnxruntime-cuda-12")
        {
            continue;
        }
        if target_os == "windows"
            && ((trimmed.starts_with("--extra-index-url")
                && trimmed.contains("download.pytorch.org/whl/cu121"))
                || trimmed.starts_with("torch==")
                || trimmed.starts_with("torchaudio=="))
        {
            continue;
        }
        sanitized.push_str(line);
        sanitized.push('\n');
    }

    (sanitized != requirements).then_some(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rvc_uses_portable_pyav_override() {
        assert_eq!(adapter_dependency_overrides("rvc"), &["av==12.0.0"]);
        assert!(adapter_dependency_overrides("openvoice").is_empty());
    }

    #[test]
    fn cosyvoice_isolates_linux_indexes_and_old_torch_off_windows() {
        let requirements = concat!(
            "--extra-index-url ",
            "https://aiinfra.pkgs.visualstudio.com/PublicPackages/",
            "_packaging/onnxruntime-cuda-12/pypi/simple/\n",
            "--extra-index-url https://download.pytorch.org/whl/cu121\n",
            "torch==2.3.1\n",
            "torchaudio==2.3.1\n",
            "onnxruntime==1.18.0; sys_platform == 'win32'\n",
            "protobuf==4.25\n",
        );

        let sanitized = sanitized_adapter_requirements("cosyvoice2", "windows", requirements)
            .expect("Windows requirements should be sanitized");
        assert!(!sanitized.contains("onnxruntime-cuda-12"));
        assert!(!sanitized.contains("download.pytorch.org/whl/cu121"));
        assert!(!sanitized.contains("torch==2.3.1"));
        assert!(!sanitized.contains("torchaudio==2.3.1"));
        assert!(sanitized.contains("onnxruntime==1.18.0"));
        assert!(sanitized.contains("protobuf==4.25"));

        let macos = sanitized_adapter_requirements("cosyvoice2", "macos", requirements)
            .expect("macOS requirements should be sanitized");
        assert!(!macos.contains("onnxruntime-cuda-12"));
        assert!(macos.contains("torch==2.3.1"));

        assert!(sanitized_adapter_requirements("cosyvoice2", "linux", requirements).is_none());
        assert!(sanitized_adapter_requirements("openvoice", "windows", requirements).is_none());
    }

    #[test]
    fn rvc_reuses_primary_python_abi() {
        let spec = adapter_spec("rvc").expect("RVC adapter spec");
        assert_eq!(spec.python, "3.11");
    }

    #[test]
    fn coqui_pins_transformers_four_for_yourtts_and_xtts() {
        let spec = adapter_spec("coqui_tts").expect("Coqui adapter spec");
        assert!(spec.packages.contains(&"coqui-tts==0.27.5"));
        assert!(spec.packages.contains(&"transformers==4.57.6"));
        assert!(spec.packages.contains(&"soundfile>=0.12"));
        assert!(!spec.packages.contains(&"torchcodec"));
        assert!(COQUI_TTS_ADAPTER.contains("ensure_compatible_transformers"));
        assert!(COQUI_TTS_ADAPTER.contains("ensure_xtts_terms_accepted"));
        assert!(COQUI_TTS_ADAPTER.contains("valid_xtts_license_receipt"));
        assert!(COQUI_TTS_ADAPTER.contains("install_soundfile_torchaudio_io"));
        assert!(COQUI_TTS_ADAPTER.contains("licenses"));
        assert!(COQUI_TTS_ADAPTER.contains("receipts"));
        assert!(COQUI_TTS_ADAPTER
            .contains("sha256:3dbb31aa8875793cde77882e71dbb5f80fe31b818ecca4a4a5812a430f7209c7"));
        assert!(COQUI_TTS_ADAPTER.contains("except SystemExit"));
        assert!(COQUI_TTS_ADAPTER.contains("coqui_model_root"));
    }

    #[test]
    fn f5_pins_release_and_avoids_runner_module_shadowing() {
        let spec = adapter_spec("f5_tts").expect("F5 adapter spec");
        assert!(spec.packages.contains(&"f5-tts==1.1.21"));
        assert!(spec.packages.contains(&"soundfile>=0.12"));
        assert!(F5_TTS_ADAPTER.contains("load_f5tts_api"));
        assert!(F5_TTS_ADAPTER.contains("install_soundfile_torchaudio_io"));
        assert!(F5_TTS_ADAPTER.contains("create_engine"));
        assert!(F5_TTS_ADAPTER.contains("device=\"cpu\""));
        assert!(F5_TTS_ADAPTER.contains("cuda_error_allows_cpu_retry"));
        assert!(F5_TTS_ADAPTER.contains("seed=0"));
    }

    #[test]
    fn targeted_runtime_round_three_is_covered() {
        let chatterbox = adapter_spec("chatterbox").expect("Chatterbox adapter spec");
        let cosyvoice = adapter_spec("cosyvoice2").expect("CosyVoice adapter spec");
        let openvoice = adapter_spec("openvoice").expect("OpenVoice adapter spec");
        let qwen_omni = adapter_spec("qwen_omni").expect("Qwen Omni adapter spec");
        let gpt_sovits = adapter_spec("gpt_sovits").expect("GPT-SoVITS adapter spec");

        assert!(chatterbox.packages.contains(&"setuptools==80.9.0"));
        assert!(chatterbox.packages.contains(&"PyYAML>=6.0"));
        assert!(cosyvoice.packages.contains(&"torch>=2.13.0"));
        assert!(cosyvoice.packages.contains(&"torchaudio>=2.11.0"));
        assert!(openvoice.packages.contains(&"unidic-lite"));
        assert!(qwen_omni.packages.contains(&"transformers==4.52.3"));
        assert!(gpt_sovits.packages.contains(&"soundfile>=0.12"));

        assert!(HF_AUDIO_ADAPTER.contains("whisper-asr"));
        assert!(HF_AUDIO_ADAPTER.contains("return_timestamps"));
        assert!(COSYVOICE2_ADAPTER.contains("paging_file_error"));
        assert!(COSYVOICE2_ADAPTER.contains("torch.__version__"));
        assert!(OPENVOICE_ADAPTER.contains("resolve_speaker_id"));
        assert!(OPENVOICE_ADAPTER.contains("dict(speaker_ids.items())"));
        assert!(QWEN_OMNI_ADAPTER.contains("enable_audio_output=enable_audio_output"));
        assert!(QWEN_OMNI_ADAPTER.contains("require_commit_headroom"));
        assert!(QWEN_OMNI_ADAPTER.contains("max_new_tokens=96"));
        assert!(GPT_SOVITS_ADAPTER.contains("configure_utf8_stdio"));
        assert!(GPT_SOVITS_ADAPTER.contains("PYTHONIOENCODING"));
        assert!(GPT_SOVITS_ADAPTER.contains("Last training log lines"));
    }

    #[test]
    fn final_runtime_prefetch_gaps_are_covered() {
        assert!(CHATTERBOX_ADAPTER.contains("install_pkg_resources_compat"));
        assert!(CHATTERBOX_ADAPTER.contains("resource_filename"));
        assert!(OPENVOICE_ADAPTER.contains("averaged_perceptron_tagger_eng"));
        assert!(OPENVOICE_ADAPTER.contains("configure_nltk_data"));
        assert!(OPENVOICE_ADAPTER.contains("download_missing=True"));
        assert!(GPT_SOVITS_ADAPTER.contains("prepare_fast_langdetect"));
        assert!(GPT_SOVITS_ADAPTER.contains("fast_langdetect_cache"));
        assert!(GPT_SOVITS_ADAPTER.contains("download_missing=True"));
        assert!(model_prefetch_required("openvoice"));
        assert!(model_prefetch_required("gpt-sovits"));
    }

    #[test]
    fn existing_hardware_smoke_fixes_remain_present() {
        let voxtral = adapter_spec("voxtral").expect("Voxtral adapter spec");
        assert!(voxtral.packages.contains(&"librosa"));
        assert!(CHATTERBOX_ADAPTER.contains("ensure_perth_watermarker"));
        assert!(CHATTERBOX_ADAPTER.contains("install_soundfile_torchaudio_io"));
        assert!(HF_AUDIO_ADAPTER.contains("decode_audio"));
        assert!(F5_TTS_ADAPTER.contains("install_soundfile_torchaudio_io"));
        assert!(COSYVOICE2_ADAPTER.contains("cuda_runtime_is_compatible"));
        assert!(COSYVOICE2_ADAPTER.contains("text_frontend=False"));
        assert!(OPENVOICE_ADAPTER.contains("configure_mecab_dictionary"));
        assert!(KYUTAI_TTS_ADAPTER.contains("NO_TORCH_COMPILE"));
        assert!(QWEN_OMNI_ADAPTER.contains("return_audio=False"));
        assert!(QWEN_OMNI_ADAPTER.contains("QWEN25_SYSTEM_PROMPT"));
        assert!(VOXTRAL_ADAPTER.contains("import librosa"));
        assert!(QWEN3_TTS_ADAPTER.contains("load_reference_audio"));
        assert!(RVC_ADAPTER.contains("operation"));
    }
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
            | "gpt-sovits"
            | "kyutai-tts-1.6b"
            | "mms-tts-eng"
            | "openvoice"
            | "sensevoice"
            | "voxtral"
            | "wav2vec2-base-960h"
            | "xtts-v2"
            | "yourtts"
    )
}
