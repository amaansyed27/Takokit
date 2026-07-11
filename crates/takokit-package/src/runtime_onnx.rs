//! Managed ONNX runtime installation.

use crate::{
    runtime_command::{run_logged_command, runner_python_path},
    runtime_uv::bootstrap_uv,
    *,
};
use std::path::Path;
const KOKORO_ONNX_PACKAGE: &str = "kokoro-onnx==0.5.0";
const KOKORO_ONNX_ADAPTER: &str = include_str!("../../../runners/onnx/kokoro_adapter.py");

pub(crate) fn install_onnx_runtime(
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
    layout: &RunnerRuntimeLayout,
) -> PackageResult<PullReport> {
    let runtime_dir = layout.root.join("runtime");
    let venv_dir = runtime_dir.join("venv");
    let adapters_dir = layout.root.join("adapters");
    let log_path = layout.logs.join("install-kokoro-onnx.log");
    std::fs::create_dir_all(&runtime_dir)?;
    std::fs::create_dir_all(&adapters_dir)?;

    let takokit_root = layout.root.parent().and_then(Path::parent).ok_or_else(|| {
        PackageError::ArtifactInstallFailed {
            artifact: "kokoro-onnx runtime".to_string(),
            reason: "cannot resolve Takokit storage root".to_string(),
        }
    })?;
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log_path,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            "3.12".into(),
            "--allow-existing".into(),
            venv_dir.clone().into(),
        ],
    )?;
    let python =
        runner_python_path(&venv_dir).ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: "kokoro-onnx runtime".to_string(),
            reason: format!(
                "uv created no Python executable below {}; see {}",
                venv_dir.display(),
                log_path.display()
            ),
        })?;
    run_logged_command(
        &log_path,
        &uv,
        &[
            "pip".into(),
            "install".into(),
            "--python".into(),
            python.clone().into(),
            "--no-progress".into(),
            KOKORO_ONNX_PACKAGE.into(),
        ],
    )?;
    std::fs::write(adapters_dir.join("kokoro.py"), KOKORO_ONNX_ADAPTER)?;

    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "Kokoro ONNX runtime is ready at {} using Python {} and {}. Piper remains blocked by the typed piper_text_frontend_not_implemented boundary. Install log: {}",
            layout.root.display(),
            python.display(),
            KOKORO_ONNX_PACKAGE,
            log_path.display()
        ),
    )
}
