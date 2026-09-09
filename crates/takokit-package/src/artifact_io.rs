//! Download, verify, unpack, and locate managed artifact files.

use crate::{
    runtime_command::configure_managed_command, ArtifactEntry, ModelManifest, PackageError,
    PackageResult,
};
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};
use zip::ZipArchive;

const DOWNLOAD_ATTEMPTS: usize = 3;

pub(crate) fn install_artifact(
    manifest: &ModelManifest,
    artifact: &ArtifactEntry,
    downloads_dir: &Path,
    blob_dir: &Path,
) -> PackageResult<PathBuf> {
    let url = artifact
        .url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| PackageError::ArtifactUrlMissing {
            model: manifest.id.clone(),
            artifact: artifact.name.clone(),
        })?;
    let expected = artifact.sha256.trim().to_ascii_lowercase();
    if expected.is_empty() || expected == "todo" {
        return Err(PackageError::ArtifactChecksumMissing {
            model: manifest.id.clone(),
            artifact: artifact.name.clone(),
        });
    }

    std::fs::create_dir_all(downloads_dir)?;
    std::fs::create_dir_all(blob_dir)?;
    let (temp_path, lock_path) = download_paths(downloads_dir, &expected);
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    lock.lock_exclusive()?;

    let final_path = blob_dir.join(&expected);
    if verified_artifact(&final_path, &expected, artifact.bytes) {
        return Ok(final_path);
    }
    if final_path.exists() {
        remove_regular_file(&final_path)?;
    }

    if let (Some(expected_bytes), Ok(metadata)) = (artifact.bytes, std::fs::metadata(&temp_path)) {
        if metadata.len() > expected_bytes {
            remove_regular_file(&temp_path)?;
        }
    }

    download_to_temp(url, &artifact.name, &temp_path)?;
    if let Some(expected_bytes) = artifact.bytes {
        let actual_bytes = std::fs::metadata(&temp_path)
            .map(|metadata| metadata.len())
            .map_err(|error| PackageError::ArtifactInstallFailed {
                artifact: artifact.name.clone(),
                reason: error.to_string(),
            })?;
        if actual_bytes != expected_bytes {
            // Preserve a short partial so the next pull can resume it. An oversized
            // partial is never useful and could make a Range retry unsafe.
            if actual_bytes > expected_bytes {
                let _ = remove_regular_file(&temp_path);
            }
            return Err(PackageError::ArtifactInstallFailed {
                artifact: artifact.name.clone(),
                reason: format!("expected {expected_bytes} bytes, got {actual_bytes}"),
            });
        }
    }
    let actual = sha256_file(&temp_path).map_err(|error| PackageError::ArtifactInstallFailed {
        artifact: artifact.name.clone(),
        reason: error.to_string(),
    })?;
    if actual != expected {
        // A completed file with the wrong digest cannot be resumed safely.
        let _ = remove_regular_file(&temp_path);
        return Err(PackageError::ArtifactChecksumMismatch {
            artifact: artifact.name.clone(),
            expected,
            actual,
        });
    }

    if final_path.exists() {
        remove_regular_file(&final_path)?;
    }
    std::fs::rename(&temp_path, &final_path).map_err(|error| {
        PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: error.to_string(),
        }
    })?;
    Ok(final_path)
}

fn download_paths(downloads_dir: &Path, expected_sha256: &str) -> (PathBuf, PathBuf) {
    (
        downloads_dir.join(format!("{expected_sha256}.part")),
        downloads_dir.join(format!("{expected_sha256}.lock")),
    )
}

fn verified_artifact(path: &Path, expected_sha256: &str, expected_bytes: Option<u64>) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return false;
    }
    if expected_bytes.is_some_and(|bytes| metadata.len() != bytes) {
        return false;
    }
    sha256_file(path)
        .map(|actual| actual == expected_sha256)
        .unwrap_or(false)
}

