#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, content: str) -> None:
    target = ROOT / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content.rstrip() + "\n", encoding="utf-8")


def replace(path: str, old: str, new: str) -> None:
    content = read(path)
    if old not in content:
        raise RuntimeError(f"missing replacement anchor in {path}: {old[:100]!r}")
    write(path, content.replace(old, new, 1))


# Hugging Face snapshot pulls are represented by a tiny verified marker artifact.
artifact_io = read("crates/takokit-package/src/artifact_io.rs")
artifact_io = artifact_io.replace(
    "use crate::{ArtifactEntry, ModelManifest, PackageError, PackageResult};",
    "use crate::{\n    runtime_command::{run_logged_command, PathOrArg}, runtime_uv::bootstrap_uv, ArtifactEntry,\n    ModelManifest, PackageError, PackageResult,\n};",
)
artifact_io = artifact_io.replace(
    ") -> PackageResult<PathBuf> {\n    let url = artifact",
    ") -> PackageResult<PathBuf> {\n    if artifact\n        .url\n        .as_deref()\n        .is_some_and(|url| url.starts_with(\"hf://\"))\n    {\n        return install_huggingface_snapshot(manifest, artifact, downloads_dir, blob_dir);\n    }\n    let url = artifact",
)
anchor = "pub(crate) fn download_to_temp(url: &str, artifact: &str, temp_path: &Path) -> PackageResult<()> {"
snapshot_fn = r'''fn install_huggingface_snapshot(
    manifest: &ModelManifest,
    artifact: &ArtifactEntry,
    downloads_dir: &Path,
    blob_dir: &Path,
) -> PackageResult<PathBuf> {
    let url = artifact
        .url
        .as_deref()
        .and_then(|value| value.strip_prefix("hf://"))
        .ok_or_else(|| PackageError::ArtifactUrlMissing {
            model: manifest.id.clone(),
            artifact: artifact.name.clone(),
        })?;
    let (repository, revision) = url
        .rsplit_once('@')
        .map(|(repository, revision)| (repository, revision))
        .unwrap_or((url, "main"));
    if repository.trim().is_empty() || revision.trim().is_empty() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: "invalid hf:// snapshot specification".to_string(),
        });
    }
    let root = blob_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: "could not resolve Takokit storage root for snapshot".to_string(),
        })?;
    let model_dir = root.join("models").join(&manifest.id);
    let helper = root.join("cache").join("hf_snapshot_download.py");
    let log = root
        .join("logs")
        .join(format!("snapshot-{}.log", manifest.id));
    std::fs::create_dir_all(&model_dir)?;
    std::fs::create_dir_all(downloads_dir)?;
    std::fs::create_dir_all(blob_dir)?;
    if let Some(parent) = helper.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &helper,
        r#"from pathlib import Path
import sys
from huggingface_hub import snapshot_download
repo, revision, output = sys.argv[1:4]
Path(output).mkdir(parents=True, exist_ok=True)
snapshot_download(repo_id=repo, revision=revision, local_dir=output)
"#,
    )?;
    let uv = bootstrap_uv(root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "run".into(),
            "--no-project".into(),
            "--with".into(),
            "huggingface_hub".into(),
            "python".into(),
            helper.into(),
            repository.into(),
            revision.into(),
            model_dir.clone().into(),
        ],
    )?;
    let has_payload = std::fs::read_dir(&model_dir)?
        .flatten()
        .any(|entry| entry.file_name() != ".takokit-snapshot");
    if !has_payload {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: format!(
                "Hugging Face snapshot produced no model files; see {}",
                log.display()
            ),
        });
    }
    let marker = format!("hf://{repository}@{revision}");
    let expected = artifact.sha256.trim().to_ascii_lowercase();
    let actual = format!("{:x}", Sha256::digest(marker.as_bytes()));
    if actual != expected {
        return Err(PackageError::ArtifactChecksumMismatch {
            artifact: artifact.name.clone(),
            expected,
            actual,
        });
    }
    if artifact.bytes.is_some_and(|bytes| bytes != marker.len() as u64) {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: "snapshot marker byte count does not match manifest".to_string(),
        });
    }
    let final_path = blob_dir.join(&actual);
    std::fs::write(&final_path, marker.as_bytes())?;
    Ok(final_path)
}

'''
if anchor not in artifact_io:
    raise RuntimeError("artifact snapshot insertion anchor missing")
artifact_io = artifact_io.replace(anchor, snapshot_fn + anchor, 1)
write("crates/takokit-package/src/artifact_io.rs", artifact_io)

