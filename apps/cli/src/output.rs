//! Human-readable and JSON rendering for package, plan, and runner views.

use super::*;

pub(crate) fn print_or_json_plan(plan: &ModelPlan, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(plan)?);
    } else {
        print_model_plan(plan);
    }
    Ok(())
}

pub(crate) fn print_models(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let models: Vec<_> = package_registry
        .models()
        .map_err(cli_error)?
        .into_iter()
        .map(|model| {
            model_info_from_plan(package_registry, installed_registry, &model.id).map_err(cli_error)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    println!("{}", serde_json::to_string_pretty(&models)?);
    Ok(())
}

pub(crate) fn print_runners(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let runners: Vec<_> = package_registry
        .runners()
        .map_err(cli_error)?
        .into_iter()
        .map(|runner| runner.to_runner_info(installed_registry.is_runner_installed(&runner.id)))
        .map(|info| {
            if let Ok(record) = installed_registry.installed_runner_record(&info.id) {
                let manifest = package_registry
                    .runner(&info.id)
                    .expect("runner listed by registry is readable");
                manifest.to_runner_info_with_state(true, record.status)
            } else {
                info
            }
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&runners)?);
    Ok(())
}

pub(crate) fn print_library_models(package_registry: &PackageRegistry) -> anyhow::Result<()> {
    let models = package_registry.library_models().map_err(cli_error)?;
    println!("{}", serde_json::to_string_pretty(&models)?);
    Ok(())
}

pub(crate) fn print_library_runners(package_registry: &PackageRegistry) -> anyhow::Result<()> {
    let runners = package_registry.library_runners().map_err(cli_error)?;
    println!("{}", serde_json::to_string_pretty(&runners)?);
    Ok(())
}

pub(crate) fn print_model_plan(plan: &ModelPlan) {
    println!("Model: {} ({})", plan.model_name, plan.model_id);
    println!("Task: {}", plan.task);
    println!("Runner: {}", plan.required_runner);
    println!("Lifecycle: {:?}", plan.lifecycle_state);
    println!("Artifacts: {:?}", plan.artifact_state);
    println!("Runner contract: {:?}", plan.runner_contract_state);
    println!("Runner runtime: {:?}", plan.runner_runtime_state);
    println!("Executable today: {}", yes_no(plan.executable));
    if plan.missing.is_empty() {
        println!("Missing: none");
    } else {
        println!("Missing: {}", plan.missing.join("; "));
    }
    println!("Next command: {}", plan.next_command);
}

pub(crate) fn print_runner_doctor(
    store: &LocalStore,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) {
    println!("Runner Doctor: {} ({})", manifest.name, manifest.id);
    println!(
        "contract manifest: {}",
        yes_no(installed_registry.is_runner_installed(&manifest.id))
    );
    match installed_registry.installed_runner_record(&manifest.id) {
        Ok(record) => {
            println!("runtime state: {:?}", record.status);
            println!("recorded at: {}", record.installed_at);
            println!("note: {}", record.note);
        }
        Err(_) => println!("runtime state: RuntimeMissing"),
    }
    let layout = runner_runtime_layout(store.root(), manifest);
    println!("runtime path: {}", layout.root.display());
    println!("logs path: {}", layout.logs.display());
    if manifest.id == "takokit-onnx" {
        let ready = installed_registry
            .installed_runner_record(&manifest.id)
            .map(|record| record.status == takokit_package::RunnerLifecycleState::Ready)
            .unwrap_or(false);
        println!(
            "ONNX session capability: {}",
            if ready {
                "kokoro-onnx-ready"
            } else {
                "not-installed"
            }
        );
        println!("Piper runtime: managed by takokit-python-managed");
        println!(
            "executable models: {}",
            if ready { "kokoro" } else { "none" }
        );
    }
    if manifest.id == "takokit-python-managed" {
        match python_adapter_records(store.root()) {
            Ok(records) if !records.is_empty() => {
                println!("adapters:");
                for record in records {
                    println!("- {}: {}", record.id, record.state);
                }
            }
            _ => println!("adapters: run `takokit runner install takokit-python-managed`"),
        }
    }
}

pub(crate) fn print_runner_doctor_json(
    store: &LocalStore,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) -> anyhow::Result<()> {
    let layout = runner_runtime_layout(store.root(), manifest);
    let record = installed_registry
        .installed_runner_record(&manifest.id)
        .ok();
    let adapters = if manifest.id == "takokit-python-managed" {
        python_adapter_records(store.root()).unwrap_or_default()
    } else {
        Vec::new()
    };
    let executable_models = if manifest.id == "takokit-onnx" {
        vec!["kokoro"]
    } else {
        Vec::new()
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "id": manifest.id,
            "name": manifest.name,
            "contract_installed": installed_registry.is_runner_installed(&manifest.id),
            "runtime_state": record
                .as_ref()
                .map(|record| record.status.to_string())
                .unwrap_or_else(|| "runtime-missing".to_string()),
            "note": record.as_ref().map(|record| record.note.clone()),
            "runtime_path": layout.root,
            "logs_path": layout.logs,
            "adapters": adapters,
            "onnx_session_capability": if manifest.id == "takokit-onnx" && record.as_ref().is_some_and(|item| item.status == takokit_package::RunnerLifecycleState::Ready) { Some("kokoro-onnx-ready") } else { None },
            "piper_runtime": if manifest.id == "takokit-onnx" { Some("managed-by-takokit-python-managed") } else { None },
            "executable_models": executable_models,
        }))?
    );
    Ok(())
}