fn remove_regular_file(path: &Path) -> PackageResult<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
            std::fs::remove_file(path)?;
            Ok(())
        }
        Ok(_) => Err(PackageError::ArtifactInstallFailed {
            artifact: path.display().to_string(),
            reason: "managed artifact path is not a regular file".to_string(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn download_to_temp(url: &str, artifact: &str, temp_path: &Path) -> PackageResult<()> {
    if url.starts_with("http://") || url.starts_with("https://") {
        if download_with_curl(url, artifact, temp_path)? {
            return Ok(());
        }
        return download_with_ureq(url, artifact, temp_path);
    }
    let local_path = url
        .strip_prefix("file://")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(url));
    let mut input =
        File::open(&local_path).map_err(|error| PackageError::ArtifactDownloadFailed {
            artifact: artifact.to_string(),
            reason: error.to_string(),
        })?;
    let mut output = File::create(temp_path)?;
    std::io::copy(&mut input, &mut output).map_err(|error| {
        PackageError::ArtifactDownloadFailed {
            artifact: artifact.to_string(),
            reason: error.to_string(),
        }
    })?;
    output.sync_data()?;
    Ok(())
}

fn download_with_curl(url: &str, artifact: &str, temp_path: &Path) -> PackageResult<bool> {
    let mut command = Command::new("curl");
    command
        .args([
            "--location",
            "--fail",
            "--silent",
            "--show-error",
            "--retry",
            "5",
            "--retry-delay",
            "1",
            "--retry-max-time",
            "300",
            "--connect-timeout",
            "30",
            "--speed-limit",
            "1024",
            "--speed-time",
            "30",
            "--continue-at",
            "-",
            "--output",
        ])
        .arg(temp_path)
        .arg(url);
    configure_managed_command(&mut command);

    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(PackageError::ArtifactDownloadFailed {
                artifact: artifact.to_string(),
                reason: format!("could not start managed curl transport: {error}"),
            })
        }
    };
    if output.status.success() {
        Ok(true)
    } else {
        // Keep the .part file. ureq is a real fallback transport and can either
        // resume it with HTTP Range or restart it when the origin ignores Range.
        Ok(false)
    }
}

fn download_with_ureq(url: &str, artifact: &str, temp_path: &Path) -> PackageResult<()> {
    for attempt in 1..=DOWNLOAD_ATTEMPTS {
        let existing = std::fs::metadata(temp_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let mut request = ureq::get(url);
        if existing > 0 {
            request = request.set("Range", &format!("bytes={existing}-"));
        }

        match request.call() {
            Ok(response) => {
                let status = response.status();
                let append = existing > 0 && status == 206;
                let mut options = OpenOptions::new();
                options.create(true).write(true);
                if append {
                    options.append(true);
                } else {
                    // A 200 response to a Range request means the origin ignored
                    // Range. Restart from byte zero rather than duplicating data.
                    options.truncate(true);
                }
                let mut file = options.open(temp_path)?;
                let mut reader = response.into_reader();
                match std::io::copy(&mut reader, &mut file) {
                    Ok(_) => {
                        file.sync_data()?;
                        return Ok(());
                    }
                    Err(error) => {
                        // Preserve the bytes already written. A later attempt or a
                        // future Takokit process can continue from this exact file.
                        let _ = file.sync_data();
                        if attempt < DOWNLOAD_ATTEMPTS {
                            thread::sleep(download_retry_delay(attempt));
                            continue;
                        }
                        return Err(PackageError::ArtifactDownloadFailed {
                            artifact: artifact.to_string(),
                            reason: format!(
                                "response body failed after {DOWNLOAD_ATTEMPTS} attempts: {error}"
                            ),
                        });
                    }
                }
            }
            Err(ureq::Error::Status(status, response)) => {
                // 416 can mean a previously interrupted download already reached
                // EOF. Let the caller perform the authoritative size + SHA check.
                if status == 416 && existing > 0 {
                    return Ok(());
                }
                let body = response.into_string().unwrap_or_default();
                if attempt < DOWNLOAD_ATTEMPTS && retryable_http_status(status) {
                    thread::sleep(download_retry_delay(attempt));
                    continue;
                }
                return Err(PackageError::ArtifactDownloadFailed {
                    artifact: artifact.to_string(),
                    reason: format!("upstream returned HTTP {status}: {}", concise_body(&body)),
                });
            }
            Err(ureq::Error::Transport(error)) => {
                if attempt < DOWNLOAD_ATTEMPTS {
                    thread::sleep(download_retry_delay(attempt));
                    continue;
                }
                return Err(PackageError::ArtifactDownloadFailed {
                    artifact: artifact.to_string(),
                    reason: format!("transport failed after {DOWNLOAD_ATTEMPTS} attempts: {error}"),
                });
            }
        }
    }

    Err(PackageError::ArtifactDownloadFailed {
        artifact: artifact.to_string(),
        reason: "download attempts were exhausted".to_string(),
    })
}

fn retryable_http_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

fn download_retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(500 * (1_u64 << attempt.saturating_sub(1).min(4)))
}

