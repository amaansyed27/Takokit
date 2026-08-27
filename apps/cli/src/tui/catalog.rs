use takokit_package::{plan_model, InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

#[derive(Debug, Clone)]
pub struct ModelRow {
    pub id: String,
    pub title: String,
    pub state: String,
    pub detail: String,
    pub tts: bool,
    pub stt: bool,
    pub voice_cloning: bool,
    pub voice_conversion: bool,
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
    UpdateStatus,
    UpdateCheck,
    UpdateInstall,
    UpdateStable,
    UpdatePreview,
    AutomaticChecksOn,
    AutomaticChecksOff,
    AutomaticDownloadOn,
    AutomaticDownloadOff,
    StartDaemon,
    StopDaemon,
    RestartDaemon,
    Logs,
    OpenGui,
}

#[derive(Debug, Clone)]
pub struct SystemRow {
    pub title: String,
    pub state: String,
    pub detail: String,
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
                voice_conversion: model.capabilities.voice_conversion,
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
    let store = LocalStore::new(LocalStore::default_root());
    let update = update_display_state(&store);
    let available = update
        .available
        .as_deref()
        .map(|version| format!("available {version}"))
        .unwrap_or_else(|| "no checked update".to_string());
    let auto_checks = if update.automatic_checks { "on" } else { "off" };
    let auto_download = if update.automatic_download { "on" } else { "off" };
    vec![
        row(
            "Runtime status",
            "read",
            "Check the daemon, storage, and currently available runtime state.",
            SystemAction::Status,
        ),
        row(
            "Doctor",
            "diagnostics",
            "Run the complete local setup and model readiness check.",
            SystemAction::Doctor,
        ),
        row(
            "Update status",
            &available,
            &format!(
                "Current version {} · channel {} · automatic checks {} · automatic download {}.",
                env!("CARGO_PKG_VERSION"), update.channel, auto_checks, auto_download
            ),
            SystemAction::UpdateStatus,
        ),
        row(
            "Check for updates",
            "signed manifest",
            "Verify the selected channel's signed release manifest without installing anything.",
            SystemAction::UpdateCheck,
        ),
        row(
            "Install available update",
            "manual",
            "Explicitly verify, stage, and install the available update. Active work blocks installation.",
            SystemAction::UpdateInstall,
        ),
        row(
            "Use stable update channel",
            if update.channel == "stable" { "selected" } else { "channel" },
            "Select stable signed application releases.",
            SystemAction::UpdateStable,
        ),
        row(
            "Use preview update channel",
            if update.channel == "preview" { "selected" } else { "channel" },
            "Select preview signed application releases.",
            SystemAction::UpdatePreview,
        ),
        row(
            "Enable automatic update checks",
            if update.automatic_checks { "enabled" } else { "setting" },
            "Check signed release metadata opportunistically at most once per day.",
            SystemAction::AutomaticChecksOn,
        ),
        row(
            "Disable automatic update checks",
            if !update.automatic_checks { "enabled" } else { "setting" },
            "Disable opportunistic background release checks.",
            SystemAction::AutomaticChecksOff,
        ),
        row(
            "Enable automatic update download",
            if update.automatic_download { "enabled" } else { "opt-in" },
            "Opt in to verified background download only. Installation and restart remain manual.",
            SystemAction::AutomaticDownloadOn,
        ),
        row(
            "Disable automatic update download",
            if !update.automatic_download { "enabled" } else { "setting" },
            "Keep update downloads user-initiated after a check.",
            SystemAction::AutomaticDownloadOff,
        ),
        row(
            "Start daemon",
            "service",
            "Start Takokit's managed local API service.",
            SystemAction::StartDaemon,
        ),
        row(
            "Stop daemon",
            "service",
            "Stop the managed local API service.",
            SystemAction::StopDaemon,
        ),
        row(
            "Restart daemon",
            "service",
            "Restart the managed local API service.",
            SystemAction::RestartDaemon,
        ),
        row(
            "View logs",
            "diagnostics",
            "Show the latest daemon log location and output.",
            SystemAction::Logs,
        ),
        row(
            "Open GUI",
            "interface",
            "Open the installed Takokit GUI in this same project session.",
            SystemAction::OpenGui,
        ),
    ]
}

fn row(title: &str, state: &str, detail: &str, action: SystemAction) -> SystemRow {
    SystemRow {
        title: title.to_string(),
        state: state.to_string(),
        detail: detail.to_string(),
        action,
    }
}

struct UpdateDisplayState {
    channel: String,
    automatic_checks: bool,
    automatic_download: bool,
    available: Option<String>,
}

fn update_display_state(store: &LocalStore) -> UpdateDisplayState {
    let value = std::fs::read(store.root().join("runtime").join("update-config.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let string = |key: &str| {
        value
            .as_ref()
            .and_then(|value| value.get(key))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    };
    let boolean = |key: &str, fallback: bool| {
        value
            .as_ref()
            .and_then(|value| value.get(key))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(fallback)
    };
    UpdateDisplayState {
        channel: string("channel").unwrap_or_else(|| "stable".to_string()),
        automatic_checks: boolean("automatic_checks", true),
        automatic_download: boolean("automatic_download", false),
        available: string("last_available_version"),
    }
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

    #[test]
    fn system_update_rows_show_safe_automatic_defaults() {
        let rows = system_rows();
        let update = rows
            .iter()
            .find(|row| row.action == SystemAction::UpdateStatus)
            .unwrap();
        assert!(update.detail.contains("automatic checks on"));
        assert!(update.detail.contains("automatic download off"));
    }
}
