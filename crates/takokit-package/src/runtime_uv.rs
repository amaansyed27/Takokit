//! Managed uv discovery, bootstrap, and pinned-version verification.

use crate::{artifact_io::sha256_file, *};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
const TAKOKIT_UV_VERSION: &str = "0.11.24";

/// Returns the uv executable Takokit will use. Runner installation never falls
/// back implicitly after bootstrap; the managed path or explicit `UV` wins.
pub fn find_uv(takokit_root: &Path) -> Option<PathBuf> {
    let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
    let managed = takokit_root.join("tools").join("uv").join(executable);
    if managed.is_file() {
        return Some(managed);
    }
    if let Ok(path) = std::env::var("UV") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn find_uv_bootstrap_source() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("UV") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|entry| entry.join(executable))
        .find(|candidate| candidate.is_file())
}

/// Attempts a Windows-first uv bootstrap and writes all output to the Takokit
/// log directory. It deliberately returns an error instead of allowing a
/// runner to be marked ready when bootstrap cannot complete.
pub fn bootstrap_uv(takokit_root: &Path) -> PackageResult<PathBuf> {
    let logs = takokit_root.join("logs");
    std::fs::create_dir_all(&logs)?;
    let log = logs.join("uv-bootstrap.log");
    let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
    let managed = takokit_root.join("tools").join("uv").join(executable);
    if managed.is_file() && verify_uv_version(&managed, &log)? {
        return Ok(managed);
    }
    let source = find_uv_bootstrap_source().ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: "uv bootstrap".to_string(),
        reason: format!(
            "no uv bootstrap source was found. Set UV to a pinned uv {} binary, then rerun. See {}",
            TAKOKIT_UV_VERSION,
            log.display()
        ),
    })?;
    std::fs::create_dir_all(managed.parent().expect("managed uv parent"))?;
    std::fs::copy(&source, &managed)?;
    let source_hash =
        sha256_file(&source).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: error.to_string(),
        })?;
    std::fs::write(&log, format!(
        "Takokit managed uv bootstrap\nsource: {}\nmanaged_path: {}\nrequested_version: {}\nsha256: {}\n",
        source.display(), managed.display(), TAKOKIT_UV_VERSION, source_hash
    ))?;
    if verify_uv_version(&managed, &log)? {
        Ok(managed)
    } else {
        let _ = std::fs::remove_file(&managed);
        Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!(
                "managed uv does not report pinned version {}; see {}",
                TAKOKIT_UV_VERSION,
                log.display()
            ),
        })
    }
}

pub(crate) fn verify_uv_version(path: &Path, log: &Path) -> PackageResult<bool> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not run {}: {error}", path.display()),
        })?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)?
        .write_all(
            format!(
                "verified_command: {} --version\nreported_version: {}\n",
                path.display(),
                version
            )
            .as_bytes(),
        )?;
    Ok(output.status.success() && version.starts_with(&format!("uv {TAKOKIT_UV_VERSION}")))
}