artifact_reuse = read("crates/takokit-package/src/artifact_reuse.rs")
old = '''    expected.into_iter().all(|artifact| {
        record
            .artifacts
            .iter()
            .find(|candidate| candidate.name == artifact.name && candidate.role == artifact.role)
            .is_some_and(|candidate| is_verified(candidate, artifact))
    })
}'''
new = '''    let verified = expected.into_iter().all(|artifact| {
        record
            .artifacts
            .iter()
            .find(|candidate| candidate.name == artifact.name && candidate.role == artifact.role)
            .is_some_and(|candidate| is_verified(candidate, artifact))
    });
    verified && snapshot_payloads_exist(record, manifest)
}

fn snapshot_payloads_exist(record: &InstalledModelRecord, manifest: &ModelManifest) -> bool {
    manifest
        .artifacts
        .all()
        .filter(|artifact| {
            artifact
                .url
                .as_deref()
                .is_some_and(|url| url.starts_with("hf://"))
        })
        .all(|artifact| {
            let Some(marker) = record
                .artifacts
                .iter()
                .find(|candidate| candidate.name == artifact.name)
                .and_then(|candidate| candidate.local_path.as_ref())
            else {
                return false;
            };
            let Some(root) = marker
                .parent()
                .and_then(std::path::Path::parent)
                .and_then(std::path::Path::parent)
            else {
                return false;
            };
            let model_dir = root.join("models").join(&manifest.id);
            std::fs::read_dir(model_dir)
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .any(|entry| entry.file_name() != ".takokit-snapshot")
        })
}'''
if old not in artifact_reuse:
    raise RuntimeError("artifact reuse anchor missing")
write("crates/takokit-package/src/artifact_reuse.rs", artifact_reuse.replace(old, new, 1))

