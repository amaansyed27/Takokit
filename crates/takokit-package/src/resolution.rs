//! Canonical execution resolution and planner-backed model lifecycle views.

use crate::{
    planning::{
        model_artifact_state, model_lifecycle_state, model_task_label, next_plan_command,
        runner_missing_component,
    },
    *,
};
use takokit_core::{CapabilityKind, ModelInfo};

pub fn resolve_execution_plan(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
    capability: CapabilityKind,
) -> PackageResult<ExecutionPlan> {
    let model = package_registry.model(model_id)?;
    let installed_model = if model.id == "mock-tts" {
        None
    } else if installed_registry.is_model_installed(&model.id) {
        Some(installed_registry.installed_model_record(&model.id)?)
    } else {
        return Err(PackageError::ModelNotInstalled(model.id));
    };

    if !model.supports(capability) {
        return Err(PackageError::CapabilityUnsupported {
            model: model.id,
            capability,
            capability_label: capability.label(),
        });
    }

    let runner = package_registry.runner(&model.runner)?;
    let platform = current_platform_id();
    if !runner
        .platforms
        .iter()
        .any(|item| item == &platform || item == "any")
    {
        return Err(PackageError::RunnerUnsupportedOnPlatform {
            model: model.id,
            runner: runner.id,
            capability,
            capability_label: capability.label(),
            platform,
        });
    }

    let runner_installed = installed_registry.is_runner_installed(&runner.id);
    if !runner_installed {
        return Err(PackageError::RunnerNotInstalled {
            model: model.id,
            runner: runner.id,
            capability,
            capability_label: capability.label(),
        });
    }

    Ok(ExecutionPlan {
        model,
        capability,
        runner,
        runner_installed,
        status: ExecutionStatus::Planned,
        installed_model,
    })
}

pub fn resolve_runner(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
    capability: CapabilityKind,
) -> PackageResult<ExecutionPlan> {
    resolve_execution_plan(package_registry, installed_registry, model_id, capability)
}

pub fn model_info_from_plan(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
) -> PackageResult<ModelInfo> {
    let model = package_registry.model(model_id)?;
    let plan = plan_model(package_registry, installed_registry, model_id)?;
    Ok(model.to_model_info_from_plan(
        &plan,
        installed_registry.is_model_installed(&model.id),
        installed_registry.is_runner_installed(&model.runner),
    ))
}

pub fn plan_model(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
) -> PackageResult<ModelPlan> {
    let model = package_registry.model(model_id)?;
    let runner = package_registry.runner(&model.runner)?;
    let installed_model = installed_registry.installed_model_record(&model.id).ok();
    let installed_runner = installed_registry.installed_runner_record(&runner.id).ok();

    let artifact_state = model_artifact_state(&model, installed_model.as_ref());
    let runner_contract_state = if installed_runner.is_some() {
        RunnerLifecycleState::ContractInstalled
    } else {
        RunnerLifecycleState::RuntimeMissing
    };
    let runner_runtime_state = installed_runner
        .as_ref()
        .map(|record| record.status)
        .unwrap_or(RunnerLifecycleState::RuntimeMissing);
    let adapter = model.required_adapter.as_deref().and_then(|id| {
        python_adapter_record(&installed_registry.storage_root(), id)
            .ok()
            .map(|record| (id, record.state))
    });
    let adapter_ready = adapter
        .as_ref()
        .is_some_and(|(_, state)| *state == AdapterLifecycleState::Ready);
    let lifecycle_state = model_lifecycle_state(
        &model,
        &runner,
        artifact_state,
        runner_runtime_state,
        adapter_ready,
    );
    let executable = lifecycle_state == ModelLifecycleState::Executable;
    let mut missing = Vec::new();

    if matches!(artifact_state, ModelLifecycleState::MetadataOnly) {
        missing.push("verified artifacts".to_string());
    }
    if runner_contract_state == RunnerLifecycleState::RuntimeMissing {
        missing.push(format!("runner contract: {}", runner.id));
    }
    if runner_runtime_state != RunnerLifecycleState::Ready {
        missing.push(runner_missing_component(&model, &runner));
    }
    if let Some((adapter, state)) = adapter.as_ref() {
        if *state != AdapterLifecycleState::Ready {
            missing.push(format!("managed adapter {adapter} ({state})"));
        }
    }
    if lifecycle_state == ModelLifecycleState::RunnerReady {
        missing.push(runner_missing_component(&model, &runner));
    }
    if executable {
        missing.clear();
    }

    Ok(ModelPlan {
        model_id: model.id.clone(),
        model_name: model.name.clone(),
        family: model.family.clone(),
        task: model_task_label(&model),
        required_runner: runner.id.clone(),
        lifecycle_state,
        artifact_state,
        runner_contract_state,
        runner_runtime_state,
        executable,
        missing,
        next_command: next_plan_command(
            &model,
            installed_model.is_some(),
            runner_runtime_state,
            adapter,
            executable,
        ),
    })
}

pub fn current_platform_id() -> String {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        std::env::consts::OS
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        std::env::consts::ARCH
    };

    format!("{os}-{arch}")
}
