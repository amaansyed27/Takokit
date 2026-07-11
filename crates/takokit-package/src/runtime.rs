//! Runner runtime layout and installer dispatch.

use crate::{
    runtime_onnx::install_onnx_runtime, runtime_python::install_python_managed_runtime,
    runtime_whisper::install_whispercpp_runtime, *,
};
use std::path::Path;

pub fn python_managed_runner_layout(takokit_root: &Path) -> PythonManagedRunnerLayout {
    let root = takokit_root.join("runners").join("python-managed");
    PythonManagedRunnerLayout {
        runtime: root.join("runtime"),
        env: root.join("env"),
        packages: root.join("packages"),
        wheels: root.join("wheels"),
        logs: root.join("logs"),
        manifests: root.join("manifests"),
        cache: root.join("cache"),
        adapters: root.join("adapters"),
        root,
    }
}

pub fn runner_runtime_layout(
    takokit_root: &Path,
    manifest: &RunnerManifest,
) -> RunnerRuntimeLayout {
    let root = if manifest.id == "takokit-python-managed" {
        python_managed_runner_layout(takokit_root).root
    } else {
        let suffix = manifest.id.strip_prefix("takokit-").unwrap_or(&manifest.id);
        takokit_root.join("runners").join(suffix)
    };

    RunnerRuntimeLayout {
        logs: root.join("logs"),
        root,
    }
}

pub fn initialize_runner_runtime(
    takokit_root: &Path,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) -> PackageResult<PullReport> {
    let layout = runner_runtime_layout(takokit_root, manifest);
    std::fs::create_dir_all(&layout.logs)?;

    match manifest.kind {
        RunnerKind::PythonManaged => {
            match install_python_managed_runtime(takokit_root, installed_registry, manifest) {
                Ok(report) => Ok(report),
                Err(error) => {
                    let _ = installed_registry.install_runner_runtime(
                        manifest,
                        RunnerLifecycleState::Failed,
                        format!(
                            "Managed Python runtime install failed: {error}. Logs: {}",
                            layout.logs.display()
                        ),
                    );
                    Err(error)
                }
            }
        }
        RunnerKind::Onnx => match install_onnx_runtime(installed_registry, manifest, &layout) {
            Ok(report) => Ok(report),
            Err(error) => {
                let _ = installed_registry.install_runner_runtime(
                    manifest,
                    RunnerLifecycleState::Failed,
                    format!(
                        "ONNX runtime install failed: {error}. Logs: {}",
                        layout.logs.display()
                    ),
                );
                Err(error)
            }
        },
        RunnerKind::Whispercpp => match install_whispercpp_runtime(installed_registry, manifest, &layout) {
            Ok(report) => Ok(report),
            Err(error) => {
                let _ = installed_registry.install_runner_runtime(
                    manifest,
                    RunnerLifecycleState::Failed,
                    format!(
                        "whisper.cpp runtime install failed: {error}. Logs: {}",
                        layout.logs.display()
                    ),
                );
                Err(error)
            }
        },
        RunnerKind::TransformersAudio => installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::RuntimeInstalled,
            format!(
                "Transformers audio runner runtime directory initialized at {}. Missing component: managed transformers audio adapter.",
                layout.root.display()
            ),
        ),
        RunnerKind::Nemo => installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::RuntimeInstalled,
            format!(
                "NeMo runner runtime directory initialized at {}. Missing component: NeMo adapter and managed dependencies.",
                layout.root.display()
            ),
        ),
        RunnerKind::Native | RunnerKind::External => installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::Failed,
            "No runtime installer is defined for this runner kind.",
        ),
    }
}
