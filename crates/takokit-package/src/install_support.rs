//! Transactional install-record construction and filesystem persistence helpers.

use crate::*;
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ModelInstallJournal {
    schema_version: u32,
    manifest_existed: bool,
    record_existed: bool,
}

const MODEL_INSTALL_JOURNAL_SCHEMA: u32 = 1;

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
    ensure_parent(manifest_path)?;
    ensure_parent(record_path)?;
    let lock = open_model_install_lock(manifest_path)?;
    lock.lock_exclusive()?;
    recover_model_install_files_locked(manifest_path, record_path)?;

    let manifest_new = transaction_sidecar(manifest_path, "txn-new");
    let record_new = transaction_sidecar(record_path, "txn-new");
    let manifest_backup = transaction_sidecar(manifest_path, "txn-bak");
    let record_backup = transaction_sidecar(record_path, "txn-bak");
    let journal_path = transaction_sidecar(manifest_path, "install-txn.json");
    let journal_new = transaction_sidecar(manifest_path, "install-txn.new");

    write_synced(&manifest_new, manifest_toml.as_bytes())?;
    if let Err(error) = write_synced(&record_new, record_toml.as_bytes()) {
        let _ = remove_file_if_exists(manifest_new.clone());
        return Err(error);
    }

    let manifest_existed = regular_file_exists(manifest_path)?;
    let record_existed = regular_file_exists(record_path)?;
    if manifest_existed {
        copy_synced(manifest_path, &manifest_backup)?;
    }
    if record_existed {
        copy_synced(record_path, &record_backup)?;
    }

    let journal = ModelInstallJournal {
        schema_version: MODEL_INSTALL_JOURNAL_SCHEMA,
        manifest_existed,
        record_existed,
    };
    write_synced(&journal_new, &serde_json::to_vec_pretty(&journal)?)?;
    std::fs::rename(&journal_new, &journal_path)?;
    sync_parent(&journal_path);

    if let Err(error) = replace_target(manifest_path, &manifest_new) {
        let _ = recover_model_install_files_locked(manifest_path, record_path);
        return Err(error);
    }
    if let Err(error) = replace_target(record_path, &record_new) {
        let _ = recover_model_install_files_locked(manifest_path, record_path);
        return Err(error);
    }
    sync_parent(manifest_path);
    sync_parent(record_path);

    // Removing the journal is the commit point. If the process dies before this,
    // the next reader deterministically restores both previous files. If it dies
    // after this, both new files are already durable and stale backups are harmless.
    remove_file_if_exists(journal_path)?;
    sync_parent(manifest_path);
    let _ = remove_file_if_exists(manifest_backup);
    let _ = remove_file_if_exists(record_backup);
    Ok(())
}

pub(crate) fn recover_model_install_files(
    manifest_path: &Path,
    record_path: &Path,
) -> PackageResult<()> {
    ensure_parent(manifest_path)?;
    ensure_parent(record_path)?;
    let lock = open_model_install_lock(manifest_path)?;
    lock.lock_exclusive()?;
    recover_model_install_files_locked(manifest_path, record_path)
}

fn recover_model_install_files_locked(
    manifest_path: &Path,
    record_path: &Path,
) -> PackageResult<()> {
    let manifest_new = transaction_sidecar(manifest_path, "txn-new");
    let record_new = transaction_sidecar(record_path, "txn-new");
    let manifest_backup = transaction_sidecar(manifest_path, "txn-bak");
    let record_backup = transaction_sidecar(record_path, "txn-bak");
    let journal_path = transaction_sidecar(manifest_path, "install-txn.json");
    let journal_new = transaction_sidecar(manifest_path, "install-txn.new");

    if journal_path.is_file() {
        let source = std::fs::read(&journal_path)?;
        let journal: ModelInstallJournal = serde_json::from_slice(&source)?;
        if journal.schema_version != MODEL_INSTALL_JOURNAL_SCHEMA {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: manifest_path.display().to_string(),
                reason: "unsupported model install transaction journal schema".to_string(),
            });
        }
        restore_transaction_target(manifest_path, &manifest_backup, journal.manifest_existed)?;
        restore_transaction_target(record_path, &record_backup, journal.record_existed)?;
        remove_file_if_exists(journal_path.clone())?;
        sync_parent(&journal_path);
    }

    // Sidecars without a journal are either from a committed transaction or from
    // a crash before the journal became durable. In both cases canonical files are
    // authoritative and these files are safe to discard.
    let _ = remove_file_if_exists(manifest_new);
    let _ = remove_file_if_exists(record_new);
    let _ = remove_file_if_exists(manifest_backup);
    let _ = remove_file_if_exists(record_backup);
    let _ = remove_file_if_exists(journal_new);
    Ok(())
}

