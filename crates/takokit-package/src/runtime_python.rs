//! Managed Python runtime and model-adapter lifecycle.

use crate::{
    runtime_command::{run_logged_command, runner_python_path},
    runtime_uv::bootstrap_uv,
    *,
};
use std::path::Path;
const QWEN3_TTS_PACKAGE: &str = "qwen-tts==0.1.1";
const QWEN3_TTS_ADAPTER: &str = include_str!("../../../runners/python/qwen3_tts_adapter.py");
const PYTHON_MANAGED_ADAPTERS: &[(&str, &str)] = &[
    ("qwen3_tts", "qwen3-tts"),
    ("chatterbox", "chatterbox"),
    ("f5_tts", "f5-tts"),
    ("cosyvoice2", "cosyvoice2"),
    ("dia", "dia"),
    ("fish_speech", "fish-speech"),
    ("openvoice", "openvoice"),
    ("gpt_sovits", "gpt-sovits"),
    ("rvc", "rvc"),
];

pub(crate) fn write_python_adapter_manifests(
    layout: &PythonManagedRunnerLayout,
) -> PackageResult<()> {
    for (adapter, model_family) in PYTHON_MANAGED_ADAPTERS {
        let adapter_dir = layout.adapters.join(adapter);
        std::fs::create_dir_all(&adapter_dir)?;
        let manifest = adapter_dir.join("adapter.toml");
        if !manifest.is_file() {
            write_adapter_record(
                &manifest,
                &AdapterRecord {
                    id: (*adapter).to_string(),
                    model_family: (*model_family).to_string(),
                    state: AdapterLifecycleState::NotInstalled,
                    dependency_strategy: "takokit-managed-python".to_string(),
                    input_contract: "json request on stdin".to_string(),
                    output_contract: "json response on stdout".to_string(),
                    logs: "../../logs".to_string(),
                    notes: "Adapter slot only. Takokit has not installed Python dependencies or model weights for this adapter.".to_string(),
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
                reason: format!("adapter is not available; run `takokit runner install takokit-python-managed`: {}", path.display()),
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
    record.notes =
        "Takokit is installing this adapter in the managed Python environment.".to_string();
    write_adapter_record(&manifest_path, &record)?;

    let result = match adapter {
        "qwen3_tts" => install_qwen3_tts_adapter(&layout),
        _ => Err(PackageError::ArtifactInstallFailed {
            artifact: adapter.to_string(),
            reason: "no executable adapter has been verified for this model family yet; Takokit left the adapter in not-installed state.".to_string(),
        }),
    };
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
    PYTHON_MANAGED_ADAPTERS
        .iter()
        .find(|(_, family)| *family == model_id)
        .map(|(adapter, _)| *adapter)
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
            "3.12".into(),
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
            "Managed Python runtime is ready at {} using {}. No model adapter is installed yet; use `takokit adapter install qwen3_tts`. Runtime log: {}",
            layout.root.display(),
            python.display(),
            log.display()
        ),
    )
}

pub(crate) fn install_qwen3_tts_adapter(
    layout: &PythonManagedRunnerLayout,
) -> PackageResult<String> {
    let venv = layout.env.join("venv");
    let python = runner_python_path(&venv).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: "qwen3_tts adapter".to_string(),
        reason: format!("managed Python runtime is missing; run `takokit runner install takokit-python-managed` first (expected {})", venv.display()),
    })?;
    let adapter_dir = layout.adapters.join("qwen3_tts");
    std::fs::create_dir_all(&adapter_dir)?;
    let log = adapter_dir.join("install.log");
    let takokit_root = layout.root.parent().and_then(Path::parent).ok_or_else(|| {
        PackageError::ArtifactInstallFailed {
            artifact: "qwen3_tts adapter".to_string(),
            reason: "cannot resolve Takokit storage root".to_string(),
        }
    })?;
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "pip".into(),
            "install".into(),
            "--python".into(),
            python.into(),
            "--no-progress".into(),
            QWEN3_TTS_PACKAGE.into(),
            "soundfile".into(),
        ],
    )?;
    std::fs::write(adapter_dir.join("qwen3_tts.py"), QWEN3_TTS_ADAPTER)?;
    Ok(format!(
        "Ready. Takokit installed {QWEN3_TTS_PACKAGE} and the JSON adapter. Model artifacts are pulled separately by `takokit pull qwen3-tts`. Install log: {}",
        log.display()
    ))
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