pub(crate) fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

pub(crate) fn cli_error(error: PackageError) -> anyhow::Error {
    runtime_error(TakokitError::from(error))
}

pub(crate) fn runtime_error(error: TakokitError) -> anyhow::Error {
    match error {
        TakokitError::Resolution { code, message } => {
            anyhow::anyhow!("{}: {}", code.as_str(), message)
        }
        error => error.into(),
    }
}

pub(crate) fn capability_labels(capabilities: &[CapabilityKind]) -> String {
    if capabilities.is_empty() {
        return "none".to_string();
    }

    capabilities
        .iter()
        .map(|capability| capability.label())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn format_runner_show(
    manifest: &RunnerManifest,
    installed: bool,
    runtime_state: Option<takokit_package::RunnerLifecycleState>,
    note: Option<String>,
    runtime_path: PathBuf,
) -> String {
    let runtime_state_label = runtime_state
        .map(|state| state.to_string())
        .unwrap_or_else(|| "runtime-missing".to_string());
    let mut lines = Vec::new();
    lines.push(format!("{} ({})", manifest.name, manifest.id));
    lines.push(format!("version: {}", manifest.version));
    lines.push(format!("kind: {:?}", manifest.kind));
    lines.push(format!("platforms: {}", manifest.platforms.join(", ")));
    lines.push(format!(
        "model families: {}",
        if manifest.supported_model_families.is_empty() {
            "none declared".to_string()
        } else {
            manifest.supported_model_families.join(", ")
        }
    ));
    lines.push(format!(
        "tasks: {}",
        capability_labels(&manifest.supported_tasks)
    ));
    lines.push(format!(
        "dependency strategy: {:?}",
        manifest.dependency_strategy
    ));
    lines.push(format!("installed: {}", installed));
    lines.push(format!("runtime state: {runtime_state_label}"));
    lines.push(format!("runtime path: {}", runtime_path.display()));
    lines.push(format!(
        "status: {}",
        runner_status_text(manifest, runtime_state)
    ));
    if manifest.id == "takokit-python-managed" {
        lines.push(
            "user setup: Takokit manages Python, packages, wheels, caches, and logs internally."
                .to_string(),
        );
    }
    if let Some(note) = note {
        lines.push(format!("installed note: {note}"));
    }
    if !manifest.notes.is_empty() {
        lines.push(format!("notes: {}", manifest.notes));
    }
    lines.push(format!("description: {}", manifest.description));
    lines.join("\n")
}

pub(crate) fn runner_status_text(
    manifest: &RunnerManifest,
    runtime_state: Option<takokit_package::RunnerLifecycleState>,
) -> String {
    match runtime_state {
        Some(takokit_package::RunnerLifecycleState::Ready) => "ready".to_string(),
        Some(takokit_package::RunnerLifecycleState::RuntimeInstalled)
            if manifest.id == "takokit-onnx" =>
        {
            "runtime installed; missing Kokoro ONNX TTS execution verification".to_string()
        }
        Some(state) => state.to_string(),
        None => "runtime missing".to_string(),
    }
}
