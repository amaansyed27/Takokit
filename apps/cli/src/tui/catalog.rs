use takokit_package::{plan_model, InstalledRegistry, PackageRegistry};

#[derive(Debug, Clone)]
pub struct ModelRow {
    pub id: String,
    pub title: String,
    pub state: String,
    pub detail: String,
    pub tts: bool,
    pub stt: bool,
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
    let models = package_registry
        .models()?
        .into_iter()
        .map(|model| {
            let plan = plan_model(package_registry, installed_registry, &model.id)?;
            let capabilities = [
                model.capabilities.tts.then_some("text to speech"),
                model.capabilities.stt.then_some("speech to text"),
                model.capabilities.voice_cloning.then_some("voice cloning"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
            Ok(ModelRow {
                id: model.id,
                title: model.name,
                state: if plan.executable {
                    "ready".to_string()
                } else {
                    plan.lifecycle_state.to_string()
                },
                detail: format!(
                    "{}\n\nCapability: {}\nFamily: {}\nRunner: {}\nHardware: {}\n\n{}",
                    model.description,
                    if capabilities.is_empty() {
                        "specialized"
                    } else {
                        &capabilities
                    },
                    model.family,
                    plan.required_runner,
                    model
                        .hardware
                        .min_ram
                        .as_deref()
                        .unwrap_or("no minimum listed"),
                    if plan.executable {
                        "Ready to use. Press Enter to open the matching task screen.".to_string()
                    } else {
                        format!(
                            "Not ready yet. Press Enter to let Takokit install what is missing.\nMissing: {}",
                            if plan.missing.is_empty() {
                                "setup".to_string()
                            } else {
                                plan.missing.join("; ")
                            }
                        )
                    }
                ),
                tts: model.capabilities.tts,
                stt: model.capabilities.stt,
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