# Managed adapters. Source checkouts stay isolated below the adapter directory.
write(
    "crates/takokit-package/src/runtime_python_specs.rs",
    r'''//! Managed-Python adapter definitions used by model planning and installation.

const QWEN3_TTS_ADAPTER: &str = include_str!("../../../runners/python/qwen3_tts_adapter.py");
const PIPER_TTS_ADAPTER: &str = include_str!("../../../runners/python/piper_tts_adapter.py");
const CHATTERBOX_ADAPTER: &str = include_str!("../../../runners/python/chatterbox_adapter.py");
const F5_TTS_ADAPTER: &str = include_str!("../../../runners/python/f5_tts_adapter.py");
const DIA_ADAPTER: &str = include_str!("../../../runners/python/dia_adapter.py");
const SENSEVOICE_ADAPTER: &str = include_str!("../../../runners/python/sensevoice_adapter.py");
const VOXTRAL_ADAPTER: &str = include_str!("../../../runners/python/voxtral_adapter.py");
const NEMO_ASR_ADAPTER: &str = include_str!("../../../runners/python/nemo_asr_adapter.py");
const HF_AUDIO_ADAPTER: &str = include_str!("../../../runners/python/hf_audio_adapter.py");
const COQUI_TTS_ADAPTER: &str = include_str!("../../../runners/python/coqui_tts_adapter.py");
const KYUTAI_TTS_ADAPTER: &str = include_str!("../../../runners/python/kyutai_tts_adapter.py");
const OPENVOICE_ADAPTER: &str = include_str!("../../../runners/python/openvoice_adapter.py");
const COSYVOICE2_ADAPTER: &str = include_str!("../../../runners/python/cosyvoice2_adapter.py");
const FISH_SPEECH_ADAPTER: &str = include_str!("../../../runners/python/fish_speech_adapter.py");
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
    pub script: &'static str,
    pub source_repo: Option<&'static str>,
    pub source_recursive: bool,
    pub requirements: &'static [&'static str],
    pub editable_source: bool,
    pub note: &'static str,
}

const HF_AUDIO_PACKAGES: &[&str] = &[
    "torch", "torchaudio", "transformers", "accelerate", "soundfile", "scipy",
];
const COQUI_PACKAGES: &[&str] = &["coqui-tts", "torch", "torchaudio"];
const NEMO_PACKAGES: &[&str] = &["torch", "nemo-toolkit[asr]"];
const QWEN_TTS_PACKAGES: &[&str] = &["qwen-tts==0.1.1", "soundfile"];

macro_rules! adapter {
    ($id:literal, $family:literal, $python:literal, $packages:expr, $script:expr, $note:literal) => {
        AdapterSpec {
            id: $id,
            model_family: $family,
            python: $python,
            packages: $packages,
            script: $script,
            source_repo: None,
            source_recursive: false,
            requirements: &[],
            editable_source: false,
            note: $note,
        }
    };
}

pub(crate) const ADAPTER_SPECS: &[AdapterSpec] = &[
    adapter!("qwen3_tts", "qwen3-tts", "3.11", QWEN_TTS_PACKAGES, QWEN3_TTS_ADAPTER, "Qwen3-TTS custom voice generation."),
    adapter!("qwen3_tts", "qwen3-tts-0.6b-custom", "3.11", QWEN_TTS_PACKAGES, QWEN3_TTS_ADAPTER, "Qwen3-TTS 0.6B custom voice generation."),
    adapter!("qwen3_tts", "qwen3-tts-0.6b-base", "3.11", QWEN_TTS_PACKAGES, QWEN3_TTS_ADAPTER, "Qwen3-TTS 0.6B zero-shot voice cloning."),
    adapter!("qwen3_tts", "qwen3-tts-1.7b-custom", "3.11", QWEN_TTS_PACKAGES, QWEN3_TTS_ADAPTER, "Qwen3-TTS 1.7B custom voice generation."),
    adapter!("qwen3_tts", "qwen3-tts-1.7b-base", "3.11", QWEN_TTS_PACKAGES, QWEN3_TTS_ADAPTER, "Qwen3-TTS 1.7B zero-shot voice cloning."),
    adapter!("qwen3_tts", "qwen3-tts-1.7b-voice-design", "3.11", QWEN_TTS_PACKAGES, QWEN3_TTS_ADAPTER, "Qwen3-TTS 1.7B natural-language voice design."),
    adapter!("piper_tts", "piper-lessac", "3.11", &["piper-tts"], PIPER_TTS_ADAPTER, "Piper command-line TTS in an isolated Takokit environment."),
    adapter!("chatterbox", "chatterbox", "3.11", &["chatterbox-tts", "torchaudio"], CHATTERBOX_ADAPTER, "Chatterbox TTS and reference-audio cloning."),
    adapter!("f5_tts", "f5-tts", "3.11", &["f5-tts"], F5_TTS_ADAPTER, "F5-TTS inference and reference-audio transfer."),
    adapter!("dia", "dia", "3.11", &["git+https://github.com/huggingface/transformers.git", "torch", "accelerate", "soundfile"], DIA_ADAPTER, "Dia dialogue generation."),
    adapter!("sensevoice", "sensevoice", "3.11", &["torch", "torchaudio", "funasr", "modelscope"], SENSEVOICE_ADAPTER, "SenseVoice multilingual transcription."),
    adapter!("voxtral", "voxtral", "3.11", &["transformers>=4.57", "torch", "accelerate", "soundfile", "mistral-common[audio]"], VOXTRAL_ADAPTER, "Voxtral multilingual transcription."),
    adapter!("nemo_asr", "canary", "3.12", NEMO_PACKAGES, NEMO_ASR_ADAPTER, "NVIDIA Canary transcription."),
    adapter!("nemo_asr", "parakeet", "3.12", NEMO_PACKAGES, NEMO_ASR_ADAPTER, "NVIDIA Parakeet transcription."),
    adapter!("hf_audio", "bark-small", "3.11", HF_AUDIO_PACKAGES, HF_AUDIO_ADAPTER, "Bark Small generation."),
    adapter!("hf_audio", "mms-tts-eng", "3.11", HF_AUDIO_PACKAGES, HF_AUDIO_ADAPTER, "MMS-TTS generation."),
    adapter!("hf_audio", "distil-whisper-large-v3", "3.11", HF_AUDIO_PACKAGES, HF_AUDIO_ADAPTER, "Distil-Whisper transcription."),
    adapter!("hf_audio", "wav2vec2-base-960h", "3.11", HF_AUDIO_PACKAGES, HF_AUDIO_ADAPTER, "Wav2Vec2 transcription."),
    adapter!("coqui_tts", "xtts-v2", "3.11", COQUI_PACKAGES, COQUI_TTS_ADAPTER, "XTTS v2 zero-shot cloning."),
    adapter!("coqui_tts", "yourtts", "3.11", COQUI_PACKAGES, COQUI_TTS_ADAPTER, "YourTTS zero-shot cloning."),
    adapter!("kyutai_tts", "kyutai-tts-1.6b", "3.12", &["moshi==0.2.11", "torch", "sphn", "numpy"], KYUTAI_TTS_ADAPTER, "Kyutai DSM TTS."),
    adapter!("openvoice", "openvoice", "3.11", &["git+https://github.com/myshell-ai/OpenVoice.git", "git+https://github.com/myshell-ai/MeloTTS.git", "huggingface_hub", "torch", "torchaudio", "soundfile"], OPENVOICE_ADAPTER, "OpenVoice V2 tone-colour cloning."),
    AdapterSpec {
        id: "cosyvoice2", model_family: "cosyvoice2", python: "3.10", packages: &[], script: COSYVOICE2_ADAPTER,
        source_repo: Some("https://github.com/FunAudioLLM/CosyVoice.git"), source_recursive: true,
        requirements: &["requirements.txt"], editable_source: false,
        note: "CosyVoice2 cross-lingual zero-shot generation through the official source tree.",
    },
    AdapterSpec {
        id: "fish_speech", model_family: "fish-speech", python: "3.10", packages: &["soundfile"], script: FISH_SPEECH_ADAPTER,
        source_repo: Some("https://github.com/fishaudio/fish-speech.git"), source_recursive: false,
        requirements: &[], editable_source: true,
        note: "Fish Speech S2 official inference pipeline; requires a high-memory GPU.",
    },
    AdapterSpec {
        id: "gpt_sovits", model_family: "gpt-sovits", python: "3.10", packages: &[], script: GPT_SOVITS_ADAPTER,
        source_repo: Some("https://github.com/RVC-Boss/GPT-SoVITS.git"), source_recursive: false,
        requirements: &["extra-req.txt", "requirements.txt"], editable_source: false,
        note: "GPT-SoVITS zero-shot TTS through the official inference package.",
    },
    adapter!("rvc", "rvc", "3.10", &["git+https://github.com/RVC-Project/Retrieval-based-Voice-Conversion"], RVC_ADAPTER, "RVC local voice conversion."),
    adapter!("qwen25_omni", "qwen2-5-omni", "3.11", &["transformers==4.52.3", "qwen-omni-utils", "accelerate", "torch", "torchaudio", "soundfile"], QWEN25_OMNI_ADAPTER, "Qwen2.5-Omni local audio transcription."),
    adapter!("qwen3_omni", "qwen3-omni", "3.11", &["transformers>=5.2.0", "qwen-omni-utils", "accelerate", "torch", "torchaudio", "soundfile"], QWEN3_OMNI_ADAPTER, "Qwen3-Omni local audio transcription."),
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
''',
)

