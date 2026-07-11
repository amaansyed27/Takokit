//! Verified whisper.cpp runtime installation.

use crate::{
    artifact_io::{
        download_to_temp, executable_name, extract_zip_safely, find_file_named, sha256_file,
    },
    *,
};
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
        return installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::RuntimeInstalled,
            format!(
                "whisper.cpp runtime directory initialized at {}. Automatic binary installation is currently implemented for Windows x64 only.",
                layout.root.display()
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
