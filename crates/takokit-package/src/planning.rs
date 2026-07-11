//! Pure model lifecycle planning and user-facing status text.

use crate::{artifact_reuse, *};
use takokit_core::CapabilityKind;

pub(crate) fn model_artifact_state(
    model: &ModelManifest,
    installed_model: Option<&InstalledModelRecord>,
) -> ModelLifecycleState {
    if installed_model.is_some_and(|record| artifact_reuse::all_verified(record, model)) {
        ModelLifecycleState::ArtifactsReady
    } else {
        ModelLifecycleState::MetadataOnly
    }
}

pub(crate) fn model_lifecycle_state(
    model: &ModelManifest,
    runner: &RunnerManifest,
    artifact_state: ModelLifecycleState,
    runner_runtime_state: RunnerLifecycleState,
    adapter_ready: bool,
) -> ModelLifecycleState {
    if runner_runtime_state == RunnerLifecycleState::Failed {
        return ModelLifecycleState::Failed;
    }
    if artifact_state == ModelLifecycleState::MetadataOnly {
        return ModelLifecycleState::MetadataOnly;
    }
    if runner_runtime_state == RunnerLifecycleState::Ready {
        if has_verified_executor(model, runner, adapter_ready) {
            ModelLifecycleState::Executable
        } else {
            ModelLifecycleState::RunnerReady
        }
    } else {
        ModelLifecycleState::ArtifactsReady
    }
}

pub(crate) fn has_verified_executor(
    model: &ModelManifest,
    runner: &RunnerManifest,
    adapter_ready: bool,
) -> bool {
    matches!(
        (runner.kind.clone(), model.id.as_str()),
        (RunnerKind::Onnx, "kokoro")
    ) || (runner.kind == RunnerKind::PythonManaged && model.id == "qwen3-tts" && adapter_ready)
        || (runner.kind == RunnerKind::Whispercpp
            && model.family.eq_ignore_ascii_case("whisper")
            && model.capabilities.stt)
}

pub(crate) fn model_task_label(model: &ModelManifest) -> String {
    capability_labels(&model.capabilities.to_model_capabilities())
}

pub(crate) fn runner_missing_component(model: &ModelManifest, runner: &RunnerManifest) -> String {
    if runner.kind == RunnerKind::Onnx && model.id == "piper-lessac" {
        return "Piper text frontend (phonemizer/token preparation)".to_string();
    }
    match runner.kind {
        RunnerKind::Onnx if model.id == "kokoro" => {
            "managed kokoro-onnx runtime (run `takokit runner install takokit-onnx`)".to_string()
        }
        RunnerKind::Onnx => "ONNX inference implementation".to_string(),
        RunnerKind::Whispercpp => "whisper.cpp transcription implementation".to_string(),
        RunnerKind::PythonManaged if model.required_adapter.is_some() => format!(
            "{} managed adapter (run `takokit adapter install {}`)",
            model
                .required_adapter
                .as_deref()
                .unwrap_or("required Python"),
            model.required_adapter.as_deref().unwrap_or("adapter")
        ),
        RunnerKind::PythonManaged => "verified artifacts and managed runtime adapter".to_string(),
        RunnerKind::TransformersAudio => "Transformers audio runtime adapter".to_string(),
        RunnerKind::Nemo => "NeMo runtime adapter".to_string(),
        RunnerKind::External => "external runner adapter".to_string(),
        RunnerKind::Native => "native runner implementation".to_string(),
    }
}

pub(crate) fn model_execution_status(plan: &ModelPlan) -> String {
    if plan.executable {
        return "executable".to_string();
    }
    match plan.lifecycle_state {
        ModelLifecycleState::MetadataOnly => {
            format!("metadata-only; missing {}", plan.missing.join("; "))
        }
        ModelLifecycleState::ArtifactsReady => {
            format!("artifacts ready; missing {}", plan.missing.join("; "))
        }
        ModelLifecycleState::RunnerReady => {
            format!("runner ready; missing {}", plan.missing.join("; "))
        }
        ModelLifecycleState::Executable => "executable".to_string(),
        ModelLifecycleState::Failed => format!("failed; run {}", plan.next_command),
    }
}

pub(crate) fn license_warning(license: &str) -> Option<String> {
    let value = license.to_ascii_lowercase();
    if value.contains("non-commercial") || value.contains("cc-by-nc") || value.contains("nc") {
        Some("Non-commercial or restricted license; do not treat as commercial-safe.".to_string())
    } else if value.contains("gpl") {
        Some("GPL/runtime boundary review required before bundling or auto-install.".to_string())
    } else if value.contains("unknown") || value.contains("check-required") {
        Some("License requires review before supported runtime installation.".to_string())
    } else {
        None
    }
}

pub(crate) fn next_plan_command(
    model: &ModelManifest,
    model_installed: bool,
    runner_runtime_state: RunnerLifecycleState,
    adapter: Option<(&str, AdapterLifecycleState)>,
    executable: bool,
) -> String {
    if !model_installed {
        format!("takokit pull {}", model.id)
    } else if runner_runtime_state == RunnerLifecycleState::RuntimeMissing {
        format!("takokit runner pull {}", model.runner)
    } else if runner_runtime_state == RunnerLifecycleState::ContractInstalled {
        format!("takokit runner install {}", model.runner)
    } else if runner_runtime_state == RunnerLifecycleState::Failed {
        format!("takokit runner doctor {}", model.runner)
    } else if let Some((adapter, state)) = adapter {
        if state != AdapterLifecycleState::Ready {
            format!("takokit adapter install {adapter}")
        } else if executable {
            format!("takokit test {}", model.id)
        } else {
            format!("takokit runner doctor {}", model.runner)
        }
    } else if executable {
        format!("takokit test {}", model.id)
    } else {
        format!("takokit runner doctor {}", model.runner)
    }
}

fn capability_labels(capabilities: &[CapabilityKind]) -> String {
    if capabilities.is_empty() {
        return "none".to_string();
    }
    capabilities
        .iter()
        .map(|capability| capability.label())
        .collect::<Vec<_>>()
        .join(" / ")
}