fn restore_transaction_target(
    target: &Path,
    backup: &Path,
    previously_existed: bool,
) -> PackageResult<()> {
    if previously_existed {
        if !backup.is_file() {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: target.display().to_string(),
                reason: format!(
                    "model install recovery is missing required backup {}",
                    backup.display()
                ),
            });
        }
        let restore = transaction_sidecar(target, "txn-restore");
        copy_synced(backup, &restore)?;
        replace_target(target, &restore)?;
        sync_parent(target);
    } else {
        remove_file_if_exists(target.to_path_buf())?;
        sync_parent(target);
    }
    Ok(())
}

fn open_model_install_lock(manifest_path: &Path) -> PackageResult<File> {
    let path = transaction_sidecar(manifest_path, "install-txn.lock");
    Ok(OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?)
}

fn transaction_sidecar(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("model");
    path.with_file_name(format!("{name}.{suffix}"))
}

fn write_synced(path: &Path, bytes: &[u8]) -> PackageResult<()> {
    remove_file_if_exists(path.to_path_buf())?;
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn copy_synced(source: &Path, destination: &Path) -> PackageResult<()> {
    let metadata = std::fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: source.display().to_string(),
            reason: "model install metadata must be a regular file".to_string(),
        });
    }
    remove_file_if_exists(destination.to_path_buf())?;
    std::fs::copy(source, destination)?;
    File::open(destination)?.sync_all()?;
    Ok(())
}

fn replace_target(target: &Path, replacement: &Path) -> PackageResult<()> {
    match std::fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
            std::fs::remove_file(target)?;
        }
        Ok(_) => {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: target.display().to_string(),
                reason: "model install metadata path is not a regular file".to_string(),
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    std::fs::rename(replacement, target)?;
    Ok(())
}

fn regular_file_exists(path: &Path) -> PackageResult<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(PackageError::ArtifactInstallFailed {
                artifact: path.display().to_string(),
                reason: "model install metadata path cannot be a symlink".to_string(),
            })
        }
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(PackageError::ArtifactInstallFailed {
            artifact: path.display().to_string(),
            reason: "model install metadata path is not a regular file".to_string(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn sync_parent(_path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = _path.parent() {
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
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

fn ensure_parent(path: &Path) -> PackageResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_install_pair_commits_together() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = temp.path().join("models/fixture.toml");
        let record = temp.path().join("installed-models/fixture.toml");
        write_model_install_files(&manifest, &record, "manifest-v1", "record-v1").unwrap();
        write_model_install_files(&manifest, &record, "manifest-v2", "record-v2").unwrap();
        assert_eq!(std::fs::read_to_string(manifest).unwrap(), "manifest-v2");
        assert_eq!(std::fs::read_to_string(record).unwrap(), "record-v2");
    }

    #[test]
    fn interrupted_pair_update_rolls_back_both_files() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = temp.path().join("models/fixture.toml");
        let record = temp.path().join("installed-models/fixture.toml");
        ensure_parent(&manifest).unwrap();
        ensure_parent(&record).unwrap();
        std::fs::write(&manifest, "manifest-old").unwrap();
        std::fs::write(&record, "record-old").unwrap();

        let manifest_backup = transaction_sidecar(&manifest, "txn-bak");
        let record_backup = transaction_sidecar(&record, "txn-bak");
        copy_synced(&manifest, &manifest_backup).unwrap();
        copy_synced(&record, &record_backup).unwrap();
        let journal = ModelInstallJournal {
            schema_version: MODEL_INSTALL_JOURNAL_SCHEMA,
            manifest_existed: true,
            record_existed: true,
        };
        std::fs::write(
            transaction_sidecar(&manifest, "install-txn.json"),
            serde_json::to_vec_pretty(&journal).unwrap(),
        )
        .unwrap();
        std::fs::write(&manifest, "manifest-new").unwrap();
        std::fs::write(&record, "record-new").unwrap();

        recover_model_install_files(&manifest, &record).unwrap();
        assert_eq!(std::fs::read_to_string(manifest).unwrap(), "manifest-old");
        assert_eq!(std::fs::read_to_string(record).unwrap(), "record-old");
    }

    #[test]
    fn interrupted_first_install_removes_half_committed_pair() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = temp.path().join("models/fixture.toml");
        let record = temp.path().join("installed-models/fixture.toml");
        ensure_parent(&manifest).unwrap();
        ensure_parent(&record).unwrap();
        let journal = ModelInstallJournal {
            schema_version: MODEL_INSTALL_JOURNAL_SCHEMA,
            manifest_existed: false,
            record_existed: false,
        };
        std::fs::write(
            transaction_sidecar(&manifest, "install-txn.json"),
            serde_json::to_vec_pretty(&journal).unwrap(),
        )
        .unwrap();
        std::fs::write(&manifest, "half-new").unwrap();

        recover_model_install_files(&manifest, &record).unwrap();
        assert!(!manifest.exists());
        assert!(!record.exists());
    }
}