runtime_python = read("crates/takokit-package/src/runtime_python.rs")
runtime_python = runtime_python.replace(
    '''    let script = spec
        .script
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: spec.id.to_string(),
            reason: format!(
                "{} is catalogued but its official adapter is not implemented yet",
                spec.model_family
            ),
        })?;
    if spec.packages.is_empty() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: spec.id.to_string(),
            reason: "adapter has no verified dependency set".to_string(),
        });
    }''',
    '''    let script = spec.script;
    if spec.packages.is_empty()
        && spec.requirements.is_empty()
        && !spec.editable_source
    {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: spec.id.to_string(),
            reason: "adapter has no dependency installation strategy".to_string(),
        });
    }''',
)
old = '''    let mut arguments: Vec<PathOrArg> = vec![
        "pip".into(),
        "install".into(),
        "--python".into(),
        python.into(),
        "--no-progress".into(),
    ];
    arguments.extend(spec.packages.iter().map(|package| (*package).into()));
    run_logged_command(&log, &uv, &arguments)?;
    std::fs::write(adapter_dir.join(format!("{}.py", spec.id)), script)?;'''
new = '''    let source_dir = adapter_dir.join("source");
    if let Some(repository) = spec.source_repo {
        if !source_dir.join(".git").is_dir() {
            let mut clone: Vec<PathOrArg> = vec!["clone".into(), "--depth".into(), "1".into()];
            if spec.source_recursive {
                clone.push("--recursive".into());
            }
            clone.push(repository.into());
            clone.push(source_dir.clone().into());
            run_logged_command(&log, "git", &clone)?;
        }
    }
    if !spec.packages.is_empty() {
        let mut arguments: Vec<PathOrArg> = vec![
            "pip".into(), "install".into(), "--python".into(), python.clone().into(),
            "--no-progress".into(),
        ];
        arguments.extend(spec.packages.iter().map(|package| (*package).into()));
        run_logged_command(&log, &uv, &arguments)?;
    }
    for requirements in spec.requirements {
        run_logged_command(
            &log,
            &uv,
            &[
                "pip".into(), "install".into(), "--python".into(), python.clone().into(),
                "--no-progress".into(), "-r".into(), source_dir.join(requirements).into(),
            ],
        )?;
    }
    if spec.editable_source {
        run_logged_command(
            &log,
            &uv,
            &[
                "pip".into(), "install".into(), "--python".into(), python.into(),
                "--no-progress".into(), "--editable".into(), source_dir.into(),
            ],
        )?;
    }
    std::fs::write(adapter_dir.join(format!("{}.py", spec.id)), script)?;'''
if old not in runtime_python:
    raise RuntimeError("runtime python install anchor missing")
write("crates/takokit-package/src/runtime_python.rs", runtime_python.replace(old, new, 1))

