use takokit_package::{plan_model, InstalledRegistry, PackageRegistry};

#[derive(Debug, Clone)]
pub struct ModelRow {
    pub id: String,
    pub title: String,
    pub model_type: String,
    pub state: String,
    pub detail: String,
    pub tts: bool,
    pub stt: bool,
    pub voice_cloning: bool,
    pub executable: bool,
}

#[derive(Debug, Clone)]
pub struct RunnerRow {
    pub id: String,
    pub title: String,
    pub state: String,
    pub detail: String,
    pub installed: bool,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAction {
    Status,
    Doctor,
    StartDaemon,
    StopDaemon,
    RestartDaemon,
    Logs,
    OpenGui,
}

#[derive(Debug, Clone)]
pub struct SystemRow {
    pub title: &'static str,
    pub state: &'static str,
    pub detail: &'static str,
    pub action: SystemAction,
}

pub fn load_runtime_rows(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<(Vec<ModelRow>, Vec<RunnerRow>)> {
    let inventory = installed_registry.installed_model_inventory(package_registry)?;
    let models = inventory
        .data
        .into_iter()
        .map(|installed| {
            let model = package_registry.model(&installed.name)?;
            let plan = plan_model(package_registry, installed_registry, &model.id)?;
            let action = if plan.executable {
                "Ready to use. Press Enter to open the matching task screen.".to_string()
            } else {
                format!(
                    "Model files are installed, but the runtime needs repair. Press Enter or P to repair it.\nMissing: {}",
                    if plan.missing.is_empty() {
                        "runtime setup".to_string()
                    } else {
                        plan.missing.join("; ")
                    }
                )
            };
            let runtime_state = if plan.executable {
                "ready"
            } else {
                "needs repair"
            };
            Ok(ModelRow {
                id: model.id,
                title: model.name,
                model_type: installed.model_type.clone(),
                state: format!("{} · {runtime_state}", installed.model_type),
                detail: format!(
                    "{}\n\nType: {}\nFamily: {}\nRunner: {}\nLocal ID: {}\nStored size: {}\nHardware: {}\n\n{}",
                    model.description,
                    installed.model_type,
                    model.family,
                    plan.required_runner,
                    installed.id,
                    format_size(installed.size_bytes),
                    model
                        .hardware
                        .min_ram
                        .as_deref()
                        .unwrap_or("no minimum listed"),
                    action
                ),
                tts: model.capabilities.tts,
                stt: model.capabilities.stt,
                voice_cloning: model.capabilities.voice_cloning,
                executable: plan.executable,
            })
        })
        .collect::<Result<Vec<_>, takokit_package::PackageError>>()?;

    let runners = package_registry
        .runners()?
        .into_iter()
        .map(|runner| {
            let record = installed_registry.installed_runner_record(&runner.id).ok();
            let state = record
                .as_ref()
                .map(|record| record.status.to_string())
                .unwrap_or_else(|| "available".to_string());
            let ready = state == "ready";
            RunnerRow {
                id: runner.id,
                title: runner.name,
                state: state.clone(),
                detail: format!(
                    "{}\n\nVersion: {}\nPlatforms: {}\nModel families: {}\nState: {}\n\n{}",
                    runner.description,
                    runner.version,
                    runner.platforms.join(", "),
                    runner.supported_model_families.join(", "),
                    state,
                    if ready {
                        "Ready. Press Enter to run its diagnostic check."
                    } else if record.is_some() {
                        "The runner contract exists. Press Enter to install its runtime."
                    } else {
                        "Press Enter to add this runner."
                    }
                ),
                installed: record.is_some(),
                ready,
            }
        })
        .collect();
    Ok((models, runners))
}

pub fn capability_indexes(models: &[ModelRow]) -> (Vec<usize>, Vec<usize>) {
    let tts = models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| model.tts.then_some(index))
        .collect();
    let stt = models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| model.stt.then_some(index))
        .collect();
    (tts, stt)
}

pub fn find_model_index(models: &[ModelRow], id: Option<&str>) -> usize {
    id.and_then(|id| models.iter().position(|model| model.id == id))
        .unwrap_or(0)
}

pub fn find_runner_index(runners: &[RunnerRow], id: Option<&str>) -> usize {
    id.and_then(|id| runners.iter().position(|runner| runner.id == id))
        .unwrap_or(0)
}

pub fn find_capability_index(
    models: &[ModelRow],
    indexes: &[usize],
    selected: Option<&str>,
    preferred: &str,
) -> usize {
    selected
        .and_then(|id| indexes.iter().position(|index| models[*index].id == id))
        .or_else(|| {
            indexes
                .iter()
                .position(|index| models[*index].id == preferred)
        })
        .unwrap_or(0)
}

pub fn system_rows() -> Vec<SystemRow> {
    vec![
        SystemRow {
            title: "Runtime status",
            state: "read",
            detail: "Check the daemon, storage, and currently available runtime state.",
            action: SystemAction::Status,
        },
        SystemRow {
            title: "Doctor",
            state: "diagnostics",
            detail: "Run the complete local setup and model readiness check.",
            action: SystemAction::Doctor,
        },
        SystemRow {
            title: "Start daemon",
            state: "service",
            detail: "Start Takokit's managed local API service.",
            action: SystemAction::StartDaemon,
        },
        SystemRow {
            title: "Stop daemon",
            state: "service",
            detail: "Stop the managed local API service.",
            action: SystemAction::StopDaemon,
        },
        SystemRow {
            title: "Restart daemon",
            state: "service",
            detail: "Restart the managed local API service.",
            action: SystemAction::RestartDaemon,
        },
        SystemRow {
            title: "View logs",
            state: "diagnostics",
            detail: "Show the latest daemon log location and output.",
            action: SystemAction::Logs,
        },
        SystemRow {
            title: "Open GUI",
            state: "interface",
            detail: "Open the browser GUI in this same project session.",
            action: SystemAction::OpenGui,
        },
    ]
}

fn format_size(bytes: u64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_store_has_no_tui_model_rows() {
        let temp = tempfile::tempdir().expect("tempdir");
        let package_registry = PackageRegistry::bundled();
        let installed_registry = InstalledRegistry::new(temp.path().join("manifests"));

        let (models, runners) =
            load_runtime_rows(&package_registry, &installed_registry).expect("runtime rows");

        assert!(models.is_empty());
        assert!(!runners.is_empty());
    }
}
