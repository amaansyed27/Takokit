//! The single lifecycle for CLI, daemon, and GUI model pulls.

use crate::*;
use std::path::Path;
use takokit_core::{InstallStep, InstallStepState, ModelInstallReport};

pub fn install_model_complete(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    takokit_root: &Path,
    model_id: &str,
    options: InstallModelOptions,
) -> PackageResult<ModelInstallReport> {
    let model = package_registry.model(model_id)?;
    let runner = package_registry.runner(&model.runner)?;
    let logs_path = takokit_root.join("logs");
    if options.metadata_only {
        let ready_before = installed_registry
            .installed_model_record(&model.id)
            .ok()
            .is_some_and(|record| record.status == InstalledPackageStatus::Ready);
        let report = installed_registry.install_model_with_options(&model, options)?;
        let plan = plan_model(package_registry, installed_registry, model_id)?;
        return Ok(ModelInstallReport {
            model_id: model.id,
            required_runner: runner.id,
            required_adapter: model.required_adapter,
            artifacts: InstallStep {
                state: if ready_before {
                    InstallStepState::AlreadyReady
                } else {
                    InstallStepState::MetadataOnly
                },
                newly_installed: !ready_before && report.installed,
                detail: report.note,
            },
            runner_contract: InstallStep {
                state: InstallStepState::NotRequested,
                newly_installed: false,
                detail: "metadata-only pull does not install runner contracts".into(),
            },
            runner_runtime: InstallStep {
                state: InstallStepState::NotRequested,
                newly_installed: false,
                detail: "metadata-only pull does not initialize runtimes".into(),
            },
            adapter: None,
            executable: plan.executable,
            missing: plan.missing,
            logs_path,
        });
    }
    let was_contract = installed_registry.is_runner_installed(&runner.id);
    if !was_contract {
        installed_registry
            .install_runner(&runner)
            .map_err(|error| PackageError::at_stage(InstallFailureStage::RunnerContract, error))?;
    }
    let runner_contract = InstallStep {
        state: if was_contract {
            InstallStepState::AlreadyReady
        } else {
            InstallStepState::Installed
        },
        newly_installed: !was_contract,
        detail: runner.id.clone(),
    };
    let runtime_ready = installed_registry
        .installed_runner_record(&runner.id)
        .ok()
        .is_some_and(|record| record.status == RunnerLifecycleState::Ready);
    if !runtime_ready {
        initialize_runner_runtime(takokit_root, installed_registry, &runner)
            .map_err(|error| PackageError::at_stage(InstallFailureStage::RunnerRuntime, error))?;
    }
    let runner_runtime = InstallStep {
        state: if runtime_ready {
            InstallStepState::AlreadyReady
        } else {
            InstallStepState::Installed
        },
        newly_installed: !runtime_ready,
        detail: runner_runtime_layout(takokit_root, &runner)
            .logs
            .display()
            .to_string(),
    };
    let adapter = if let Some(adapter_id) = model.required_adapter.as_deref() {
        let ready = python_adapter_record(takokit_root, adapter_id)
            .ok()
            .is_some_and(|record| record.state == AdapterLifecycleState::Ready);
        if !ready {
            install_python_adapter(takokit_root, adapter_id)
                .map_err(|error| PackageError::at_stage(InstallFailureStage::Adapter, error))?;
        }
        let state = python_adapter_record(takokit_root, adapter_id)?.state;
        Some(InstallStep {
            state: if ready {
                InstallStepState::AlreadyReady
            } else if state == AdapterLifecycleState::Ready {
                InstallStepState::Installed
            } else {
                InstallStepState::Failed
            },
            newly_installed: !ready,
            detail: python_managed_runner_layout(takokit_root)
                .adapters
                .join(adapter_id)
                .join("install.log")
                .display()
                .to_string(),
        })
    } else {
        None
    };
    let artifacts_ready = installed_registry
        .installed_model_record(&model.id)
        .ok()
        .is_some_and(|record| record.status == InstalledPackageStatus::Ready);
    let artifact_report = installed_registry
        .install_model_with_options(&model, options)
        .map_err(|error| PackageError::at_stage(InstallFailureStage::Artifacts, error))?;
    let artifacts = InstallStep {
        state: if artifacts_ready {
            InstallStepState::AlreadyReady
        } else {
            InstallStepState::Installed
        },
        newly_installed: !artifacts_ready && artifact_report.installed,
        detail: artifact_report.note,
    };
    let plan = plan_model(package_registry, installed_registry, model_id)?;
    if !plan.executable {
        return Err(PackageError::at_stage(
            InstallFailureStage::FinalVerification,
            PackageError::ArtifactInstallFailed {
                artifact: model.id,
                reason: format!(
                    "final execution-plan verification failed: {}",
                    plan.missing.join("; ")
                ),
            },
        ));
    }
    Ok(ModelInstallReport {
        model_id: model.id,
        required_runner: runner.id,
        required_adapter: model.required_adapter,
        artifacts,
        runner_contract,
        runner_runtime,
        adapter,
        executable: true,
        missing: Vec::new(),
        logs_path,
    })
}