# Qwen3-TTS adapter branches by official checkpoint capability.
write(
    "runners/python/qwen3_tts_adapter.py",
    r'''"""Takokit JSON adapter for all official Qwen3-TTS checkpoints."""
from __future__ import annotations

import json
import sys
from contextlib import redirect_stdout
from pathlib import Path

import soundfile as sf


def fail(message: str) -> None:
    print(json.dumps({"ok": False, "error": message}), flush=True)
    raise SystemExit(1)


def main() -> None:
    try:
        request = json.load(sys.stdin)
        text = str(request.get("input") or "").strip()
        if request.get("operation") != "speech" or not text:
            fail("Qwen3-TTS requires a non-empty speech request")
        model_id = str(request["model_id"])
        model_dir = Path(request["model_dir"])
        output_path = Path(request["output_path"])
        if not model_dir.is_dir():
            fail(f"Qwen3-TTS model directory is missing: {model_dir}")
        with redirect_stdout(sys.stderr):
            import torch
            from qwen_tts import Qwen3TTSModel

            device = "cuda:0" if torch.cuda.is_available() else "cpu"
            dtype = torch.bfloat16 if torch.cuda.is_available() else torch.float32
            model = Qwen3TTSModel.from_pretrained(str(model_dir), device_map=device, dtype=dtype)
            voice = request.get("voice")
            if model_id.endswith("-base"):
                if not voice:
                    fail("Qwen3-TTS Base requires a consent-backed reference voice")
                reference = Path(str(voice)).expanduser().resolve()
                if not reference.is_file():
                    fail(f"voice reference does not exist: {reference}")
                wavs, sample_rate = model.generate_voice_clone(
                    text=text, language="English", ref_audio=str(reference),
                    ref_text=None, x_vector_only_mode=True,
                )
            elif model_id.endswith("voice-design"):
                instruction = str(voice or "A clear, natural, warm speaking voice.")
                wavs, sample_rate = model.generate_voice_design(
                    text=text, language="English", instruct=instruction,
                )
                voice = instruction
            else:
                speaker = str(voice or "Ryan")
                wavs, sample_rate = model.generate_custom_voice(
                    text=text, language="English", speaker=speaker, instruct="",
                )
                voice = speaker
        output_path.parent.mkdir(parents=True, exist_ok=True)
        sf.write(str(output_path), wavs[0], sample_rate)
        print(json.dumps({
            "ok": True, "output_path": str(output_path),
            "bytes": output_path.stat().st_size, "sample_rate": int(sample_rate),
            "voice": voice, "device": device,
        }), flush=True)
    except Exception as error:
        fail(f"{type(error).__name__}: {error}")


if __name__ == "__main__":
    main()
''',
)

write(
    "runners/python/piper_tts_adapter.py",
    r'''from __future__ import annotations
import json
import subprocess
import sys
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "speech":
        raise ValueError("Piper adapter only supports speech")
    text = str(request.get("input") or "").strip()
    if not text:
        raise ValueError("speech input cannot be empty")
    model_dir = Path(request["model_dir"])
    model = model_dir / "en_US-lessac-medium.onnx"
    config = model_dir / "en_US-lessac-medium.onnx.json"
    output = Path(request["output_path"])
    output.parent.mkdir(parents=True, exist_ok=True)
    command = [sys.executable, "-m", "piper", "--model", str(model), "--config", str(config), "--output_file", str(output)]
    completed = subprocess.run(command, input=text, text=True, capture_output=True)
    if completed.returncode:
        raise RuntimeError(completed.stderr.strip() or completed.stdout.strip())
    if not output.is_file():
        raise RuntimeError(f"Piper did not create {output}")
    respond(ok=True, output_path=str(output), bytes=output.stat().st_size, sample_rate=22050, voice="lessac")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
''',
)

write(
    "runners/python/cosyvoice2_adapter.py",
    r'''from __future__ import annotations
import json
import sys
from pathlib import Path

import torch
import torchaudio


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    text = str(request.get("input") or "").strip()
    voice = request.get("voice")
    if request.get("operation") != "speech" or not text:
        raise ValueError("CosyVoice2 requires a speech request")
    if not voice:
        raise ValueError("CosyVoice2 requires a consent-backed reference voice")
    reference = Path(str(voice)).expanduser().resolve()
    if not reference.is_file():
        raise FileNotFoundError(reference)
    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    sys.path.insert(0, str(source / "third_party" / "Matcha-TTS"))
    from cosyvoice.cli.cosyvoice import CosyVoice2

    engine = CosyVoice2(str(Path(request["model_dir"])), load_jit=False, load_trt=False, fp16=torch.cuda.is_available())
    chunks = list(engine.inference_cross_lingual(text, str(reference), stream=False))
    if not chunks:
        raise RuntimeError("CosyVoice2 returned no audio")
    audio = torch.cat([chunk["tts_speech"] for chunk in chunks], dim=1)
    output = Path(request["output_path"])
    output.parent.mkdir(parents=True, exist_ok=True)
    torchaudio.save(str(output), audio.cpu(), engine.sample_rate)
    respond(ok=True, output_path=str(output), bytes=output.stat().st_size, sample_rate=int(engine.sample_rate), voice=str(reference))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
''',
)

