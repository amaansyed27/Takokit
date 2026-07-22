//! The single lifecycle for CLI, daemon, and GUI model pulls.

use crate::{
    artifact_reuse::{self, ArtifactReuseState},
    planning::has_verified_executor,
    runtime_model_source::{estimate_model_source_bytes, model_source_staging_path},
    runtime_python::{prefetch_python_adapter_model, python_adapter_is_current},
    transaction::ModelInstallSnapshot,
    *,
};
use std::path::Path;
use takokit_core::{InstallStep, InstallStepState, ModelInstallReport};

pub fn install_model_complete(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    takokit_root: &Path,
    model_id: &str,
    options: InstallModelOptions,
) -> PackageResult<ModelInstallReport> {
    let progress = InstallProgressReporter::model(takokit_root, model_id);
    let result = install_model_complete_inner(
        package_registry,
        installed_registry,
        takokit_root,
        model_id,
        options,
        &progress,
    );
    match &result {
        Ok(report) => progress.complete(format!("{} is ready", report.model_id)),
        Err(error) => progress.fail(error.to_string()),
    }
    result
}

fn install_model_complete_inner(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    takokit_root: &Path,
    model_id: &str,
    options: InstallModelOptions,
    progress: &InstallProgressReporter,
) -> PackageResult<ModelInstallReport> {
    progress.update("resolving", "Resolving model and runner", 0, None);
    let model = package_registry.model(model_id)?;
    let runner = package_registry.runner(&model.runner)?;
    let logs_path = takokit_root.join("logs");

    if options.metadata_only {
        progress.update("metadata", "Installing model metadata", 0, None);
        let artifacts_before = artifact_reuse::classify(
            installed_registry
                .installed_model_record(&model.id)
                .ok()
                .as_ref(),
            &model,
        );
        let report = installed_registry.install_model_with_options(&model, options)?;
        let plan = plan_model(package_registry, installed_registry, model_id)?;
        return Ok(ModelInstallReport {
            model_id: model.id,
            required_runner: runner.id,
            required_adapter: model.required_adapter,
            artifacts: InstallStep {
                state: if artifacts_before == ArtifactReuseState::Verified {
                    InstallStepState::AlreadyReady
                } else {
                    InstallStepState::MetadataOnly
                },
                newly_installed: artifacts_before == ArtifactReuseState::Missing
                    && report.installed,
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

    progress.update("runner-contract", "Installing runner contract", 0, None);
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

    let runtime_ready_before = installed_registry
        .installed_runner_record(&runner.id)
        .ok()
        .is_some_and(|record| record.status == RunnerLifecycleState::Ready);
    if !runtime_ready_before {
        let runtime_layout = runner_runtime_layout(takokit_root, &runner);
        let monitor = InstallProgressMonitor::start(
            progress.clone(),
            vec![runtime_layout.root],
            "runner-runtime",
            format!("Installing {} runtime", runner.id),
            None,
        );
        let result = initialize_runner_runtime(takokit_root, installed_registry, &runner)
            .map_err(|error| PackageError::at_stage(InstallFailureStage::RunnerRuntime, error));
        drop(monitor);
        result?;
    } else {
        progress.update(
            "runner-runtime",
            format!("{} runtime is already ready", runner.id),
            0,
            None,
        );
    }
    let runtime_state = installed_registry
        .installed_runner_record(&runner.id)
        .map_err(|error| PackageError::at_stage(InstallFailureStage::RunnerRuntime, error))?
        .status;
    if runtime_state != RunnerLifecycleState::Ready {
        return Err(PackageError::at_stage(
            InstallFailureStage::RunnerRuntime,
            PackageError::ArtifactInstallFailed {
                artifact: runner.id.clone(),
                reason: format!(
                    "runner initialization completed without reaching ready state: {runtime_state}"
                ),
            },
        ));
    }
    let runner_runtime = InstallStep {
        state: if runtime_ready_before {
            InstallStepState::AlreadyReady
        } else {
            InstallStepState::Installed
        },
        newly_installed: !runtime_ready_before,
        detail: runner_runtime_layout(takokit_root, &runner)
            .logs
            .display()
            .to_string(),
    };

    let (adapter, adapter_ready) = if let Some(adapter_id) = model.required_adapter.as_deref() {
        let ready_before = python_adapter_is_current(takokit_root, adapter_id);
        if !ready_before {
            let adapter_path = python_managed_runner_layout(takokit_root)
                .adapters
                .join(adapter_id);
            let monitor = InstallProgressMonitor::start(
                progress.clone(),
                vec![adapter_path],
                "adapter",
                format!("Installing {adapter_id} dependencies"),
                None,
            );
            let result = install_python_adapter(takokit_root, adapter_id)
                .map_err(|error| PackageError::at_stage(InstallFailureStage::Adapter, error));
            drop(monitor);
            result?;
        } else {
            progress.update(
                "adapter",
                format!("{adapter_id} adapter is already ready"),
                0,
                None,
            );
        }
        let state = python_adapter_record(takokit_root, adapter_id)
            .map_err(|error| PackageError::at_stage(InstallFailureStage::Adapter, error))?
            .state;
        if state != AdapterLifecycleState::Ready {
            return Err(PackageError::at_stage(
                InstallFailureStage::Adapter,
                PackageError::ArtifactInstallFailed {
                    artifact: adapter_id.to_string(),
                    reason: format!(
                        "adapter installation completed without reaching ready state: {state}"
                    ),
                },
            ));
        }
        (
            Some(InstallStep {
                state: if ready_before {
                    InstallStepState::AlreadyReady
                } else {
                    InstallStepState::Installed
                },
                newly_installed: !ready_before,
                detail: python_managed_runner_layout(takokit_root)
                    .adapters
                    .join(adapter_id)
                    .join("install.log")
                    .display()
                    .to_string(),
            }),
            true,
        )
    } else {
        (None, false)
    };

    if !has_verified_executor(&model, &runner, adapter_ready) {
        return Err(PackageError::at_stage(
            InstallFailureStage::FinalVerification,
            PackageError::ArtifactInstallFailed {
                artifact: model.id.clone(),
                reason: format!(
                    "no verified executor is implemented for model {} on runner {}",
                    model.id, runner.id
                ),
            },
        ));
    }

    let previous_record = installed_registry.installed_model_record(&model.id).ok();
    let reuse_state = artifact_reuse::classify(previous_record.as_ref(), &model);
    let snapshot = ModelInstallSnapshot::capture(installed_registry, &model.id)
        .map_err(|error| PackageError::at_stage(InstallFailureStage::Materialization, error))?;
    let download_total = if reuse_state == ArtifactReuseState::Verified {
        None
    } else {
        progress.update("planning-download", "Calculating download size", 0, None);
        estimated_download_total(takokit_root, &model)
    };
    let monitor = InstallProgressMonitor::start(
        progress.clone(),
        vec![
            takokit_root.join("cache").join("downloads"),
            model_source_staging_path(takokit_root, &model.id),
        ],
        if reuse_state == ArtifactReuseState::Verified {
            "verifying-cache"
        } else {
            "model-download"
        },
        if reuse_state == ArtifactReuseState::Verified {
            "Verifying cached model files"
        } else {
            "Downloading and materializing model files"
        },
        download_total,
    );
    let artifact_result = installed_registry
        .install_model_with_options(&model, options)
        .map_err(|error| PackageError::at_stage(InstallFailureStage::Artifacts, error));
    drop(monitor);
    let artifact_report = artifact_result?;
    let mut artifact_detail = artifact_report.note;
    if let Some(adapter_id) = model.required_adapter.as_deref() {
        let prefetch_monitor = InstallProgressMonitor::start(
            progress.clone(),
            vec![
                takokit_root.join("cache"),
                takokit_root.join("models").join(&model.id),
            ],
            "model-prefetch",
            format!("Acquiring {} checkpoint files", model.id),
            None,
        );
        let prefetch = prefetch_python_adapter_model(takokit_root, &model, adapter_id)
            .map_err(|error| PackageError::at_stage(InstallFailureStage::Artifacts, error));
        drop(prefetch_monitor);
        if let Some(note) = prefetch? {
            installed_registry
                .mark_runtime_model_ready(&model.id, &note)
                .map_err(|error| PackageError::at_stage(InstallFailureStage::Artifacts, error))?;
            artifact_detail = note;
        }
    }
    let artifacts = InstallStep {
        state: match reuse_state {
            ArtifactReuseState::Verified => InstallStepState::AlreadyReady,
            ArtifactReuseState::RepairRequired => InstallStepState::Repaired,
            ArtifactReuseState::Missing => InstallStepState::Installed,
        },
        newly_installed: reuse_state != ArtifactReuseState::Verified && artifact_report.installed,
        detail: artifact_detail,
    };

    progress.update(
        "final-verification",
        "Verifying the completed execution plan",
        download_total.unwrap_or(0),
        download_total,
    );
    let plan = match plan_model(package_registry, installed_registry, model_id) {
        Ok(plan) => plan,
        Err(error) => {
            return Err(rollback_final_verification(
                snapshot,
                installed_registry,
                model_id,
                error,
            ))
        }
    };
    if !plan.executable {
        return Err(rollback_final_verification(
            snapshot,
            installed_registry,
            model_id,
            PackageError::ArtifactInstallFailed {
                artifact: model.id.clone(),
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

fn estimated_download_total(takokit_root: &Path, model: &ModelManifest) -> Option<u64> {
    let artifact_total = model.artifacts.all().try_fold(0_u64, |total, artifact| {
        artifact.bytes.and_then(|bytes| total.checked_add(bytes))
    })?;
    let source_total = if model.source.is_some() {
        estimate_model_source_bytes(takokit_root, model)?
    } else {
        0
    };
    artifact_total
        .checked_add(source_total)
        .filter(|total| *total > 0)
}

fn rollback_final_verification(
    snapshot: ModelInstallSnapshot,
    installed_registry: &InstalledRegistry,
    model_id: &str,
    source: PackageError,
) -> PackageError {
    match snapshot.restore(installed_registry, model_id) {
        Ok(()) => PackageError::at_stage(InstallFailureStage::FinalVerification, source),
        Err(rollback_error) => PackageError::at_stage(
            InstallFailureStage::FinalVerification,
            PackageError::ArtifactInstallFailed {
                artifact: model_id.to_string(),
                reason: format!("{source}; rollback also failed: {rollback_error}"),
            },
        ),
    }
}
