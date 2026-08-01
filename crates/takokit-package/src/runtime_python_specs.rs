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

pub(crate) fn sanitized_adapter_requirements(
    id: &str,
    target_os: &str,
    requirements: &str,
) -> Option<String> {
    if !matches!((id, target_os), ("cosyvoice2", "windows" | "macos")) {
        return None;
    }

    let mut sanitized = String::with_capacity(requirements.len());
    for line in requirements.lines() {
        // This auxiliary repository is needed for the Linux GPU wheel, but it
        // shadows ordinary PyPI packages under uv's safe first-index policy.
        if line
            .contains("aiinfra.pkgs.visualstudio.com/PublicPackages/_packaging/onnxruntime-cuda-12")
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
    fn cosyvoice_isolates_the_linux_onnx_index_off_linux() {
        let requirements = concat!(
            "--extra-index-url ",
            "https://aiinfra.pkgs.visualstudio.com/PublicPackages/",
            "_packaging/onnxruntime-cuda-12/pypi/simple/\n",
            "onnxruntime==1.18.0; sys_platform == 'win32'\n",
            "protobuf==4.25\n",
        );

        let sanitized = sanitized_adapter_requirements("cosyvoice2", "windows", requirements)
            .expect("Windows requirements should be sanitized");
        assert!(!sanitized.contains("onnxruntime-cuda-12"));
        assert!(sanitized.contains("onnxruntime==1.18.0"));
        assert!(sanitized.contains("protobuf==4.25"));

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
        assert!(spec.packages.contains(&"torchcodec"));
        assert!(COQUI_TTS_ADAPTER.contains("ensure_compatible_transformers"));
        assert!(COQUI_TTS_ADAPTER.contains("ensure_xtts_terms_accepted"));
        assert!(COQUI_TTS_ADAPTER.contains("valid_xtts_license_receipt"));
        assert!(COQUI_TTS_ADAPTER.contains("licenses"));
        assert!(COQUI_TTS_ADAPTER.contains("receipts"));
        assert!(COQUI_TTS_ADAPTER.contains(
            "sha256:3dbb31aa8875793cde77882e71dbb5f80fe31b818ecca4a4a5812a430f7209c7"
        ));
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
        assert!(F5_TTS_ADAPTER.contains("seed=0"));
        assert!(F5_TTS_ADAPTER.contains("reference_text"));
    }

    #[test]
    fn hardware_smoke_adapters_include_runtime_compatibility_fixes() {
        let cosyvoice = adapter_spec("cosyvoice2").expect("CosyVoice adapter spec");
        let openvoice = adapter_spec("openvoice").expect("OpenVoice adapter spec");
        let voxtral = adapter_spec("voxtral").expect("Voxtral adapter spec");
        let qwen_omni = adapter_spec("qwen_omni").expect("Qwen Omni adapter spec");

        assert!(cosyvoice.packages.contains(&"setuptools==80.9.0"));
        assert!(openvoice.packages.contains(&"setuptools==80.9.0"));
        assert!(voxtral.packages.contains(&"librosa"));
        assert!(qwen_omni.packages.contains(&"torchvision"));

        assert!(CHATTERBOX_ADAPTER.contains("load_chatterbox_tts"));
        assert!(!HF_AUDIO_ADAPTER.contains("model_kwargs={\"local_files_only\": True}"));
        assert!(COSYVOICE2_ADAPTER.contains("import pkg_resources"));
        assert!(OPENVOICE_ADAPTER.contains("import pkg_resources"));
        assert!(KYUTAI_TTS_ADAPTER.contains("NO_TORCH_COMPILE"));
        assert!(QWEN_OMNI_ADAPTER.contains("import torchvision"));
        assert!(VOXTRAL_ADAPTER.contains("import librosa"));
        assert!(QWEN3_TTS_ADAPTER.contains("load_reference_audio"));
        assert!(QWEN3_TTS_ADAPTER.contains("always_2d=True"));
        assert!(GPT_SOVITS_ADAPTER.contains("Last training log lines"));
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