write(
    "runners/python/fish_speech_adapter.py",
    r'''from __future__ import annotations
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def run(command, cwd):
    completed = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
    if completed.returncode:
        raise RuntimeError(completed.stderr.strip() or completed.stdout.strip())


def main():
    request = json.load(sys.stdin)
    text = str(request.get("input") or "").strip()
    if request.get("operation") != "speech" or not text:
        raise ValueError("Fish Speech requires a speech request")
    source = Path(__file__).resolve().parent / "source"
    checkpoint = Path(request["model_dir"])
    output = Path(request["output_path"])
    voice = request.get("voice")
    with tempfile.TemporaryDirectory(prefix="takokit-fish-") as temp:
        temp = Path(temp)
        prompt_tokens = None
        if voice:
            reference = Path(str(voice)).expanduser().resolve()
            run([sys.executable, "fish_speech/models/dac/inference.py", "-i", str(reference), "--checkpoint-path", str(checkpoint / "codec.pth")], temp)
            prompt_tokens = next(temp.glob("*.npy"), None)
        command = [sys.executable, str(source / "fish_speech/models/text2semantic/inference.py"), "--text", text, "--checkpoint-path", str(checkpoint)]
        if prompt_tokens:
            command.extend(["--prompt-tokens", str(prompt_tokens), "--prompt-text", ""])
        run(command, temp)
        codes = next(temp.glob("codes_*.npy"), None)
        if not codes:
            raise RuntimeError("Fish Speech produced no semantic token file")
        run([sys.executable, str(source / "fish_speech/models/dac/inference.py"), "-i", str(codes), "--checkpoint-path", str(checkpoint / "codec.pth")], temp)
        generated = next(temp.glob("*.wav"), None)
        if not generated:
            raise RuntimeError("Fish Speech produced no WAV")
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(generated, output)
    respond(ok=True, output_path=str(output), bytes=output.stat().st_size, sample_rate=None, voice=voice or "random")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
''',
)

write(
    "runners/python/gpt_sovits_adapter.py",
    r'''from __future__ import annotations
import json
import sys
from pathlib import Path

import numpy as np
import soundfile as sf


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    text = str(request.get("input") or "").strip()
    voice = request.get("voice")
    if request.get("operation") != "speech" or not text:
        raise ValueError("GPT-SoVITS requires a speech request")
    if not voice:
        raise ValueError("GPT-SoVITS requires a consent-backed reference voice")
    source = Path(__file__).resolve().parent / "source"
    sys.path.insert(0, str(source))
    from GPT_SoVITS.TTS_infer_pack.TTS import TTS, TTS_Config

    model_dir = Path(request["model_dir"])
    config_candidates = list(model_dir.rglob("tts_infer.yaml"))
    config = TTS_Config(str(config_candidates[0])) if config_candidates else TTS_Config()
    engine = TTS(config)
    payload = {
        "text": text, "text_lang": "en", "ref_audio_path": str(Path(str(voice)).resolve()),
        "prompt_text": "", "prompt_lang": "en", "text_split_method": "cut5",
        "batch_size": 1, "media_type": "wav", "streaming_mode": False,
    }
    pieces = list(engine.run(payload))
    if not pieces:
        raise RuntimeError("GPT-SoVITS returned no audio")
    sample_rate, audio = pieces[-1]
    output = Path(request["output_path"])
    output.parent.mkdir(parents=True, exist_ok=True)
    sf.write(str(output), np.asarray(audio), int(sample_rate))
    respond(ok=True, output_path=str(output), bytes=output.stat().st_size, sample_rate=int(sample_rate), voice=str(voice))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
''',
)

write(
    "runners/python/rvc_adapter.py",
    r'''from __future__ import annotations
import json
import subprocess
import sys
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "convert":
        raise ValueError("RVC adapter requires a convert operation")
    audio = Path(request["audio_path"]).expanduser().resolve()
    voice = Path(str(request.get("voice") or "")).expanduser().resolve()
    output = Path(request["output_path"]).expanduser().resolve()
    if not audio.is_file() or not voice.is_file():
        raise FileNotFoundError("RVC requires input audio and a consent-backed .pth voice model")
    output.parent.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run([sys.executable, "-m", "rvc.wrapper.cli.cli", "infer", "-m", str(voice), "-i", str(audio), "-o", str(output)], capture_output=True, text=True)
    if completed.returncode:
        raise RuntimeError(completed.stderr.strip() or completed.stdout.strip())
    respond(ok=True, output_path=str(output), bytes=output.stat().st_size, sample_rate=None, voice=str(voice))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
''',
)

write(
    "runners/python/qwen25_omni_adapter.py",
    r'''from __future__ import annotations
import json
import sys
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "transcribe":
        raise ValueError("Qwen2.5-Omni adapter only supports transcription")
    audio = str(Path(request["audio_path"]).resolve())
    model_dir = str(Path(request["model_dir"]).resolve())
    from transformers import Qwen2_5OmniForConditionalGeneration, Qwen2_5OmniProcessor
    from qwen_omni_utils import process_mm_info
    model = Qwen2_5OmniForConditionalGeneration.from_pretrained(model_dir, torch_dtype="auto", device_map="auto")
    model.disable_talker()
    processor = Qwen2_5OmniProcessor.from_pretrained(model_dir)
    conversation = [{"role": "user", "content": [{"type": "audio", "audio": audio}, {"type": "text", "text": "Transcribe this audio exactly. Output only the transcript."}]}]
    prompt = processor.apply_chat_template(conversation, add_generation_prompt=True, tokenize=False)
    audios, images, videos = process_mm_info(conversation, use_audio_in_video=False)
    inputs = processor(text=prompt, audio=audios, images=images, videos=videos, return_tensors="pt", padding=True, use_audio_in_video=False)
    inputs = inputs.to(model.device).to(model.dtype)
    generated = model.generate(**inputs, return_audio=False, use_audio_in_video=False)
    text = processor.batch_decode(generated[:, inputs["input_ids"].shape[1]:], skip_special_tokens=True, clean_up_tokenization_spaces=False)[0].strip()
    respond(ok=True, text=text)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
''',
)

