//! Verified whisper.cpp runtime installation.

use crate::{
    artifact_io::{
        download_to_temp, executable_name, extract_zip_safely, find_file_named, sha256_file,
    },
    *,
};
use std::path::Path;
const WHISPERCPP_WIN_X64_URL: &str =
    "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.1/whisper-bin-x64.zip";
const WHISPERCPP_WIN_X64_SHA256: &str =
    "7d8be46ecd31828e1eb7a2ecdd0d6b314feafd82163038ab6092594b0a063539";

pub(crate) fn install_whispercpp_runtime(
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
    layout: &RunnerRuntimeLayout,
) -> PackageResult<PullReport> {
    let runtime_dir = layout.root.join("runtime");
    let downloads_dir = layout.root.join("cache").join("downloads");
    std::fs::create_dir_all(&runtime_dir)?;
    std::fs::create_dir_all(&downloads_dir)?;

    if !(cfg!(target_os = "windows") && cfg!(target_arch = "x86_64")) {
        let bundled =
            bundled_unix_runtime().ok_or_else(|| PackageError::ArtifactInstallFailed {
                artifact: "whisper.cpp bundled runtime".to_string(),
                reason: "installed distribution does not contain the native whisper.cpp runtime"
                    .to_string(),
            })?;
        copy_runtime_tree(&bundled, &runtime_dir)?;
        let binary =
            find_file_named(&runtime_dir, executable_name("whisper-cli")).ok_or_else(|| {
                PackageError::ArtifactInstallFailed {
                    artifact: "whisper.cpp bundled runtime".to_string(),
                    reason: "bundled runtime did not contain whisper-cli".to_string(),
                }
            })?;
        make_executable(&binary)?;
        return installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::Ready,
            format!(
                "Bundled whisper.cpp v1.9.1 runtime installed at {}. Executable: {}",
                runtime_dir.display(),
                binary.display()
            ),
        );
    }

    let archive_path = downloads_dir.join("whisper-bin-x64-v1.9.1.zip");
    if !archive_path.is_file() {
        download_to_temp(WHISPERCPP_WIN_X64_URL, "whisper-bin-x64.zip", &archive_path)?;
    }
    let actual =
        sha256_file(&archive_path).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "whisper-bin-x64.zip".to_string(),
            reason: error.to_string(),
        })?;
    if actual != WHISPERCPP_WIN_X64_SHA256 {
        let _ = std::fs::remove_file(&archive_path);
        return Err(PackageError::ArtifactChecksumMismatch {
            artifact: "whisper-bin-x64.zip".to_string(),
            expected: WHISPERCPP_WIN_X64_SHA256.to_string(),
            actual,
        });
    }

    extract_zip_safely(&archive_path, &runtime_dir, "whisper-bin-x64.zip")?;
    let binary =
        find_file_named(&runtime_dir, executable_name("whisper-cli")).ok_or_else(|| {
            PackageError::ArtifactInstallFailed {
                artifact: "whisper-bin-x64.zip".to_string(),
                reason: "archive did not contain whisper-cli executable".to_string(),
            }
        })?;

    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "whisper.cpp v1.9.1 runtime installed at {}. Executable: {}",
            runtime_dir.display(),
            binary.display()
        ),
    )
}

fn bundled_unix_runtime() -> Option<std::path::PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let root = executable.parent()?.parent()?;
    let candidate = root
        .join("resources")
        .join("runners")
        .join("whispercpp-runtime");
    candidate.is_dir().then_some(candidate)
}

fn copy_runtime_tree(source: &Path, destination: &Path) -> PackageResult<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "whisper.cpp bundled runtime".to_string(),
                reason: format!(
                    "bundled runtime contains a symlink: {}",
                    source_path.display()
                ),
            });
        }
        if file_type.is_dir() {
            copy_runtime_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> PackageResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> PackageResult<()> {
    Ok(())
}
