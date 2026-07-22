//! Checkpoint acquisition for managed Python model adapters.

use crate::{
    runtime_command::{configure_managed_command, runner_python_path},
    runtime_python_specs::model_prefetch_required,
    *,
};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Debug, Serialize)]
struct ModelPrefetchRequest<'a> {
    operation: &'static str,
    model_id: &'a str,
    model_dir: &'a Path,
    cache_dir: &'a Path,
}

#[derive(Debug, Deserialize)]
struct ModelPrefetchResponse {
    ok: bool,
    detail: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelPrefetchMarker {
    model_id: String,
    model_version: String,
    adapter: String,
}

pub(crate) fn prefetch_python_adapter_model(
    takokit_root: &Path,
    model: &ModelManifest,
    adapter: &str,
) -> PackageResult<Option<String>> {
    if !model_prefetch_required(&model.id) {
        return Ok(None);
    }

    let model_dir = takokit_root.join("models").join(&model.id);
    let marker_path = model_dir.join(".takokit-prefetch.json");
    let expected = ModelPrefetchMarker {
        model_id: model.id.clone(),
        model_version: model.version.clone(),
        adapter: adapter.to_string(),
    };
    if std::fs::read(&marker_path)
        .ok()
        .and_then(|source| serde_json::from_slice::<ModelPrefetchMarker>(&source).ok())
        .is_some_and(|marker| {
            marker.model_id == expected.model_id
                && marker.model_version == expected.model_version
                && marker.adapter == expected.adapter
        })
    {
        return Ok(Some(format!(
            "Verified managed checkpoint prefetch marker at {}.",
            marker_path.display()
        )));
    }

    let layout = python_managed_runner_layout(takokit_root);
    let adapter_dir = layout.adapters.join(adapter);
    let script = adapter_dir.join(format!("{adapter}.py"));
    let python = runner_python_path(&adapter_dir.join("venv")).ok_or_else(|| {
        PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!(
                "adapter environment has no Python executable below {}",
                adapter_dir.display()
            ),
        }
    })?;
    if !script.is_file() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!("adapter script is missing: {}", script.display()),
        });
    }

    std::fs::create_dir_all(&model_dir)?;
    let cache_dir = takokit_root.join("cache");
    let hf_cache = cache_dir.join("huggingface");
    let torch_cache = cache_dir.join("torch");
    let tts_cache = cache_dir.join("coqui");
    let modelscope_cache = cache_dir.join("modelscope");
    for path in [
        &cache_dir,
        &hf_cache,
        &torch_cache,
        &tts_cache,
        &modelscope_cache,
    ] {
        std::fs::create_dir_all(path)?;
    }

    let payload = serde_json::to_vec(&ModelPrefetchRequest {
        operation: "prefetch",
        model_id: &model.id,
        model_dir: &model_dir,
        cache_dir: &cache_dir,
    })?;
    let log_path = adapter_dir.join(format!("prefetch-{}.log", model.id));
    let mut command = Command::new(&python);
    command
        .arg(&script)
        .env("HF_HOME", &hf_cache)
        .env("TORCH_HOME", &torch_cache)
        .env("TTS_HOME", &tts_cache)
        .env("MODELSCOPE_CACHE", &modelscope_cache)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(std::fs::File::create(&log_path)?));
    configure_managed_command(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!(
                "could not start {adapter} checkpoint prefetch: {error}; see {}",
                log_path.display()
            ),
        })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!("{adapter} prefetch stdin was unavailable"),
        })?
        .write_all(&payload)?;
    let output = child
        .wait_with_output()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!(
                "could not wait for {adapter} checkpoint prefetch: {error}; see {}",
                log_path.display()
            ),
        })?;

    let mut log = std::fs::OpenOptions::new().append(true).open(&log_path)?;
    if !output.stdout.is_empty() {
        if !output.stdout.ends_with(b"\n") {
            log.write_all(b"\n")?;
        }
        log.write_all(&output.stdout)?;
    }

    let response = String::from_utf8_lossy(&output.stdout)
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<ModelPrefetchResponse>(line.trim()).ok())
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!(
                "{adapter} prefetch returned no valid JSON response; see {}",
                log_path.display()
            ),
        })?;
    if !output.status.success() || !response.ok {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!(
                "{adapter} checkpoint prefetch failed: {}; see {}",
                response
                    .error
                    .unwrap_or_else(|| format!("process exited with {}", output.status)),
                log_path.display()
            ),
        });
    }

    std::fs::write(&marker_path, serde_json::to_vec_pretty(&expected)?)?;
    Ok(Some(response.detail.unwrap_or_else(|| {
        format!(
            "Managed checkpoint prefetch completed; marker: {}",
            marker_path.display()
        )
    })))
}
