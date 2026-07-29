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

pub(crate) fn adapter_dependency_overrides(id: &str) -> &'static [&'static str] {
    match id {
        // PyAV 11.0.0 only publishes a source distribution. The closest newer
        // release provides bundled FFmpeg wheels for every supported Takokit OS.
        "rvc" => &["av==12.0.0"],
        _ => &[],
    }
}

pub(crate) fn adapter_pypi_bootstrap_packages(
    id: &str,
    target_os: &str,
) -> &'static [&'static str] {
    match (id, target_os) {
        // CosyVoice's requirements add Microsoft's CUDA 12 ONNX index. On
        // Windows and macOS the project requests the CPU onnxruntime wheel,
        // which is published on PyPI instead of that auxiliary index.
        ("cosyvoice2", "windows" | "macos") => &["onnxruntime==1.18.0"],
        _ => &[],
    }
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
    fn cosyvoice_bootstraps_cpu_onnxruntime_from_pypi_off_linux() {
        assert_eq!(
            adapter_pypi_bootstrap_packages("cosyvoice2", "windows"),
            &["onnxruntime==1.18.0"]
        );
        assert_eq!(
            adapter_pypi_bootstrap_packages("cosyvoice2", "macos"),
            &["onnxruntime==1.18.0"]
        );
        assert!(adapter_pypi_bootstrap_packages("cosyvoice2", "linux").is_empty());
        assert!(adapter_pypi_bootstrap_packages("openvoice", "windows").is_empty());
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
        assert!(spec.packages.contains(&"torchcodec"));
        assert!(COQUI_TTS_ADAPTER.contains("ensure_compatible_transformers"));
        assert!(COQUI_TTS_ADAPTER.contains("ensure_xtts_terms_accepted"));
        assert!(COQUI_TTS_ADAPTER.contains("COQUI_TOS_AGREED"));
        assert!(COQUI_TTS_ADAPTER.contains("except SystemExit"));
        assert!(COQUI_TTS_ADAPTER.contains("coqui_model_root"));
        assert!(COQUI_TTS_ADAPTER.contains("coqui_home / \"tts\" / model_directory"));
        assert!(COQUI_TTS_ADAPTER.contains("Coqui checkpoint directory is empty"));
    }

    #[test]
    fn f5_pins_release_and_avoids_runner_module_shadowing() {
        let spec = adapter_spec("f5_tts").expect("F5 adapter spec");
        assert!(spec.packages.contains(&"f5-tts==1.1.21"));
        assert!(F5_TTS_ADAPTER.contains("load_f5tts_api"));
        assert!(F5_TTS_ADAPTER.contains("sys.path[:]"));
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
            | "kyutai-tts-1.6b"
            | "mms-tts-eng"
            | "sensevoice"
            | "voxtral"
            | "wav2vec2-base-960h"
            | "xtts-v2"
            | "yourtts"
    )
}
