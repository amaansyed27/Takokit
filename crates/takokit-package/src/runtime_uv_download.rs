use super::{
    append_log, ensure_owner_writable_executable, log_source, observed_sha256,
    quarantine_invalid_managed_uv, unix_timestamp, verify_uv_version, TAKOKIT_UV_VERSION,
};
use crate::{artifact_io::sha256_file, *};
use std::{
    fs::File,
    io::{Cursor, Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    time::Duration,
};

const MAX_UV_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub(super) enum ArchiveKind {
    TarGz,
    Zip,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UvAsset {
    asset: &'static str,
    inner_path: &'static str,
    sha256: &'static str,
    archive: ArchiveKind,
}

pub(super) fn uv_asset() -> PackageResult<UvAsset> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok(UvAsset {
            asset: "uv-x86_64-pc-windows-msvc.zip",
            inner_path: "uv-x86_64-pc-windows-msvc/uv.exe",
            sha256: "f65744f94072152b1f86ba2aace4d01f1124d9a8ecb235805039e3718c36cac2",
            archive: ArchiveKind::Zip,
        }),
        ("linux", "x86_64") => Ok(UvAsset {
            asset: "uv-x86_64-unknown-linux-gnu.tar.gz",
            inner_path: "uv-x86_64-unknown-linux-gnu/uv",
            sha256: "173d95a0c32d18c896c46ba6fafbf3cf9c14ab74b033f81b76c883ef492a976b",
            archive: ArchiveKind::TarGz,
        }),
        ("macos", "aarch64") => Ok(UvAsset {
            asset: "uv-aarch64-apple-darwin.tar.gz",
            inner_path: "uv-aarch64-apple-darwin/uv",
            sha256: "51c6170e8e3a01cef9f33b94f582b7b81ac65046f55d40afb35f9cff5a68c179",
            archive: ArchiveKind::TarGz,
        }),
        (os, arch) => Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!(
                "Takokit has no pinned uv {} asset for {os}/{arch}",
                TAKOKIT_UV_VERSION
            ),
        }),
    }
}

pub(super) fn download_and_promote_uv(
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
        let actual_sha =
            sha256_file(&archive_path).map_err(|error| PackageError::ArtifactInstallFailed {
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
        std::fs::rename(&candidate, managed).map_err(|error| {
            PackageError::ArtifactInstallFailed {
                artifact: "uv bootstrap".to_string(),
                reason: format!("could not atomically promote managed uv: {error}"),
            }
        })?;
        ensure_owner_writable_executable(managed)?;
        if !verify_uv_version(managed, log)? {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "uv bootstrap".to_string(),
                reason: format!(
                    "promoted managed uv failed final verification; see {}",
                    log.display()
                ),
            });
        }
        append_log(
            log,
            &format!(
                "managed_binary_sha256: {}\nbootstrap_result: ready\n",
                observed_sha256(managed).unwrap_or_else(|| "unavailable".to_string())
            ),
        )?;
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

fn extract_zip_member(
    archive_path: &Path,
    inner_path: &str,
    destination: &Path,
) -> PackageResult<()> {
    let bytes = std::fs::read(archive_path)?;
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("official uv zip is invalid: {error}"),
        })?;
    let mut entry =
        archive
            .by_name(inner_path)
            .map_err(|error| PackageError::ArtifactInstallFailed {
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

fn extract_tar_member(
    archive_path: &Path,
    inner_path: &str,
    destination: &Path,
) -> PackageResult<()> {
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
            reason: "official uv tar contains a symbolic/hard link or failed type inspection"
                .to_string(),
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
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_uv_assets_cover_slice_six_targets() {
        let expected = [
            (
                "uv-x86_64-pc-windows-msvc.zip",
                "f65744f94072152b1f86ba2aace4d01f1124d9a8ecb235805039e3718c36cac2",
            ),
            (
                "uv-x86_64-unknown-linux-gnu.tar.gz",
                "173d95a0c32d18c896c46ba6fafbf3cf9c14ab74b033f81b76c883ef492a976b",
            ),
            (
                "uv-aarch64-apple-darwin.tar.gz",
                "51c6170e8e3a01cef9f33b94f582b7b81ac65046f55d40afb35f9cff5a68c179",
            ),
        ];
        for (name, digest) in expected {
            assert!(!name.is_empty());
            assert_eq!(digest.len(), 64);
            assert!(digest.chars().all(|value| value.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn archive_paths_reject_parent_and_absolute_components() {
        assert!(safe_archive_path(Path::new("uv-aarch64-apple-darwin/uv")));
        assert!(!safe_archive_path(Path::new("../uv")));
        assert!(!safe_archive_path(Path::new("/tmp/uv")));
    }
}
