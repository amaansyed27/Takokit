//! Crash-safe persistence and recovery for managed-Python adapter records.

use crate::{
    runtime_python_specs::AdapterSpec, AdapterLifecycleState, AdapterRecord, PackageError,
    PackageResult,
};
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

const MANIFEST_FILE: &str = "adapter.toml";
const PREVIOUS_FILE: &str = "adapter.toml.previous";
const CORRUPT_FILE: &str = "adapter.toml.corrupt";
const INSTALL_LOCK_FILE: &str = ".install.lock";
const INSTALL_LOCK_WAIT: Duration = Duration::from_secs(15 * 60);
const INSTALL_LOCK_RETRY: Duration = Duration::from_millis(250);
static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct AdapterInstallLock {
    file: File,
}

impl Drop for AdapterInstallLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

pub(super) fn lock_adapter_install(
    adapter_dir: &Path,
    adapter: &str,
) -> PackageResult<AdapterInstallLock> {
    lock_adapter_install_with_timeout(adapter_dir, adapter, INSTALL_LOCK_WAIT)
}

fn lock_adapter_install_with_timeout(
    adapter_dir: &Path,
    adapter: &str,
    timeout: Duration,
) -> PackageResult<AdapterInstallLock> {
    std::fs::create_dir_all(adapter_dir)?;
    let path = adapter_dir.join(INSTALL_LOCK_FILE);
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)?;
    let started = Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(AdapterInstallLock { file }),
            Err(error) if lock_is_contended(&error) && started.elapsed() < timeout => {
                thread::sleep(INSTALL_LOCK_RETRY.min(timeout.saturating_sub(started.elapsed())));
            }
            Err(error) if lock_is_contended(&error) => {
                return Err(PackageError::ArtifactInstallFailed {
                    artifact: adapter.to_string(),
                    reason: format!(
                        "timed out after {} seconds waiting for another Takokit process to finish installing this adapter ({error})",
                        timeout.as_secs()
                    ),
                });
            }
            Err(error) => return Err(PackageError::Io(error)),
        }
    }
}

fn lock_is_contended(error: &std::io::Error) -> bool {
    error.kind() == ErrorKind::WouldBlock || matches!(error.raw_os_error(), Some(11 | 33 | 35 | 36))
}

pub(super) fn default_adapter_record(spec: &AdapterSpec) -> AdapterRecord {
    AdapterRecord {
        id: spec.id.to_string(),
        model_family: spec.model_family.to_string(),
        state: AdapterLifecycleState::NotInstalled,
        dependency_strategy: "shared-takokit-python-base-with-isolated-overlay".to_string(),
        input_contract: "typed JSON request on stdin".to_string(),
        output_contract: "typed JSON response on stdout".to_string(),
        logs: "install.log".to_string(),
        notes: spec.note.to_string(),
    }
}

pub(super) fn ensure_adapter_manifest(path: &Path, spec: &AdapterSpec) -> PackageResult<()> {
    if path.is_file() || previous_path(path).is_file() {
        read_adapter_record(path, spec).map(|_| ())
    } else {
        write_adapter_record(path, &default_adapter_record(spec))
    }
}

pub(super) fn read_adapter_record(path: &Path, spec: &AdapterSpec) -> PackageResult<AdapterRecord> {
    match read_manifest(path) {
        Ok(record) => Ok(record),
        Err(ManifestReadError::Missing) => restore_previous_or_report_missing(path, spec),
        Err(ManifestReadError::Invalid) => recover_invalid_manifest(path, spec),
        Err(ManifestReadError::Io(error)) => Err(PackageError::Io(error)),
    }
}

fn recover_invalid_manifest(path: &Path, spec: &AdapterSpec) -> PackageResult<AdapterRecord> {
    quarantine(path, corrupt_path(path))?;
    if let Some(record) = restore_previous(path)? {
        return Ok(record);
    }

    let mut record = default_adapter_record(spec);
    record.state = AdapterLifecycleState::Failed;
    record.notes = format!(
        "Takokit recovered an invalid adapter manifest. The adapter environment will be rebuilt; the original record is preserved at {}.",
        corrupt_path(path).display()
    );
    write_adapter_record(path, &record)?;
    Ok(record)
}

fn restore_previous_or_report_missing(
    path: &Path,
    spec: &AdapterSpec,
) -> PackageResult<AdapterRecord> {
    if let Some(record) = restore_previous(path)? {
        return Ok(record);
    }
    Err(PackageError::ArtifactInstallFailed {
        artifact: spec.id.to_string(),
        reason: format!(
            "adapter is not available; run `takokit runner install takokit-python-managed`: {}",
            path.display()
        ),
    })
}

fn restore_previous(path: &Path) -> PackageResult<Option<AdapterRecord>> {
    let previous = previous_path(path);
    match read_manifest(&previous) {
        Ok(record) => {
            if path.exists() {
                std::fs::remove_file(path)?;
            }
            std::fs::rename(&previous, path)?;
            Ok(Some(record))
        }
        Err(ManifestReadError::Missing) => Ok(None),
        Err(ManifestReadError::Invalid) => {
            quarantine(&previous, previous_corrupt_path(path))?;
            Ok(None)
        }
        Err(ManifestReadError::Io(error)) => Err(PackageError::Io(error)),
    }
}

