//! Transactional install-record construction and filesystem persistence helpers.

use crate::*;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn installed_model_record(
    manifest: &ModelManifest,
    manifest_path: PathBuf,
) -> InstalledModelRecord {
    installed_model_record_with_artifacts(
        manifest,
        manifest_path,
        InstalledArtifactSet {
            records: installed_artifacts(&manifest.artifacts),
            snapshot: None,
            status: InstalledPackageStatus::MetadataOnly,
            note: "Installed model metadata only. No model files were downloaded.".to_string(),
        },
    )
}

pub(crate) fn installed_model_record_with_artifacts(
    manifest: &ModelManifest,
    manifest_path: PathBuf,
    artifacts: InstalledArtifactSet,
) -> InstalledModelRecord {
    InstalledModelRecord {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        source: manifest
            .source
            .as_ref()
            .map(|source| format!("{}@{}", source.repository, source.revision))
            .unwrap_or_else(|| "takokit-registry".to_string()),
        manifest_path,
        runner: manifest.runner.clone(),
        installed_at: timestamp_now(),
        artifacts: artifacts.records,
        snapshot: artifacts.snapshot,
        status: artifacts.status,
        note: artifacts.note,
    }
}

pub(crate) fn installed_runner_record(
    manifest: &RunnerManifest,
    manifest_path: PathBuf,
) -> InstalledRunnerRecord {
    InstalledRunnerRecord {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        kind: manifest.kind.as_str().to_string(),
        manifest_path,
        installed_at: timestamp_now(),
        platforms: manifest.platforms.clone(),
        status: RunnerLifecycleState::ContractInstalled,
        note: runner_contract_note(manifest).to_string(),
    }
}

pub(crate) fn runner_contract_note(manifest: &RunnerManifest) -> &'static str {
    match manifest.kind {
        RunnerKind::Whispercpp => {
            "Installed runner contract. Run `takokit runner install takokit-whispercpp` to install or verify whisper.cpp."
        }
        RunnerKind::Onnx => {
            "Installed runner contract. Run `takokit runner install takokit-onnx` to initialize the ONNX runtime."
        }
        RunnerKind::PythonManaged => {
            "Installed runner contract. Run `takokit runner install takokit-python-managed` to initialize managed Python and adapter slots."
        }
        RunnerKind::TransformersAudio => {
            "Installed Transformers audio runner contract. Managed adapter installation is required."
        }
        RunnerKind::Nemo => {
            "Installed NeMo runner contract. Managed adapter installation is required."
        }
        RunnerKind::Native => {
            "Installed native runner contract. Run runner doctor for current readiness."
        }
        RunnerKind::External => {
            "Installed external runner contract. Run runner doctor for current readiness."
        }
    }
}

pub(crate) fn installed_artifacts(manifest: &ArtifactManifest) -> Vec<InstalledArtifactRecord> {
    manifest
        .all()
        .map(|artifact| InstalledArtifactRecord {
            name: artifact.name.clone(),
            sha256: artifact.sha256.clone(),
            bytes: artifact.bytes,
            url: artifact.url.clone(),
            role: artifact.role,
            local_path: None,
            downloaded: false,
        })
        .collect()
}

pub(crate) fn write_model_install_files(
    manifest_path: &Path,
    record_path: &Path,
    manifest_toml: &str,
    record_toml: &str,
) -> PackageResult<()> {
    let manifest_tmp = sibling_temp_path(manifest_path);
    let record_tmp = sibling_temp_path(record_path);
    std::fs::write(&manifest_tmp, manifest_toml)?;
    std::fs::write(&record_tmp, record_toml).map_err(|error| {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&record_tmp);
        PackageError::Io(error)
    })?;

    let manifest_backup = backup_existing_file(manifest_path).map_err(|error| {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&record_tmp);
        error
    })?;
    if let Err(error) = std::fs::rename(&manifest_tmp, manifest_path) {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&record_tmp);
        restore_backup(manifest_path, manifest_backup);
        return Err(PackageError::Io(error));
    }

    let record_backup = match backup_existing_file(record_path) {
        Ok(backup) => backup,
        Err(error) => {
            let _ = std::fs::remove_file(&record_tmp);
            let _ = std::fs::remove_file(manifest_path);
            restore_backup(manifest_path, manifest_backup);
            return Err(error);
        }
    };
    if let Err(error) = std::fs::rename(&record_tmp, record_path) {
        let _ = std::fs::remove_file(&record_tmp);
        let _ = std::fs::remove_file(manifest_path);
        restore_backup(manifest_path, manifest_backup);
        restore_backup(record_path, record_backup);
        return Err(PackageError::Io(error));
    }

    remove_backup(manifest_backup);
    remove_backup(record_backup);
    Ok(())
}

fn sibling_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("install");
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| timestamp_now());
    path.with_file_name(format!("{file_name}.{suffix}.tmp"))
}

fn backup_existing_file(path: &Path) -> PackageResult<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let backup_path = sibling_temp_path(path).with_extension("bak");
    std::fs::rename(path, &backup_path)?;
    Ok(Some(backup_path))
}

fn restore_backup(path: &Path, backup: Option<PathBuf>) {
    if let Some(backup) = backup {
        let _ = std::fs::rename(backup, path);
    }
}

fn remove_backup(backup: Option<PathBuf>) {
    if let Some(backup) = backup {
        let _ = std::fs::remove_file(backup);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledArtifactSet {
    pub(crate) records: Vec<InstalledArtifactRecord>,
    pub(crate) snapshot: Option<InstalledSnapshotRecord>,
    pub(crate) status: InstalledPackageStatus,
    pub(crate) note: String,
}

pub(crate) fn timestamp_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

pub(crate) fn remove_file_if_exists(path: PathBuf) -> PackageResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PackageError::Io(error)),
    }
}
