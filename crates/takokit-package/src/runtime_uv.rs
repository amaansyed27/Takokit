//! Deterministic Takokit-managed uv discovery and bootstrap.

use crate::{artifact_io::sha256_file, *};
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    io::{Cursor, Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const TAKOKIT_UV_VERSION: &str = "0.12.9";
const MAX_UV_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
enum ArchiveKind {
    TarGz,
    Zip,
}

#[derive(Debug, Clone, Copy)]
struct UvAsset {
    asset: &'static str,
    inner_path: &'static str,
    sha256: &'static str,
    archive: ArchiveKind,
}

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
        let override_path = explicit_uv_override().ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!(
                "UV must be an absolute path to an existing uv {} executable; see {}",
                TAKOKIT_UV_VERSION,
                log.display()
            ),
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
    lock.lock_exclusive().map_err(|error| PackageError::ArtifactInstallFailed {
        artifact: "uv bootstrap".to_string(),
        reason: format!("could not lock managed uv bootstrap: {error}"),
    })?;

    // Another process may have completed the bootstrap while this process waited.
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
    let _ = fs2::FileExt::unlock(&lock);
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

fn uv_asset() -> PackageResult<UvAsset> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok(UvAsset {
            asset: "uv-x86_64-pc-windows-msvc.zip",
            inner_path: "uv-x86_64-pc-windows-msvc/uv.exe",
            sha256: "ddbfcee1ac615a0499f6aa97b5ec8ebdf3ee4a7714a48055ec2ba0030e3cf810",
            archive: ArchiveKind::Zip,
        }),
        ("linux", "x86_64") => Ok(UvAsset {
            asset: "uv-x86_64-unknown-linux-gnu.tar.gz",
            inner_path: "uv-x86_64-unknown-linux-gnu/uv",
            sha256: "ec7a99cd05e0cd7f80243f135ce1361c76835cb0ee60055d14d20eba8eba1460",
            archive: ArchiveKind::TarGz,
        }),
        ("macos", "aarch64") => Ok(UvAsset {
            asset: "uv-aarch64-apple-darwin.tar.gz",
            inner_path: "uv-aarch64-apple-darwin/uv",
            sha256: "301f72afaf54060f92da7016cb0115bd077f43a9c8e39c1d8170a0bac80fd398",
            archive: ArchiveKind::TarGz,
        }),
        (os, arch) => Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("Takokit has no pinned uv {} asset for {os}/{arch}", TAKOKIT_UV_VERSION),
        }),
    }
}

fn download_and_promote_uv(
    parent: &Path,
    managed: &Path,
    asset: UvAsset,
    log: &Path,
) -> PackageResult<PathBuf> {
    let nonce = format!("{}-{}", std::process::id(), unix_timestamp());
    let staging = parent.join(format!(".bootstrap-{nonce}"));
    std::fs::create_dir_all(&staging)?;
    let archive_path = staging.join(asset.asset);
    let candidate = staging.join(if cfg!(windows) { "uv.exe" } else { "uv" });
    let url = format!(
        "https://github.com/astral-sh/uv/releases/download/{}/{}",
        TAKOKIT_UV_VERSION, asset.asset
    );

    let result = (|| {
        download_exact(&url, &archive_path)?;
        let actual_sha = sha256_file(&archive_path).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: error.to_string(),
        })?;
        log_source(
            log,
            "upstream_download",
            &url,
            Some(&actual_sha),
            Some(asset.sha256),
        )?;
        if !actual_sha.eq_ignore_ascii_case(asset.sha256) {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "uv bootstrap".to_string(),
                reason: format!(
                    "official uv archive checksum mismatch for {}; expected {}, got {}; see {}",
                    asset.asset,
                    asset.sha256,
                    actual_sha,
                    log.display()
                ),
            });
        }

        match asset.archive {
            ArchiveKind::Zip => extract_zip_member(&archive_path, asset.inner_path, &candidate)?,
            ArchiveKind::TarGz => extract_tar_member(&archive_path, asset.inner_path, &candidate)?,
        }
        ensure_owner_writable_executable(&candidate)?;
        if !verify_uv_version(&candidate, log)? {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "uv bootstrap".to_string(),
                reason: format!(
                    "downloaded official uv asset does not report pinned version {}; see {}",
                    TAKOKIT_UV_VERSION,
                    log.display()
                ),
            });
        }

        if managed.exists() {
            quarantine_invalid_managed_uv(managed, log)?;
        }
        std::fs::rename(&candidate, managed).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not atomically promote managed uv: {error}"),
        })?;
        ensure_owner_writable_executable(managed)?;
        if !verify_uv_version(managed, log)? {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "uv bootstrap".to_string(),
                reason: format!("promoted managed uv failed final verification; see {}", log.display()),
            });
        }
        append_log(log, &format!("managed_binary_sha256: {}\nbootstrap_result: ready\n", observed_sha256(managed).unwrap_or_else(|| "unavailable".to_string())))?;
        Ok(managed.to_path_buf())
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn download_exact(url: &str, destination: &Path) -> PackageResult<()> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(30))
        .call()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not download pinned official uv from {url}: {error}"),
        })?;
    let mut reader = response.into_reader().take(MAX_UV_ARCHIVE_BYTES + 1);
    let mut file = File::create(destination)?;
    let copied = std::io::copy(&mut reader, &mut file)?;
    file.flush()?;
    if copied == 0 || copied > MAX_UV_ARCHIVE_BYTES {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("downloaded uv archive has unsafe size: {copied} bytes"),
        });
    }
    Ok(())
}

