//! Download, verify, unpack, and locate managed artifact files.

use crate::{
    runtime_command::{run_logged_command, PathOrArg},
    runtime_uv::bootstrap_uv,
    ArtifactEntry, ModelManifest, PackageError, PackageResult,
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
use zip::ZipArchive;

pub(crate) fn install_artifact(
    manifest: &ModelManifest,
    artifact: &ArtifactEntry,
    downloads_dir: &Path,
    blob_dir: &Path,
) -> PackageResult<PathBuf> {
    if artifact
        .url
        .as_deref()
        .is_some_and(|url| url.starts_with("hf://"))
    {
        return install_huggingface_snapshot(manifest, artifact, downloads_dir, blob_dir);
    }

    let url = artifact
        .url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| PackageError::ArtifactUrlMissing {
            model: manifest.id.clone(),
            artifact: artifact.name.clone(),
        })?;
    let expected = expected_checksum(manifest, artifact)?;
    let temp_path = downloads_dir.join(format!(
        "{}.{}.part",
        sanitize_file_name(&artifact.name),
        timestamp_now()
    ));
    download_to_temp(url, &artifact.name, &temp_path)?;
    verify_size(&temp_path, artifact)?;
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
    persist_verified_blob(&temp_path, blob_dir, artifact, &actual)
}

fn install_huggingface_snapshot(
    manifest: &ModelManifest,
    artifact: &ArtifactEntry,
    downloads_dir: &Path,
    blob_dir: &Path,
) -> PackageResult<PathBuf> {
    let specification = artifact
        .url
        .as_deref()
        .and_then(|value| value.strip_prefix("hf://"))
        .ok_or_else(|| PackageError::ArtifactUrlMissing {
            model: manifest.id.clone(),
            artifact: artifact.name.clone(),
        })?;
    let (repository, revision) = specification
        .rsplit_once('@')
        .unwrap_or((specification, "main"));
    if repository.trim().is_empty() || revision.trim().is_empty() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: "invalid hf:// snapshot specification".to_string(),
        });
    }

    let root = blob_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: "could not resolve Takokit storage root for snapshot".to_string(),
        })?;
    let model_dir = root.join("models").join(&manifest.id);
    let helper = root.join("cache").join("hf_snapshot_download.py");
    let log = root
        .join("logs")
        .join(format!("snapshot-{}.log", manifest.id));
    std::fs::create_dir_all(&model_dir)?;
    std::fs::create_dir_all(downloads_dir)?;
    std::fs::create_dir_all(blob_dir)?;
    if let Some(parent) = helper.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &helper,
        r#"from pathlib import Path
import sys
from huggingface_hub import snapshot_download
repo, revision, output = sys.argv[1:4]
Path(output).mkdir(parents=True, exist_ok=True)
snapshot_download(repo_id=repo, revision=revision, local_dir=output)
"#,
    )?;

    let uv = bootstrap_uv(root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "run".into(),
            "--no-project".into(),
            "--with".into(),
            "huggingface_hub".into(),
            "python".into(),
            helper.into(),
            repository.into(),
            revision.into(),
            model_dir.clone().into(),
        ],
    )?;
    let has_payload = std::fs::read_dir(&model_dir)?
        .flatten()
        .any(|entry| entry.file_name() != ".takokit-snapshot");
    if !has_payload {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: format!(
                "Hugging Face snapshot produced no model files; see {}",
                log.display()
            ),
        });
    }

    let marker = format!("hf://{repository}@{revision}");
    let expected = expected_checksum(manifest, artifact)?;
    let actual = format!("{:x}", Sha256::digest(marker.as_bytes()));
    if actual != expected {
        return Err(PackageError::ArtifactChecksumMismatch {
            artifact: artifact.name.clone(),
            expected,
            actual,
        });
    }
    if artifact
        .bytes
        .is_some_and(|expected_bytes| expected_bytes != marker.len() as u64)
    {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: "snapshot marker byte count does not match manifest".to_string(),
        });
    }
    let final_path = blob_dir.join(&actual);
    std::fs::write(&final_path, marker.as_bytes())?;
    Ok(final_path)
}

fn expected_checksum(
    manifest: &ModelManifest,
    artifact: &ArtifactEntry,
) -> PackageResult<String> {
    let expected = artifact.sha256.trim().to_ascii_lowercase();
    if expected.is_empty() || expected == "todo" {
        return Err(PackageError::ArtifactChecksumMissing {
            model: manifest.id.clone(),
            artifact: artifact.name.clone(),
        });
    }
    Ok(expected)
}

fn verify_size(path: &Path, artifact: &ArtifactEntry) -> PackageResult<()> {
    let Some(expected_bytes) = artifact.bytes else {
        return Ok(());
    };
    let actual_bytes = std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: error.to_string(),
        })?;
    if actual_bytes == expected_bytes {
        return Ok(());
    }
    let _ = std::fs::remove_file(path);
    Err(PackageError::ArtifactInstallFailed {
        artifact: artifact.name.clone(),
        reason: format!("expected {expected_bytes} bytes, got {actual_bytes}"),
    })
}

fn persist_verified_blob(
    temp_path: &Path,
    blob_dir: &Path,
    artifact: &ArtifactEntry,
    checksum: &str,
) -> PackageResult<PathBuf> {
    let final_path = blob_dir.join(checksum);
    if final_path.exists() {
        let valid_existing = artifact.bytes.is_none_or(|expected_bytes| {
            std::fs::metadata(&final_path)
                .map(|metadata| metadata.len() == expected_bytes)
                .unwrap_or(false)
        }) && sha256_file(&final_path)
            .map(|actual| actual == checksum)
            .unwrap_or(false);
        if valid_existing {
            let _ = std::fs::remove_file(temp_path);
            return Ok(final_path);
        }
        std::fs::remove_file(&final_path).map_err(|error| {
            PackageError::ArtifactInstallFailed {
                artifact: artifact.name.clone(),
                reason: error.to_string(),
            }
        })?;
    }
    std::fs::rename(temp_path, &final_path).map_err(|error| {
        let _ = std::fs::remove_file(temp_path);
        PackageError::ArtifactInstallFailed {
            artifact: artifact.name.clone(),
            reason: error.to_string(),
        }
    })?;
    Ok(final_path)
}

pub(crate) fn download_to_temp(url: &str, artifact: &str, temp_path: &Path) -> PackageResult<()> {
    if url.starts_with("http://") || url.starts_with("https://") {
        if url.contains("huggingface.co/") && download_with_curl(url, artifact, temp_path)? {
            return Ok(());
        }
        let response =
            ureq::get(url)
                .call()
                .map_err(|error| PackageError::ArtifactDownloadFailed {
                    artifact: artifact.to_string(),
                    reason: error.to_string(),
                })?;
        let mut reader = response.into_reader();
        let mut file = File::create(temp_path)?;
        std::io::copy(&mut reader, &mut file).map_err(|error| {
            let _ = std::fs::remove_file(temp_path);
            PackageError::ArtifactDownloadFailed {
                artifact: artifact.to_string(),
                reason: error.to_string(),
            }
        })?;
        return Ok(());
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
    let output = match Command::new("curl")
        .args([
            "--location",
            "--fail",
            "--silent",
            "--show-error",
            "--retry",
            "3",
            "--output",
        ])
        .arg(temp_path)
        .arg(url)
        .output()
    {
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
                "managed curl transport exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        })
    }
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
