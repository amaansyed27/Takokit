//! Transactional installation of repository-backed model snapshots.

use crate::{
    runtime_command::{run_logged_command, PathOrArg},
    runtime_uv::bootstrap_uv,
    *,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const READY_MARKER: &str = ".takokit-source.json";

#[derive(Debug, Serialize, Deserialize)]
struct SourceMarker {
    provider: ModelSourceProvider,
    repository: String,
    revision: String,
}

pub(crate) fn install_model_source(
    takokit_root: &Path,
    manifest: &ModelManifest,
    previous: Option<&InstalledModelRecord>,
) -> PackageResult<Option<InstalledSnapshotRecord>> {
    let Some(source) = manifest.source.as_ref() else {
        return Ok(None);
    };
    match source.provider {
        ModelSourceProvider::HuggingFace => {
            install_hugging_face_snapshot(takokit_root, manifest, source, previous).map(Some)
        }
    }
}

pub(crate) fn snapshot_is_ready(
    record: &InstalledSnapshotRecord,
    source: &ModelSourceManifest,
) -> bool {
    if record.provider != source.provider
        || record.repository != source.repository
        || record.revision != source.revision
        || !record.local_path.is_dir()
        || !record.ready_marker.is_file()
    {
        return false;
    }
    read_marker(&record.ready_marker).is_some_and(|marker| {
        marker.provider == source.provider
            && marker.repository == source.repository
            && marker.revision == source.revision
    })
}

fn install_hugging_face_snapshot(
    takokit_root: &Path,
    manifest: &ModelManifest,
    source: &ModelSourceManifest,
    previous: Option<&InstalledModelRecord>,
) -> PackageResult<InstalledSnapshotRecord> {
    if let Some(record) = previous.and_then(|record| record.snapshot.as_ref()) {
        if snapshot_is_ready(record, source) {
            return Ok(record.clone());
        }
    }

    let models_root = takokit_root.join("models");
    let cache_root = takokit_root.join("cache").join("huggingface");
    let logs_root = takokit_root.join("logs");
    std::fs::create_dir_all(&models_root)?;
    std::fs::create_dir_all(&cache_root)?;
    std::fs::create_dir_all(&logs_root)?;

    let destination = models_root.join(&manifest.id);
    let temporary = models_root.join(format!(".{}.download-{}", manifest.id, timestamp_suffix()));
    let backup = models_root.join(format!(".{}.backup", manifest.id));
    remove_dir_if_exists(&temporary)?;
    remove_dir_if_exists(&backup)?;
    std::fs::create_dir_all(&temporary)?;

    let uv = bootstrap_uv(takokit_root)?;
    let log = logs_root.join(format!("model-{}-download.log", manifest.id));
    let mut arguments: Vec<PathOrArg> = vec![
        "tool".into(),
        "run".into(),
        "--from".into(),
        "huggingface_hub".into(),
        "hf".into(),
        "download".into(),
        source.repository.clone().into(),
        "--revision".into(),
        source.revision.clone().into(),
        "--local-dir".into(),
        temporary.clone().into(),
        "--cache-dir".into(),
        cache_root.into(),
    ];
    for pattern in &source.allow_patterns {
        arguments.push("--include".into());
        arguments.push(pattern.clone().into());
    }
    for pattern in &source.ignore_patterns {
        arguments.push("--exclude".into());
        arguments.push(pattern.clone().into());
    }

    if let Err(error) = run_logged_command(&log, &uv, &arguments) {
        let _ = remove_dir_if_exists(&temporary);
        return Err(PackageError::ArtifactInstallFailed {
            artifact: manifest.id.clone(),
            reason: format!(
                "Hugging Face snapshot download failed for {}@{}: {error}; see {}",
                source.repository,
                source.revision,
                log.display()
            ),
        });
    }

    let marker = temporary.join(READY_MARKER);
    std::fs::write(
        &marker,
        serde_json::to_vec_pretty(&SourceMarker {
            provider: source.provider,
            repository: source.repository.clone(),
            revision: source.revision.clone(),
        })?,
    )?;

    if destination.exists() {
        std::fs::rename(&destination, &backup)?;
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        let _ = remove_dir_if_exists(&temporary);
        if backup.exists() {
            let _ = std::fs::rename(&backup, &destination);
        }
        return Err(PackageError::Io(error));
    }
    remove_dir_if_exists(&backup)?;

    Ok(InstalledSnapshotRecord {
        provider: source.provider,
        repository: source.repository.clone(),
        revision: source.revision.clone(),
        local_path: destination.clone(),
        ready_marker: destination.join(READY_MARKER),
    })
}

fn read_marker(path: &Path) -> Option<SourceMarker> {
    let source = std::fs::read(path).ok()?;
    serde_json::from_slice(&source).ok()
}

fn remove_dir_if_exists(path: &Path) -> PackageResult<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PackageError::Io(error)),
    }
}

fn timestamp_suffix() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_readiness_requires_matching_marker() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join(READY_MARKER);
        let source = ModelSourceManifest {
            provider: ModelSourceProvider::HuggingFace,
            repository: "owner/model".into(),
            revision: "0123456789abcdef".into(),
            allow_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
        };
        std::fs::write(
            &marker,
            serde_json::to_vec(&SourceMarker {
                provider: source.provider,
                repository: source.repository.clone(),
                revision: source.revision.clone(),
            })
            .expect("marker"),
        )
        .expect("write marker");
        let record = InstalledSnapshotRecord {
            provider: source.provider,
            repository: source.repository.clone(),
            revision: source.revision.clone(),
            local_path: temp.path().to_path_buf(),
            ready_marker: marker,
        };
        assert!(snapshot_is_ready(&record, &source));
    }
}