fn quarantine(source: &Path, destination: PathBuf) -> PackageResult<()> {
    if !source.exists() {
        return Ok(());
    }
    if destination.exists() {
        std::fs::remove_file(&destination)?;
    }
    std::fs::rename(source, destination)?;
    Ok(())
}

enum ManifestReadError {
    Missing,
    Invalid,
    Io(std::io::Error),
}

fn read_manifest(path: &Path) -> Result<AdapterRecord, ManifestReadError> {
    let source = std::fs::read_to_string(path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            ManifestReadError::Missing
        } else {
            ManifestReadError::Io(error)
        }
    })?;
    toml::from_str(&source).map_err(|_| ManifestReadError::Invalid)
}

pub(super) fn write_adapter_record(path: &Path, record: &AdapterRecord) -> PackageResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: record.id.clone(),
            reason: "adapter manifest path has no parent directory".to_string(),
        })?;
    std::fs::create_dir_all(parent)?;
    let serialized = toml::to_string_pretty(record)?;
    let temporary = temporary_path(path);
    let previous = previous_path(path);

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(serialized.as_bytes())?;
    file.sync_all()?;
    drop(file);

    if previous.exists() {
        std::fs::remove_file(&previous)?;
    }
    let replaced_existing = path.exists();
    if replaced_existing {
        std::fs::rename(path, &previous)?;
    }

    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        if replaced_existing {
            let _ = std::fs::rename(&previous, path);
        }
        return Err(PackageError::Io(error));
    }
    if replaced_existing {
        std::fs::remove_file(previous)?;
    }
    Ok(())
}

fn previous_path(path: &Path) -> PathBuf {
    path.with_file_name(PREVIOUS_FILE)
}

fn corrupt_path(path: &Path) -> PathBuf {
    path.with_file_name(CORRUPT_FILE)
}

fn previous_corrupt_path(path: &Path) -> PathBuf {
    path.with_file_name("adapter.toml.previous.corrupt")
}

fn temporary_path(path: &Path) -> PathBuf {
    let sequence = WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(
        ".{MANIFEST_FILE}.tmp-{}-{sequence}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_python_specs::adapter_spec;

    #[test]
    fn concurrent_installer_waits_for_owner_and_then_acquires_lock() {
        let root = tempfile::tempdir().expect("tempdir");
        let adapter_dir = root.path().join("rvc_training");
        std::fs::create_dir_all(&adapter_dir).expect("adapter dir");
        let lock_path = adapter_dir.join(INSTALL_LOCK_FILE);
        let owner = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(lock_path)
            .expect("owner lock file");
        owner.lock_exclusive().expect("owner lock");
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            FileExt::unlock(&owner).expect("release owner lock");
        });

        let started = Instant::now();
        let waiting =
            lock_adapter_install_with_timeout(&adapter_dir, "rvc_training", Duration::from_secs(2))
                .expect("waiting installer acquires released lock");

        assert!(started.elapsed() >= Duration::from_millis(250));
        drop(waiting);
        release.join().expect("release thread");
    }

    #[test]
    fn invalid_manifest_is_quarantined_and_reset_for_reinstall() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join(MANIFEST_FILE);
        std::fs::write(
            &path,
            "id = 'nemo_asr'\non-managed\\adapters\\nemo_asr\\venv install finished",
        )
        .expect("malformed record");

        let spec = adapter_spec("nemo_asr").expect("NeMo adapter spec");
        let record = read_adapter_record(&path, spec).expect("recovered record");

        assert_eq!(record.state, AdapterLifecycleState::Failed);
        assert!(corrupt_path(&path).is_file());
        let persisted: AdapterRecord =
            toml::from_str(&std::fs::read_to_string(&path).expect("recovered manifest contents"))
                .expect("valid recovered TOML");
        assert_eq!(persisted, record);
    }

    #[test]
    fn interrupted_manifest_swap_restores_previous_record() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join(MANIFEST_FILE);
        let spec = adapter_spec("nemo_asr").expect("NeMo adapter spec");
        let mut record = default_adapter_record(spec);
        record.state = AdapterLifecycleState::Installing;
        write_adapter_record(&path, &record).expect("initial record");
        std::fs::rename(&path, previous_path(&path)).expect("simulate interrupted swap");

        let restored = read_adapter_record(&path, spec).expect("restored record");

        assert_eq!(restored, record);
        assert!(path.is_file());
        assert!(!previous_path(&path).exists());
    }

    #[test]
    fn manifest_writer_roundtrips_windows_paths() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join(MANIFEST_FILE);
        let spec = adapter_spec("nemo_asr").expect("NeMo adapter spec");
        let mut record = default_adapter_record(spec);
        record.state = AdapterLifecycleState::Ready;
        record.notes = "Ready at C:\\Users\\Amaan\\.takokit\\runners; it's shared.".into();

        write_adapter_record(&path, &record).expect("write record");
        write_adapter_record(&path, &record).expect("replace record");

        let persisted: AdapterRecord =
            toml::from_str(&std::fs::read_to_string(path).expect("manifest contents"))
                .expect("valid TOML");
        assert_eq!(persisted, record);
    }
}