write(
    "runners/python/qwen3_omni_adapter.py",
    r'''from __future__ import annotations
import json
import sys
from pathlib import Path


def respond(**payload):
    print(json.dumps(payload), flush=True)


def main():
    request = json.load(sys.stdin)
    if request.get("operation") != "transcribe":
        raise ValueError("Qwen3-Omni adapter only supports transcription")
    audio = str(Path(request["audio_path"]).resolve())
    model_dir = str(Path(request["model_dir"]).resolve())
    from transformers import Qwen3OmniMoeForConditionalGeneration, Qwen3OmniMoeProcessor
    from qwen_omni_utils import process_mm_info
    model = Qwen3OmniMoeForConditionalGeneration.from_pretrained(model_dir, dtype="auto", device_map="auto")
    model.disable_talker()
    processor = Qwen3OmniMoeProcessor.from_pretrained(model_dir)
    conversation = [{"role": "user", "content": [{"type": "audio", "audio": audio}, {"type": "text", "text": "Transcribe this audio exactly. Output only the transcript."}]}]
    prompt = processor.apply_chat_template(conversation, add_generation_prompt=True, tokenize=False)
    audios, images, videos = process_mm_info(conversation, use_audio_in_video=False)
    inputs = processor(text=prompt, audio=audios, images=images, videos=videos, return_tensors="pt", padding=True, use_audio_in_video=False)
    inputs = inputs.to(model.device).to(model.dtype)
    generated, _ = model.generate(**inputs, return_audio=False, use_audio_in_video=False)
    sequences = generated.sequences if hasattr(generated, "sequences") else generated
    text = processor.batch_decode(sequences[:, inputs["input_ids"].shape[1]:], skip_special_tokens=True, clean_up_tokenization_spaces=False)[0].strip()
    respond(ok=True, text=text)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        respond(ok=False, error=f"{type(error).__name__}: {error}")
        raise
''',
)

# Helpers for verified snapshot marker manifests.
import hashlib

def snapshot_manifest(model_id: str, name: str, repo: str, kind: str, adapter_id: str, capabilities: dict[str, bool], license_name: str, min_ram: str, description: str, revision: str = "main") -> str:
    marker = f"hf://{repo}@{revision}"
    digest = hashlib.sha256(marker.encode()).hexdigest()
    caps = "\n".join(f"{key} = {'true' if value else 'false'}" for key, value in capabilities.items())
    return f'''id = "{model_id}"
name = "{name}"
family = "{model_id.split('-')[0]}"
version = "0.2.0"
kind = "{kind}"
backend = "python-managed"
runner = "takokit-python-managed"
required_adapter = "{adapter_id}"
license = "{license_name}"
description = "{description}"

[capabilities]
{caps}

[hardware]
cpu = true
gpu = true
min_ram = "{min_ram}"

[artifacts]
metadata_only = false
weights = []
voices = []

[[artifacts.configs]]
name = ".takokit-snapshot"
url = "{marker}"
sha256 = "{digest}"
bytes = {len(marker)}
role = "other"
'''

common_tts = {"tts": True, "stt": False, "voice_cloning": False, "live_transcription": False, "live_audio": False}
clone_tts = {"tts": True, "stt": False, "voice_cloning": True, "live_transcription": False, "live_audio": False}
omni_stt = {"tts": False, "stt": True, "voice_cloning": False, "live_transcription": False, "live_audio": False}

for model_id, name, repo, caps in [
    ("qwen3-tts-0.6b-custom", "Qwen3-TTS 0.6B CustomVoice", "Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice", common_tts),
    ("qwen3-tts-0.6b-base", "Qwen3-TTS 0.6B Base", "Qwen/Qwen3-TTS-12Hz-0.6B-Base", clone_tts),
    ("qwen3-tts-1.7b-custom", "Qwen3-TTS 1.7B CustomVoice", "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice", common_tts),
    ("qwen3-tts-1.7b-base", "Qwen3-TTS 1.7B Base", "Qwen/Qwen3-TTS-12Hz-1.7B-Base", clone_tts),
    ("qwen3-tts-1.7b-voice-design", "Qwen3-TTS 1.7B VoiceDesign", "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign", common_tts),
]:
    write(f"registry/models/{model_id}.toml", snapshot_manifest(model_id, name, repo, "voice-cloning" if caps["voice_cloning"] else "tts", "qwen3_tts", caps, "apache-2.0", "16gb", "Official Qwen3-TTS checkpoint with a Takokit-managed local snapshot."))

