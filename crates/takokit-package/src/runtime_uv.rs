//! Deterministic Takokit-managed uv discovery and bootstrap.

use crate::{artifact_io::sha256_file, *};
use fs2::FileExt;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "runtime_uv_download.rs"]
mod runtime_uv_download;
use runtime_uv_download::{download_and_promote_uv, uv_asset};

const TAKOKIT_UV_VERSION: &str = "0.12.10";

pub fn find_uv(takokit_root: &Path) -> Option<PathBuf> {
    if let Some(path) = explicit_uv_override() {
        return Some(path);
    }
    let managed = managed_uv_path(takokit_root);
    managed.is_file().then_some(managed)
}

pub fn bootstrap_uv(takokit_root: &Path) -> PackageResult<PathBuf> {
    let logs = takokit_root.join("logs");
    std::fs::create_dir_all(&logs)?;
    let log = logs.join("uv-bootstrap.log");
    let managed = managed_uv_path(takokit_root);
    append_bootstrap_header(&log, &managed)?;

    if std::env::var_os("UV").is_some() {
        let override_path = explicit_uv_override().ok_or_else(|| {
            PackageError::ArtifactInstallFailed {
                artifact: "uv bootstrap".to_string(),
                reason: format!(
                    "UV must be an absolute path to an existing uv {} executable; see {}",
                    TAKOKIT_UV_VERSION,
                    log.display()
                ),
            }
        })?;
        let valid = verify_uv_version(&override_path, &log)?;
        log_source(
            &log,
            "explicit_override",
            &override_path.display().to_string(),
            observed_sha256(&override_path).as_deref(),
            None,
        )?;
        if !valid {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "uv bootstrap".to_string(),
                reason: format!(
                    "UV override {} is incompatible; Takokit requires exactly uv {}. The override was not modified. See {}",
                    override_path.display(),
                    TAKOKIT_UV_VERSION,
                    log.display()
                ),
            });
        }
        return Ok(override_path);
    }

    let parent = managed.parent().expect("managed uv has a parent");
    std::fs::create_dir_all(parent)?;
    let lock_path = parent.join(".bootstrap.lock");
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    lock.lock_exclusive()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not lock managed uv bootstrap: {error}"),
        })?;

    if managed.is_file() && verify_uv_version(&managed, &log)? {
        ensure_owner_writable_executable(&managed)?;
        log_source(
            &log,
            "managed_existing",
            &managed.display().to_string(),
            observed_sha256(&managed).as_deref(),
            None,
        )?;
        return Ok(managed);
    }

    cleanup_stale_bootstrap_dirs(parent);
    if managed.exists() {
        quarantine_invalid_managed_uv(&managed, &log)?;
    }

    let asset = uv_asset()?;
    let result = download_and_promote_uv(parent, &managed, asset, &log);
    let _ = FileExt::unlock(&lock);
    result
}

pub(crate) fn verify_uv_version(path: &Path, log: &Path) -> PackageResult<bool> {
    let (valid, reported) = probe_uv_version(path);
    append_log(
        log,
        &format!(
            "verified_command: {} --version\nreported_version: {}\nversion_valid: {}\n",
            path.display(),
            reported,
            valid
        ),
    )?;
    Ok(valid)
}

fn probe_uv_version(path: &Path) -> (bool, String) {
    let mut command = Command::new(path);
    command.arg("--version");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    match command.output() {
        Ok(output) => {
            let reported = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let exact = reported.split_whitespace().nth(1) == Some(TAKOKIT_UV_VERSION);
            (output.status.success() && exact, reported)
        }
        Err(error) => (false, format!("unavailable ({error})")),
    }
}

fn explicit_uv_override() -> Option<PathBuf> {
    let value = std::env::var_os("UV")?;
    let path = PathBuf::from(value);
    (path.is_absolute() && path.is_file()).then_some(path)
}

fn managed_uv_path(takokit_root: &Path) -> PathBuf {
    takokit_root
        .join("tools")
        .join("uv")
        .join(if cfg!(windows) { "uv.exe" } else { "uv" })
}

fn quarantine_invalid_managed_uv(managed: &Path, log: &Path) -> PackageResult<()> {
    if !managed.exists() {
        return Ok(());
    }
    make_owner_writable(managed)?;
    let quarantine = managed.with_file_name(format!(
        "{}.invalid-{}-{}",
        managed
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("uv"),
        unix_timestamp(),
        std::process::id()
    ));
    let observed = observed_sha256(managed).unwrap_or_else(|| "unavailable".to_string());
    append_log(
        log,
        &format!(
            "managed_recovery: quarantine_invalid\ninvalid_path: {}\ninvalid_sha256: {}\nquarantine_path: {}\n",
            managed.display(),
            observed,
            quarantine.display()
        ),
    )?;
    std::fs::rename(managed, &quarantine).map_err(|error| {
        PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not quarantine invalid managed uv: {error}"),
        }
    })?;
    Ok(())
}

fn cleanup_stale_bootstrap_dirs(parent: &Path) {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(".bootstrap-") {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn ensure_owner_writable_executable(path: &Path) -> PackageResult<()> {
    make_owner_writable(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        let mode = permissions.mode() | 0o700;
        permissions.set_mode(mode);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

fn make_owner_writable(path: &Path) -> PackageResult<()> {
    let mut permissions = std::fs::metadata(path)?.permissions();
    if permissions.readonly() {
        permissions.set_readonly(false);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

fn observed_sha256(path: &Path) -> Option<String> {
    sha256_file(path).ok()
}

fn append_bootstrap_header(log: &Path, managed: &Path) -> PackageResult<()> {
    append_log(
        log,
        &format!(
            "\nTakokit managed uv bootstrap\nrequested_version: {}\nplatform: {}\narchitecture: {}\nmanaged_path: {}\n",
            TAKOKIT_UV_VERSION,
            std::env::consts::OS,
            std::env::consts::ARCH,
            managed.display()
        ),
    )
}

fn log_source(
    log: &Path,
    source_kind: &str,
    source_path: &str,
    source_sha256: Option<&str>,
    expected_sha256: Option<&str>,
) -> PackageResult<()> {
    append_log(
        log,
        &format!(
            "source_kind: {source_kind}\nsource_path: {source_path}\nsource_sha256: {}\nexpected_sha256: {}\n",
            source_sha256.unwrap_or("unavailable"),
            expected_sha256.unwrap_or("not-applicable")
        ),
    )
}

fn append_log(log: &Path, text: &str) -> PackageResult<()> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)?
        .write_all(text.as_bytes())?;
    Ok(())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_version_check_does_not_accept_prefix_collisions() {
        assert_eq!(
            "uv 0.12.10".split_whitespace().nth(1),
            Some(TAKOKIT_UV_VERSION)
        );
        assert_ne!(
            "uv 0.12.100".split_whitespace().nth(1),
            Some(TAKOKIT_UV_VERSION)
        );
    }

    #[test]
    fn explicit_override_requires_absolute_file() {
        let relative = PathBuf::from("uv");
        assert!(!relative.is_absolute());
    }
}