fn extract_zip_member(archive_path: &Path, inner_path: &str, destination: &Path) -> PackageResult<()> {
    let bytes = std::fs::read(archive_path)?;
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|error| PackageError::ArtifactInstallFailed {
        artifact: "uv bootstrap".to_string(),
        reason: format!("official uv zip is invalid: {error}"),
    })?;
    let mut entry = archive.by_name(inner_path).map_err(|error| PackageError::ArtifactInstallFailed {
        artifact: "uv bootstrap".to_string(),
        reason: format!("official uv zip is missing {inner_path}: {error}"),
    })?;
    if !safe_archive_path(Path::new(inner_path)) || entry.is_dir() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: "official uv zip member is not a safe executable file".to_string(),
        });
    }
    let mut output = File::create(destination)?;
    std::io::copy(&mut entry, &mut output)?;
    output.flush()?;
    Ok(())
}

fn extract_tar_member(archive_path: &Path, inner_path: &str, destination: &Path) -> PackageResult<()> {
    if !safe_archive_path(Path::new(inner_path)) {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: "pinned uv archive member path is unsafe".to_string(),
        });
    }
    let listing = Command::new("tar")
        .arg("-tzf")
        .arg(archive_path)
        .output()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not inspect official uv tar archive: {error}"),
        })?;
    if !listing.status.success() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: "official uv tar archive failed safety inspection".to_string(),
        });
    }
    let names = String::from_utf8_lossy(&listing.stdout);
    let mut found = false;
    for name in names.lines().filter(|line| !line.trim().is_empty()) {
        if !safe_archive_path(Path::new(name)) || name.contains('\\') {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "uv bootstrap".to_string(),
                reason: format!("official uv tar contains unsafe path: {name}"),
            });
        }
        found |= name == inner_path;
    }
    if !found {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("official uv tar is missing {inner_path}"),
        });
    }

    let verbose = Command::new("tar")
        .arg("-tvzf")
        .arg(archive_path)
        .output()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not inspect uv archive entry types: {error}"),
        })?;
    if !verbose.status.success()
        || String::from_utf8_lossy(&verbose.stdout)
            .lines()
            .any(|line| matches!(line.as_bytes().first(), Some(b'l' | b'h')))
    {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: "official uv tar contains a symbolic/hard link or failed type inspection".to_string(),
        });
    }

    let output = Command::new("tar")
        .arg("-xOzf")
        .arg(archive_path)
        .arg(inner_path)
        .output()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not safely extract uv executable: {error}"),
        })?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: "could not extract pinned uv executable from verified archive".to_string(),
        });
    }
    std::fs::write(destination, output.stdout)?;
    Ok(())
}

fn safe_archive_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path.components().all(|component| {
            matches!(component, Component::Normal(_) | Component::CurDir)
        })
}

fn quarantine_invalid_managed_uv(managed: &Path, log: &Path) -> PackageResult<()> {
    if !managed.exists() {
        return Ok(());
    }
    make_owner_writable(managed)?;
    let quarantine = managed.with_file_name(format!(
        "{}.invalid-{}",
        managed.file_name().and_then(|value| value.to_str()).unwrap_or("uv"),
        unix_timestamp()
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
    std::fs::rename(managed, &quarantine).map_err(|error| PackageError::ArtifactInstallFailed {
        artifact: "uv bootstrap".to_string(),
        reason: format!("could not quarantine invalid managed uv: {error}"),
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
    fn pinned_uv_assets_cover_slice_six_targets() {
        let expected = [
            ("uv-x86_64-pc-windows-msvc.zip", 64),
            ("uv-x86_64-unknown-linux-gnu.tar.gz", 64),
            ("uv-aarch64-apple-darwin.tar.gz", 64),
        ];
        for (name, digest_len) in expected {
            let digest = match name {
                "uv-x86_64-pc-windows-msvc.zip" => "ddbfcee1ac615a0499f6aa97b5ec8ebdf3ee4a7714a48055ec2ba0030e3cf810",
                "uv-x86_64-unknown-linux-gnu.tar.gz" => "ec7a99cd05e0cd7f80243f135ce1361c76835cb0ee60055d14d20eba8eba1460",
                _ => "301f72afaf54060f92da7016cb0115bd077f43a9c8e39c1d8170a0bac80fd398",
            };
            assert_eq!(digest.len(), digest_len);
            assert!(digest.chars().all(|value| value.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn exact_version_check_does_not_accept_prefix_collisions() {
        assert_eq!("uv 0.12.9".split_whitespace().nth(1), Some(TAKOKIT_UV_VERSION));
        assert_ne!("uv 0.12.90".split_whitespace().nth(1), Some(TAKOKIT_UV_VERSION));
    }

    #[test]
    fn archive_paths_reject_parent_and_absolute_components() {
        assert!(safe_archive_path(Path::new("uv-aarch64-apple-darwin/uv")));
        assert!(!safe_archive_path(Path::new("../uv")));
        assert!(!safe_archive_path(Path::new("/tmp/uv")));
    }
}
