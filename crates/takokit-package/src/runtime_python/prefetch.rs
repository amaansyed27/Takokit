//! Checkpoint acquisition for managed Python model adapters.

use crate::{
    artifact_io::sha256_file,
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
    #[serde(default)]
    size_bytes: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ModelPrefetchMarker {
    model_id: String,
    model_version: String,
    adapter: String,
    adapter_script_sha256: String,
    #[serde(default)]
    size_bytes: u64,
}

impl ModelPrefetchMarker {
    fn same_install(&self, other: &Self) -> bool {
        self.model_id == other.model_id
            && self.model_version == other.model_version
            && self.adapter == other.adapter
            && self.adapter_script_sha256 == other.adapter_script_sha256
    }
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
    let layout = python_managed_runner_layout(takokit_root);
    let adapter_dir = layout.adapters.join(adapter);
    let script = adapter_dir.join(format!("{adapter}.py"));
    if !script.is_file() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!("adapter script is missing: {}", script.display()),
        });
    }
    let python = runner_python_path(&adapter_dir.join("venv")).ok_or_else(|| {
        PackageError::ArtifactInstallFailed {
            artifact: model.id.clone(),
            reason: format!(
                "adapter environment has no Python executable below {}",
                adapter_dir.display()
            ),
        }
    })?;
    let expected = ModelPrefetchMarker {
        model_id: model.id.clone(),
        model_version: model.version.clone(),
        adapter: adapter.to_string(),
        adapter_script_sha256: sha256_file(&script)?,
        size_bytes: 0,
    };
    let previous_marker = std::fs::read(&marker_path)
        .ok()
        .and_then(|source| serde_json::from_slice::<ModelPrefetchMarker>(&source).ok());
    let previously_marked = previous_marker
        .as_ref()
        .is_some_and(|marker| marker.same_install(&expected));

    std::fs::create_dir_all(&model_dir)?;

    // A marker is only a record of a previous successful prefetch. It is not
    // proof that an external cache still contains every checkpoint byte. Make
    // the model non-reusable while the adapter revalidates/resumes its cache,
    // then publish a fresh marker atomically after a successful response.
    remove_file_if_exists(&marker_path)?;

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
        .env("HF_HUB_DISABLE_XET", "1")
        .env("HF_HUB_DISABLE_TELEMETRY", "1")
        .env("HF_HUB_DOWNLOAD_TIMEOUT", "60")
        .env("HF_HUB_ETAG_TIMEOUT", "30")
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

    let previous_size_bytes = previous_marker
        .as_ref()
        .filter(|marker| marker.same_install(&expected))
        .map(|marker| marker.size_bytes)
        .filter(|bytes| *bytes > 0);
    let mut completed_marker = expected;
    completed_marker.size_bytes = response
        .size_bytes
        .filter(|bytes| *bytes > 0)
        .or(previous_size_bytes)
        .unwrap_or_default();
    write_marker_atomic(&marker_path, &completed_marker)?;
    Ok(Some(response.detail.unwrap_or_else(|| {
        format!(
            "{} managed checkpoint prefetch; marker: {}",
            if previously_marked {
                "Revalidated"
            } else {
                "Completed"
            },
            marker_path.display()
        )
    })))
}

fn write_marker_atomic(path: &Path, marker: &ModelPrefetchMarker) -> PackageResult<()> {
    let temporary = path.with_extension(format!(
        "json.tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::write(&temporary, serde_json::to_vec_pretty(marker)?)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        PackageError::Io(error)
    })?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> PackageResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PackageError::Io(error)),
    }
}
