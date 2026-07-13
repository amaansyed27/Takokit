//! Managed Python runtime and model-adapter lifecycle.

use crate::{
    runtime_command::{run_logged_command, runner_python_path},
    runtime_uv::bootstrap_uv,
    *,
};
use std::path::{Path, PathBuf};

const QWEN3_TTS_ADAPTER: &str = include_str!("../../../runners/python/qwen3_tts_adapter.py");
const CHATTERBOX_ADAPTER: &str = include_str!("../../../runners/python/chatterbox_adapter.py");
const F5_TTS_ADAPTER: &str = include_str!("../../../runners/python/f5_tts_adapter.py");
const DIA_ADAPTER: &str = include_str!("../../../runners/python/dia_adapter.py");

#[derive(Debug, Clone, Copy)]
struct AdapterSpec {
    id: &'static str,
    model_family: &'static str,
    python: &'static str,
    packages: &'static [&'static str],
    script: Option<&'static str>,
    note: &'static str,
}

const ADAPTER_SPECS: &[AdapterSpec] = &[
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

pub(crate) fn write_python_adapter_manifests(
    layout: &PythonManagedRunnerLayout,
) -> PackageResult<()> {
    for spec in ADAPTER_SPECS {
        let adapter_dir = layout.adapters.join(spec.id);
        std::fs::create_dir_all(&adapter_dir)?;
        let manifest = adapter_dir.join("adapter.toml");
        if !manifest.is_file() {
            write_adapter_record(
                &manifest,
                &AdapterRecord {
                    id: spec.id.to_string(),
                    model_family: spec.model_family.to_string(),
                    state: AdapterLifecycleState::NotInstalled,
                    dependency_strategy: "isolated-takokit-managed-python".to_string(),
                    input_contract: "typed JSON request on stdin".to_string(),
                    output_contract: "typed JSON response on stdout".to_string(),
                    logs: "install.log".to_string(),
                    notes: spec.note.to_string(),
                },
            )?;
        }
    }
    Ok(())
}

pub fn python_adapter_records(takokit_root: &Path) -> PackageResult<Vec<AdapterRecord>> {
    let layout = python_managed_runner_layout(takokit_root);
    let mut records = Vec::new();
    if !layout.adapters.is_dir() {
        return Ok(records);
    }
    let mut entries = std::fs::read_dir(&layout.adapters)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path().join("adapter.toml");
        if path.is_file() {
            records.push(toml::from_str(&std::fs::read_to_string(path)?)?);
        }
    }
    Ok(records)
}

pub fn python_adapter_record(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let path = python_managed_runner_layout(takokit_root)
        .adapters
        .join(adapter)
        .join("adapter.toml");
    std::fs::read_to_string(&path)
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => PackageError::ArtifactInstallFailed {
                artifact: adapter.to_string(),
                reason: format!(
                    "adapter is not available; run `takokit runner install takokit-python-managed`: {}",
                    path.display()
                ),
            },
            _ => PackageError::Io(error),
        })
        .and_then(|source| Ok(toml::from_str(&source)?))
}

pub fn install_python_adapter(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let layout = python_managed_runner_layout(takokit_root);
    write_python_adapter_manifests(&layout)?;
    let manifest_path = layout.adapters.join(adapter).join("adapter.toml");
    let mut record = python_adapter_record(takokit_root, adapter)?;
    record.state = AdapterLifecycleState::Installing;
    record.notes = "Takokit is installing this adapter in an isolated managed environment."
        .to_string();
    write_adapter_record(&manifest_path, &record)?;

    let result = adapter_spec(adapter)
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: adapter.to_string(),
            reason: "unknown managed adapter".to_string(),
        })
        .and_then(|spec| install_adapter_spec(takokit_root, &layout, spec));
    match result {
        Ok(note) => {
            record.state = AdapterLifecycleState::Ready;
            record.notes = note;
            write_adapter_record(&manifest_path, &record)?;
            Ok(record)
        }
        Err(error) => {
            record.state = AdapterLifecycleState::Failed;
            record.notes = format!("Adapter install failed: {error}");
            write_adapter_record(&manifest_path, &record)?;
            Err(error)
        }
    }
}

pub fn adapter_for_model(model_id: &str) -> Option<&'static str> {
    ADAPTER_SPECS
        .iter()
        .find(|spec| spec.model_family == model_id)
        .map(|spec| spec.id)
}

pub(crate) fn install_python_managed_runtime(
    takokit_root: &Path,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) -> PackageResult<PullReport> {
    let layout = python_managed_runner_layout(takokit_root);
    for path in [
        &layout.root,
        &layout.runtime,
        &layout.env,
        &layout.packages,
        &layout.wheels,
        &layout.logs,
        &layout.manifests,
        &layout.cache,
        &layout.adapters,
    ] {
        std::fs::create_dir_all(path)?;
    }
    write_python_adapter_manifests(&layout)?;
    let venv = layout.env.join("venv");
    let log = layout.logs.join("runtime-install.log");
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            "3.11".into(),
            "--allow-existing".into(),
            venv.clone().into(),
        ],
    )?;
    let python = runner_python_path(&venv).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: "managed Python runtime".to_string(),
        reason: format!(
            "uv created no Python executable below {}; see {}",
            venv.display(),
            log.display()
        ),
    })?;
    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "Managed Python runtime is ready at {} using {}. Install model adapters independently with `takokit adapter install <id>`. Runtime log: {}",
            layout.root.display(),
            python.display(),
            log.display()
        ),
    )
}

fn install_adapter_spec(
    takokit_root: &Path,
    layout: &PythonManagedRunnerLayout,
    spec: &AdapterSpec,
) -> PackageResult<String> {
    let script = spec.script.ok_or_else(|| PackageError::ArtifactInstallFailed {
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
    }
    let adapter_dir = layout.adapters.join(spec.id);
    std::fs::create_dir_all(&adapter_dir)?;
    let venv = adapter_dir.join("venv");
    let log = adapter_dir.join("install.log");
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            spec.python.into(),
            "--allow-existing".into(),
            venv.clone().into(),
        ],
    )?;
    let python = runner_python_path(&venv).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: spec.id.to_string(),
        reason: format!("adapter environment has no Python executable: {}", venv.display()),
    })?;
    let mut arguments = vec![
        "pip".into(),
        "install".into(),
        "--python".into(),
        python.into(),
        "--no-progress".into(),
    ];
    arguments.extend(spec.packages.iter().map(PathBuf::from));
    run_logged_command(&log, &uv, &arguments)?;
    std::fs::write(adapter_dir.join(format!("{}.py", spec.id)), script)?;
    Ok(format!(
        "Ready. {} Environment: {}. Install log: {}",
        spec.note,
        venv.display(),
        log.display()
    ))
}

fn adapter_spec(id: &str) -> Option<&'static AdapterSpec> {
    ADAPTER_SPECS.iter().find(|spec| spec.id == id)
}

pub(crate) fn write_adapter_record(path: &Path, record: &AdapterRecord) -> PackageResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: record.id.clone(),
            reason: "adapter manifest path has no parent directory".to_string(),
        })?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, toml::to_string_pretty(record)?)?;
    Ok(())
}
