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
    print_serializable(&models)
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
    print_serializable(&runners)
}

pub(crate) fn print_library_models(package_registry: &PackageRegistry) -> anyhow::Result<()> {
    let models = package_registry.library_models().map_err(cli_error)?;
    print_serializable(&models)
}

pub(crate) fn print_library_runners(package_registry: &PackageRegistry) -> anyhow::Result<()> {
    let runners = package_registry.library_runners().map_err(cli_error)?;
    print_serializable(&runners)
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

pub(crate) fn print_serializable<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    print_value(&serde_json::to_value(value)?)
}

pub(crate) fn print_value(value: &serde_json::Value) -> anyhow::Result<()> {
    if json_requested() {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        render_value(value, 0);
    }
    Ok(())
}

fn json_requested() -> bool {
    std::env::var("TAKOKIT_OUTPUT")
        .map(|value| value.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn render_value(value: &serde_json::Value, depth: usize) {
    if let Some(data) = value.get("data") {
        render_value(data, depth);
        return;
    }
    match value {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                println!("No entries.");
            } else {
                for item in items {
                    render_row(item);
                }
            }
        }
        serde_json::Value::Object(map) => {
            if map.contains_key("model_id") && map.contains_key("artifacts") {
                render_pull(map);
                return;
            }
            if map.contains_key("output_path") {
                render_output(map);
                return;
            }
            if map.contains_key("instance_id") && map.contains_key("pid") {
                println!("Daemon running");
                render_field("pid", map.get("pid"));
                println!(
                    "  {:<12} {}:{}",
                    "address",
                    text(map, "host"),
                    scalar(map.get("port"))
                );
                render_field("executable", map.get("executable"));
                render_field("storage", map.get("storage_root"));
                return;
            }
            if let Some(removed) = map.get("removed").and_then(|value| value.as_bool()) {
                println!(
                    "{} {}",
                    if removed { "Removed" } else { "Not installed:" },
                    text(map, "id")
                );
                return;
            }
            if let Some(summary) = map.get("summary") {
                render_row(summary);
                return;
            }
            for (key, item) in map {
                if item.is_null() {
                    continue;
                }
                match item {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        println!("{}{}:", "  ".repeat(depth), label(key));
                        render_value(item, depth + 1);
                    }
                    _ => println!(
                        "{}{:<14} {}",
                        "  ".repeat(depth),
                        label(key),
                        scalar(Some(item))
                    ),
                }
            }
        }
        _ => println!("{}", scalar(Some(value))),
    }
}

fn render_pull(map: &serde_json::Map<String, serde_json::Value>) {
    println!("Model {}", text(map, "model_id"));
    render_stage(map, "artifacts", "artifacts");
    render_stage(map, "runner_contract", "runner");
    render_stage(map, "runner_runtime", "runtime");
    render_stage(map, "adapter", "adapter");
    println!(
        "  {:<12} {}",
        "ready",
        yes_no(
            map.get("executable")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        )
    );
    if let Some(path) = map.get("logs_path").and_then(|value| value.as_str()) {
        println!("  {:<12} {path}", "logs");
    }
}

fn render_output(map: &serde_json::Map<String, serde_json::Value>) {
    let output = text(map, "output_path");
    if let Some(body) = map.get("text").and_then(|value| value.as_str()) {
        println!("Transcription complete");
        if !text(map, "model").is_empty() {
            println!("  {:<12} {}", "model", text(map, "model"));
        }
        println!("\n{body}");
        if !output.is_empty() {
            println!("\nSaved to {output}");
        }
    } else {
        println!("Audio ready");
        if !text(map, "model").is_empty() {
            println!("  {:<12} {}", "model", text(map, "model"));
        }
        if !text(map, "engine").is_empty() {
            println!("  {:<12} {}", "engine", text(map, "engine"));
        }
        if let Some(bytes) = map.get("bytes").and_then(|value| value.as_u64()) {
            println!("  {:<12} {}", "size", bytes_label(bytes));
        }
        if !output.is_empty() {
            println!("  {:<12} {output}", "output");
        }
    }
}

fn render_row(value: &serde_json::Value) {
    let Some(map) = value.as_object() else {
        println!("{}", scalar(Some(value)));
        return;
    };
    let primary = ["name", "title", "id", "model_id"]
        .iter()
        .find_map(|key| map.get(*key).and_then(|value| value.as_str()))
        .unwrap_or("entry");
    print!("{primary}");
    if let Some(id) = map
        .get("id")
        .and_then(|value| value.as_str())
        .filter(|id| *id != primary)
    {
        print!("  {id}");
    }
    if let Some(state) = ["status", "state", "lifecycle_state", "runtime_state"]
        .iter()
        .find_map(|key| map.get(*key).and_then(|value| value.as_str()))
    {
        print!("  [{state}]");
    }
    if let Some(model) = map.get("last_model").and_then(|value| value.as_str()) {
        print!("  model={model}");
    }
    if let Some(count) = map.get("event_count").and_then(|value| value.as_u64()) {
        print!("  events={count}");
    }
    println!();
}

fn render_stage(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    name: &str,
) {
    let Some(item) = map.get(key) else {
        return;
    };
    if item.is_null() {
        return;
    }
    let state = item
        .get("state")
        .and_then(|value| value.as_str())
        .unwrap_or("ready");
    println!("  {name:<12} {state}");
    if let Some(detail) = item
        .get("detail")
        .and_then(|value| value.as_str())
        .filter(|detail| *detail != state)
    {
        println!("               {detail}");
    }
}

fn render_field(name: &str, value: Option<&serde_json::Value>) {
    println!("  {name:<12} {}", scalar(value));
}

fn text(map: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    map.get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

fn scalar(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(value)) => value.clone(),
        Some(serde_json::Value::Bool(value)) => value.to_string(),
        Some(serde_json::Value::Number(value)) => value.to_string(),
        Some(serde_json::Value::Null) | None => "-".to_string(),
        Some(value) => value.to_string(),
    }
}

fn label(value: &str) -> String {
    value.replace('_', " ")
}

fn bytes_label(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
