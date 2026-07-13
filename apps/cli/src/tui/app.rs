use takokit_core::RuntimeConfig;
use takokit_package::{plan_model, InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

use super::catalog::{operation_rows, system_rows};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    Quit,
    Refresh,
    RunCli(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiTab {
    Models,
    Runners,
    Operations,
    System,
}

impl TuiTab {
    pub const ALL: [Self; 4] = [Self::Models, Self::Runners, Self::Operations, Self::System];

    pub fn title(self) -> &'static str {
        match self {
            Self::Models => "Models",
            Self::Runners => "Runners",
            Self::Operations => "Operations",
            Self::System => "System",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Models => Self::Runners,
            Self::Runners => Self::Operations,
            Self::Operations => Self::System,
            Self::System => Self::Models,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Models => Self::System,
            Self::Runners => Self::Models,
            Self::Operations => Self::Runners,
            Self::System => Self::Operations,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TuiRow {
    pub id: String,
    pub title: String,
    pub state: String,
    pub detail: String,
    pub command: Option<Vec<String>>,
    pub template: Option<String>,
}

pub struct App {
    pub tab: TuiTab,
    pub models: Vec<TuiRow>,
    pub runners: Vec<TuiRow>,
    pub operations: Vec<TuiRow>,
    pub system: Vec<TuiRow>,
    pub model_index: usize,
    pub runner_index: usize,
    pub operation_index: usize,
    pub system_index: usize,
    pub storage_root: String,
    pub server: String,
    pub status: String,
    pub command_input: String,
    pub command_cursor: usize,
    pub command_mode: bool,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    pub running_command: Option<String>,
    pub last_command: Option<String>,
    pub output_scroll: u16,
    pub tick: u64,
    pub show_help: bool,
}

impl App {
    pub fn new(
        config: &RuntimeConfig,
        store: &LocalStore,
        package_registry: &PackageRegistry,
        installed_registry: &InstalledRegistry,
        status: String,
    ) -> anyhow::Result<Self> {
        let (models, runners) = load_runtime_rows(package_registry, installed_registry)?;
        Ok(Self {
            tab: TuiTab::Models,
            models,
            runners,
            operations: operation_rows(),
            system: system_rows(),
            model_index: 0,
            runner_index: 0,
            operation_index: 0,
            system_index: 0,
            storage_root: store.root().display().to_string(),
            server: config.local_base_url(),
            status,
            command_input: String::new(),
            command_cursor: 0,
            command_mode: false,
            command_history: Vec::new(),
            history_index: None,
            running_command: None,
            last_command: None,
            output_scroll: 0,
            tick: 0,
            show_help: false,
        })
    }

    pub fn reload(
        &mut self,
        config: &RuntimeConfig,
        store: &LocalStore,
        package_registry: &PackageRegistry,
        installed_registry: &InstalledRegistry,
    ) -> anyhow::Result<()> {
        let (models, runners) = load_runtime_rows(package_registry, installed_registry)?;
        self.models = models;
        self.runners = runners;
        self.operations = operation_rows();
        self.system = system_rows();
        self.storage_root = store.root().display().to_string();
        self.server = config.local_base_url();
        self.model_index = clamped_index(self.model_index, self.models.len());
        self.runner_index = clamped_index(self.runner_index, self.runners.len());
        self.operation_index = clamped_index(self.operation_index, self.operations.len());
        self.system_index = clamped_index(self.system_index, self.system.len());
        Ok(())
    }

    pub fn selected_rows(&self) -> &[TuiRow] {
        match self.tab {
            TuiTab::Models => &self.models,
            TuiTab::Runners => &self.runners,
            TuiTab::Operations => &self.operations,
            TuiTab::System => &self.system,
        }
    }

    pub fn selected_index(&self) -> usize {
        match self.tab {
            TuiTab::Models => self.model_index,
            TuiTab::Runners => self.runner_index,
            TuiTab::Operations => self.operation_index,
            TuiTab::System => self.system_index,
        }
    }

    pub fn selected_row(&self) -> Option<&TuiRow> {
        self.selected_rows().get(self.selected_index())
    }

    pub fn selected_detail(&self) -> String {
        self.selected_row()
            .map(|row| row.detail.clone())
            .unwrap_or_else(|| "No entries are available in this section.".to_string())
    }

    pub fn set_status(&mut self, value: impl Into<String>) {
        self.status = value.into();
        self.output_scroll = 0;
    }
}

fn load_runtime_rows(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<(Vec<TuiRow>, Vec<TuiRow>)> {
    let models = package_registry
        .models()?
        .into_iter()
        .map(|model| {
            let plan = plan_model(package_registry, installed_registry, &model.id)?;
            Ok(TuiRow {
                id: model.id.clone(),
                title: model.name,
                state: if plan.executable {
                    "ready".to_string()
                } else {
                    plan.lifecycle_state.to_string()
                },
                detail: format!(
                    "{}\n\nFamily: {}\nTask: {}\nRunner: {}\nExecutable: {}\nMissing: {}\nNext: {}\n\nEnter loads the plan command into the command bar. Press Enter again to run it.\nCtrl+P pull  ·  Ctrl+T test  ·  Ctrl+X remove",
                    model.description,
                    plan.family,
                    plan.task,
                    plan.required_runner,
                    yes_no(plan.executable),
                    if plan.missing.is_empty() {
                        "none".to_string()
                    } else {
                        plan.missing.join("; ")
                    },
                    plan.next_command
                ),
                command: Some(vec!["plan".into(), model.id.clone()]),
                template: None,
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
            TuiRow {
                id: runner.id.clone(),
                title: runner.name,
                state: state.clone(),
                detail: format!(
                    "{}\n\nVersion: {}\nPlatforms: {}\nFamilies: {}\nState: {}\nNote: {}\n\nEnter loads the show command into the command bar. Press Enter again to run it.\nCtrl+P pull contract  ·  Ctrl+I install runtime  ·  Ctrl+X remove",
                    runner.description,
                    runner.version,
                    runner.platforms.join(", "),
                    runner.supported_model_families.join(", "),
                    state,
                    record
                        .as_ref()
                        .map(|record| record.note.as_str())
                        .unwrap_or(&runner.notes)
                ),
                command: Some(vec!["runner".into(), "show".into(), runner.id.clone()]),
                template: None,
            }
        })
        .collect();

    Ok((models, runners))
}

fn clamped_index(current: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        current.min(len - 1)
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_index_clamps_to_available_rows() {
        assert_eq!(clamped_index(9, 3), 2);
        assert_eq!(clamped_index(0, 0), 0);
    }
}
