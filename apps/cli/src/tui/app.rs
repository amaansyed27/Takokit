use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use takokit_core::RuntimeConfig;
use takokit_package::{plan_model, InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

use super::{
    catalog::{operation_rows, system_rows},
    command, ui,
};

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

    fn next(self) -> Self {
        match self {
            Self::Models => Self::Runners,
            Self::Runners => Self::Operations,
            Self::Operations => Self::System,
            Self::System => Self::Models,
        }
    }

    fn previous(self) -> Self {
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
    pub command_mode: bool,
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
                        "{}\n\nFamily: {}\nTask: {}\nRunner: {}\nExecutable: {}\nMissing: {}\nNext: {}\n\nKeys\nEnter plan  ·  p pull  ·  t test  ·  x remove",
                        model.description,
                        plan.family,
                        plan.task,
                        plan.required_runner,
                        yes_no(plan.executable),
                        if plan.missing.is_empty() { "none".to_string() } else { plan.missing.join("; ") },
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
                        "{}\n\nVersion: {}\nPlatforms: {}\nFamilies: {}\nState: {}\nNote: {}\n\nKeys\nEnter show  ·  p pull contract  ·  i install runtime  ·  x remove",
                        runner.description,
                        runner.version,
                        runner.platforms.join(", "),
                        runner.supported_model_families.join(", "),
                        state,
                        record.as_ref().map(|record| record.note.as_str()).unwrap_or(&runner.notes)
                    ),
                    command: Some(vec!["runner".into(), "show".into(), runner.id.clone()]),
                    template: None,
                }
            })
            .collect();

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
            command_mode: false,
            show_help: false,
        })
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

    fn move_selection(&mut self, delta: isize) {
        let len = self.selected_rows().len();
        match self.tab {
            TuiTab::Models => self.model_index = shifted_index(self.model_index, len, delta),
            TuiTab::Runners => self.runner_index = shifted_index(self.runner_index, len, delta),
            TuiTab::Operations => {
                self.operation_index = shifted_index(self.operation_index, len, delta)
            }
            TuiTab::System => self.system_index = shifted_index(self.system_index, len, delta),
        }
    }

    fn open_template(&mut self, template: String) {
        self.command_mode = true;
        self.command_input = template;
    }

    fn action_for_enter(&mut self) -> Option<TuiAction> {
        let row = self.selected_row()?.clone();
        if let Some(template) = row.template {
            self.open_template(template);
            None
        } else {
            row.command.map(TuiAction::RunCli)
        }
    }

    fn selected_cli(&self, command: &[&str]) -> Option<TuiAction> {
        let id = self.selected_row()?.id.clone();
        let mut args = command
            .iter()
            .map(|part| (*part).to_string())
            .collect::<Vec<_>>();
        args.push(id);
        Some(TuiAction::RunCli(args))
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Esc => {
                self.command_mode = false;
                self.command_input.clear();
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.command_input);
                self.command_mode = false;
                match command::parse(&input) {
                    Ok(action) => return Some(action),
                    Err(error) => self.status = error,
                }
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Char(character) => self.command_input.push(character),
            _ => {}
        }
        None
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(TuiAction::Quit);
        }
        if self.show_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.show_help = false;
            }
            return None;
        }
        if self.command_mode {
            return self.handle_command_key(key);
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(TuiAction::Quit),
            KeyCode::Char('?') => {
                self.show_help = true;
                None
            }
            KeyCode::Char('/') => {
                self.open_template(String::new());
                None
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.tab = self.tab.next();
                None
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.tab = self.tab.previous();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                None
            }
            KeyCode::Enter => self.action_for_enter(),
            KeyCode::Char('p') if self.tab == TuiTab::Models => self.selected_cli(&["pull"]),
            KeyCode::Char('p') if self.tab == TuiTab::Runners => {
                self.selected_cli(&["runner", "pull"])
            }
            KeyCode::Char('i') if self.tab == TuiTab::Runners => {
                self.selected_cli(&["runner", "install"])
            }
            KeyCode::Char('x') if self.tab == TuiTab::Models => self.selected_cli(&["rm"]),
            KeyCode::Char('x') if self.tab == TuiTab::Runners => {
                self.selected_cli(&["runner", "rm"])
            }
            KeyCode::Char('t') if self.tab == TuiTab::Models => {
                if let Some(row) = self.selected_row() {
                    self.open_template(format!("test {} --run", row.id));
                }
                None
            }
            KeyCode::Char('d') => Some(TuiAction::RunCli(vec!["doctor".into()])),
            KeyCode::Char('g') => Some(TuiAction::RunCli(vec!["gui".into()])),
            KeyCode::Char('s') => Some(TuiAction::RunCli(vec!["daemon".into(), "start".into()])),
            KeyCode::Char('r') => Some(TuiAction::Refresh),
            _ => None,
        }
    }
}

pub fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<TuiAction> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        if let Some(action) = app.handle_key(key) {
            return Ok(action);
        }
    }
}

fn shifted_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).rem_euclid(len as isize) as usize
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
    fn selection_wraps_in_both_directions() {
        assert_eq!(shifted_index(0, 3, -1), 2);
        assert_eq!(shifted_index(2, 3, 1), 0);
        assert_eq!(shifted_index(0, 0, 1), 0);
    }

    #[test]
    fn operations_cover_execution_setup_and_testing() {
        let rows = operation_rows();
        let ids = rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>();
        for expected in [
            "speech",
            "run",
            "transcribe",
            "clone",
            "train",
            "adapter-install",
            "test-fast",
            "quickstart",
            "deps",
            "samples",
        ] {
            assert!(ids.contains(&expected), "missing operation {expected}");
        }
    }
}