# Preserve qwen3-tts as the short alias while fixing its overstated capabilities.
qwen_alias = read("registry/models/qwen3-tts.toml")
qwen_alias = qwen_alias.replace("voice_cloning = true", "voice_cloning = false")
qwen_alias = qwen_alias.replace("live_audio = true", "live_audio = false")
write("registry/models/qwen3-tts.toml", qwen_alias)

piper = read("registry/models/piper-lessac.toml")
piper = piper.replace('backend = "onnx"', 'backend = "python-managed"')
piper = piper.replace('runner = "takokit-onnx"', 'runner = "takokit-python-managed"\nrequired_adapter = "piper_tts"')
piper = piper.replace("live_audio = true", "live_audio = false")
write("registry/models/piper-lessac.toml", piper)

write("registry/models/cosyvoice2.toml", snapshot_manifest("cosyvoice2", "CosyVoice2 0.5B", "FunAudioLLM/CosyVoice2-0.5B", "voice-cloning", "cosyvoice2", clone_tts, "apache-2.0", "16gb", "Official CosyVoice2 zero-shot multilingual TTS snapshot."))
write("registry/models/fish-speech.toml", snapshot_manifest("fish-speech", "Fish Speech S2 Pro", "fishaudio/s2-pro", "voice-cloning", "fish_speech", clone_tts, "fish-audio-research-license", "32gb", "Official Fish Audio S2 Pro local inference snapshot; 24GB VRAM recommended."))
write("registry/models/openvoice.toml", snapshot_manifest("openvoice", "OpenVoice V2", "myshell-ai/OpenVoiceV2", "voice-cloning", "openvoice", clone_tts, "mit", "12gb", "OpenVoice V2 local tone-colour cloning with MeloTTS base speech."))
write("registry/models/gpt-sovits.toml", snapshot_manifest("gpt-sovits", "GPT-SoVITS", "lj1995/GPT-SoVITS", "voice-cloning", "gpt_sovits", clone_tts, "mit", "16gb", "GPT-SoVITS pretrained local zero-shot TTS and cloning assets."))
write("registry/models/qwen2-5-omni.toml", snapshot_manifest("qwen2-5-omni", "Qwen2.5 Omni 7B", "Qwen/Qwen2.5-Omni-7B", "omni-audio", "qwen25_omni", omni_stt, "apache-2.0", "32gb", "Qwen2.5-Omni local audio transcription; high-memory GPU recommended."))
write("registry/models/qwen3-omni.toml", snapshot_manifest("qwen3-omni", "Qwen3 Omni 30B A3B", "Qwen/Qwen3-Omni-30B-A3B-Instruct", "omni-audio", "qwen3_omni", omni_stt, "apache-2.0", "64gb", "Qwen3-Omni local audio transcription; multi-GPU class hardware recommended."))

# RVC installs the official runtime but needs a user-supplied consent-backed voice model.
rvc = read("registry/models/rvc.toml")
rvc = rvc.replace('backend = "python-managed"', 'backend = "python-managed"')
if 'required_adapter = "rvc"' not in rvc:
    rvc = rvc.replace('runner = "takokit-python-managed"', 'runner = "takokit-python-managed"\nrequired_adapter = "rvc"')
rvc = rvc.replace("metadata_only = true", "metadata_only = false")
rvc = rvc.replace("weights = []", '''[[artifacts.configs]]
name = "runtime-ready.marker"
url = "https://raw.githubusercontent.com/RVC-Project/Retrieval-based-Voice-Conversion/main/README.md"
sha256 = "todo"
role = "other"''')
# Leave RVC metadata-only until a pinned upstream marker checksum is available; the adapter remains installable.
rvc = rvc.replace("metadata_only = false", "metadata_only = true")
write("registry/models/rvc.toml", rvc)

# Expand catalog count invariant for the five official Qwen3-TTS checkpoints.
invariants = read("crates/takokit-package/src/tests/catalog_invariants.rs")
invariants = invariants.replace("(20..=30).contains(&models.len())", "(27..=35).contains(&models.len())")
write("crates/takokit-package/src/tests/catalog_invariants.rs", invariants)

catalog_tests = read("crates/takokit-package/src/tests/catalog.rs")
catalog_tests = catalog_tests.replace('"qwen3-tts",\n        "cosyvoice2",', '"qwen3-tts",\n        "qwen3-tts-0.6b-base",\n        "qwen3-tts-1.7b-voice-design",\n        "cosyvoice2",')
write("crates/takokit-package/src/tests/catalog.rs", catalog_tests)

print("runtime completion patch applied")
