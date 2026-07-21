//! Download, verify, unpack, and locate managed artifact files.

use crate::{
    runtime_command::configure_managed_command, ArtifactEntry, ModelManifest, PackageError,
    PackageResult,
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
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
    let temp_path = downloads_dir.join(format!(
        "{}.{}.part",
        sanitize_file_name(&artifact.name),
        timestamp_now()
    ));
    download_to_temp(url, &artifact.name, &temp_path)?;
    if let Some(expected_bytes) = artifact.bytes {
        let actual_bytes = std::fs::metadata(&temp_path)
            .map(|metadata| metadata.len())
            .map_err(|error| {
                let _ = std::fs::remove_file(&temp_path);
                PackageError::ArtifactInstallFailed {
                    artifact: artifact.name.clone(),
                    reason: error.to_string(),
                }
            })?;
        if actual_bytes != expected_bytes {
            let _ = std::fs::remove_file(&temp_path);
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
        let _ = std::fs::remove_file(&temp_path);
        return Err(PackageError::ArtifactChecksumMismatch {
            artifact: artifact.name.clone(),
            expected,
            actual,
        });
    }
    let final_path = blob_dir.join(&expected);
    if final_path.exists() {
        let valid_existing = artifact.bytes.is_none_or(|expected_bytes| {
            std::fs::metadata(&final_path)
                .map(|metadata| metadata.len() == expected_bytes)
                .unwrap_or(false)
        }) && sha256_file(&final_path)
            .map(|actual| actual == expected)
            .unwrap_or(false);
        if valid_existing {
            let _ = std::fs::remove_file(&temp_path);
        } else {
            std::fs::remove_file(&final_path).map_err(|error| {
                PackageError::ArtifactInstallFailed {
                    artifact: artifact.name.clone(),
                    reason: error.to_string(),
                }
            })?;
            std::fs::rename(&temp_path, &final_path).map_err(|error| {
                let _ = std::fs::remove_file(&temp_path);
                PackageError::ArtifactInstallFailed {
                    artifact: artifact.name.clone(),
                    reason: error.to_string(),
                }
            })?;
        }
    } else {
        std::fs::rename(&temp_path, &final_path).map_err(|error| {
            let _ = std::fs::remove_file(&temp_path);
            PackageError::ArtifactInstallFailed {
                artifact: artifact.name.clone(),
                reason: error.to_string(),
            }
        })?;
    }
    Ok(final_path)
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
        let _ = std::fs::remove_file(temp_path);
        PackageError::ArtifactDownloadFailed {
            artifact: artifact.to_string(),
            reason: error.to_string(),
        }
    })?;
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
        let _ = std::fs::remove_file(temp_path);
        Err(PackageError::ArtifactDownloadFailed {
            artifact: artifact.to_string(),
            reason: format!(
                "managed curl transport exited with {} after retries: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        })
    }
}

fn download_with_ureq(url: &str, artifact: &str, temp_path: &Path) -> PackageResult<()> {
    for attempt in 1..=DOWNLOAD_ATTEMPTS {
        let result = ureq::get(url).call();
        match result {
            Ok(response) => {
                let mut reader = response.into_reader();
                let mut file = File::create(temp_path)?;
                match std::io::copy(&mut reader, &mut file) {
                    Ok(_) => return Ok(()),
                    Err(error) => {
                        let _ = std::fs::remove_file(temp_path);
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
                let body = response.into_string().unwrap_or_default();
                if attempt < DOWNLOAD_ATTEMPTS && retryable_http_status(status) {
                    let _ = std::fs::remove_file(temp_path);
                    thread::sleep(download_retry_delay(attempt));
                    continue;
                }
                return Err(PackageError::ArtifactDownloadFailed {
                    artifact: artifact.to_string(),
                    reason: format!(
                        "upstream returned HTTP {status}: {}",
                        concise_body(&body)
                    ),
                });
            }
            Err(ureq::Error::Transport(error)) => {
                let _ = std::fs::remove_file(temp_path);
                if attempt < DOWNLOAD_ATTEMPTS {
                    thread::sleep(download_retry_delay(attempt));
                    continue;
                }
                return Err(PackageError::ArtifactDownloadFailed {
                    artifact: artifact.to_string(),
                    reason: format!(
                        "transport failed after {DOWNLOAD_ATTEMPTS} attempts: {error}"
                    ),
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
fn timestamp_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::retryable_http_status;

    #[test]
    fn retries_only_transient_upstream_statuses() {
        assert!(retryable_http_status(408));
        assert!(retryable_http_status(429));
        assert!(retryable_http_status(502));
        assert!(retryable_http_status(503));
        assert!(!retryable_http_status(400));
        assert!(!retryable_http_status(404));
    }
}