fn concise_body(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return "no response body".to_string();
    }
    body.chars().take(300).collect()
}

pub(crate) fn extract_zip_safely(
    archive_path: &Path,
    output_dir: &Path,
    artifact: &str,
) -> PackageResult<()> {
    let file = File::open(archive_path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: artifact.to_string(),
            reason: error.to_string(),
        })?;
    for index in 0..archive.len() {
        let mut item =
            archive
                .by_index(index)
                .map_err(|error| PackageError::ArtifactInstallFailed {
                    artifact: artifact.to_string(),
                    reason: error.to_string(),
                })?;
        let Some(enclosed_name) = item.enclosed_name() else {
            continue;
        };
        let output_path = output_dir.join(enclosed_name);
        if item.is_dir() {
            std::fs::create_dir_all(&output_path)?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&output_path)?;
        std::io::copy(&mut item, &mut output).map_err(|error| {
            PackageError::ArtifactInstallFailed {
                artifact: artifact.to_string(),
                reason: error.to_string(),
            }
        })?;
        output.flush()?;
    }
    Ok(())
}

pub(crate) fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}
pub(crate) fn find_file_named(root: &Path, name: String) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_named(&path, name.clone()) {
                return Some(found);
            }
        } else if path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case(&name))
            .unwrap_or(false)
        {
            return Some(path);
        }
    }
    None
}
pub(crate) fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
fn sanitize_file_name(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{download_paths, retryable_http_status, verified_artifact};
    use sha2::{Digest, Sha256};
    use std::fs;

    #[test]
    fn retries_only_transient_upstream_statuses() {
        assert!(retryable_http_status(408));
        assert!(retryable_http_status(429));
        assert!(retryable_http_status(502));
        assert!(retryable_http_status(503));
        assert!(!retryable_http_status(400));
        assert!(!retryable_http_status(404));
    }

    #[test]
    fn resumable_download_paths_are_stable_across_processes() {
        let root = tempfile::tempdir().unwrap();
        let digest = "a".repeat(64);
        let first = download_paths(root.path(), &digest);
        let second = download_paths(root.path(), &digest);
        assert_eq!(first, second);
        assert_eq!(first.0, root.path().join(format!("{digest}.part")));
        assert_eq!(first.1, root.path().join(format!("{digest}.lock")));
    }

    #[test]
    fn verified_artifact_rejects_symlinks_and_wrong_content() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("blob");
        fs::write(&path, b"takokit").unwrap();
        let digest = format!("{:x}", Sha256::digest(b"takokit"));
        assert!(verified_artifact(&path, &digest, Some(7)));
        assert!(!verified_artifact(&path, &digest, Some(8)));
        assert!(!verified_artifact(&path, &"0".repeat(64), Some(7)));
    }
}
