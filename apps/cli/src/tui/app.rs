use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use takokit_core::RuntimeConfig;
use takokit_package::{plan_model, InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

use super::{command, ui};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    Quit,
    Refresh,
    PullModel(String),
    PlanModel(String),
    InstallRunner(String),
    ShowRunner(String),
    Doctor,
    OpenGui,
    StartServer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiTab {
    Models,
    Runners,
    System,
}

impl TuiTab {
    pub const ALL: [Self; 3] = [Self::Models, Self::Runners, Self::System];

    pub fn title(self) -> &'static str {
        match self {
            Self::Models => "Models",
            Self::Runners => "Runners",
            Self::System => "System",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Models => Self::Runners,
            Self::Runners => Self::System,
            Self::System => Self::Models,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Models => Self::System,
            Self::Runners => Self::Models,
            Self::System => Self::Runners,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TuiRow {
    pub id: String,
    pub title: String,
    pub state: String,
    pub detail: String,
}

pub struct App {
    pub tab: TuiTab,
    pub models: Vec<TuiRow>,
    pub runners: Vec<TuiRow>,
    pub model_index: usize,
    pub runner_index: usize,
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
                let state = if plan.executable {
                    "ready".to_string()
                } else {
                    plan.lifecycle_state.to_string()
                };
                let detail = format!(
                    "{}\n\nFamily: {}\nTask: {}\nRunner: {}\nExecutable: {}\nMissing: {}\nNext: {}",
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
                );
                Ok(TuiRow {
                    id: model.id,
                    title: model.name,
                    state,
                    detail,
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
                let detail = format!(
                    "{}\n\nVersion: {}\nPlatforms: {}\nFamilies: {}\nState: {}\nNote: {}",
                    runner.description,
                    runner.version,
                    runner.platforms.join(", "),
                    runner.supported_model_families.join(", "),
                    state,
                    record
                        .as_ref()
                        .map(|record| record.note.as_str())
                        .unwrap_or(&runner.notes)
                );
                TuiRow {
                    id: runner.id,
                    title: runner.name,
                    state,
                    detail,
                }
            })
            .collect();

        Ok(Self {
            tab: TuiTab::Models,
            models,
            runners,
            model_index: 0,
            runner_index: 0,
            storage_root: store.root().display().to_string(),
            server: config.local_base_url(),
            status,
            command_input: String::new(),
            command_mode: false,
            show_help: false,
        })
    }

    pub fn selected_model(&self) -> Option<&TuiRow> {
        self.models.get(self.model_index)
    }

    pub fn selected_runner(&self) -> Option<&TuiRow> {
        self.runners.get(self.runner_index)
    }

    pub fn selected_detail(&self) -> String {
        match self.tab {
            TuiTab::Models => self
                .selected_model()
                .map(|row| row.detail.clone())
                .unwrap_or_else(|| "No models in the catalog.".to_string()),
            TuiTab::Runners => self
                .selected_runner()
                .map(|row| row.detail.clone())
                .unwrap_or_else(|| "No runners in the catalog.".to_string()),
            TuiTab::System => format!(
                "Storage\n{}\n\nManaged daemon\n{}\n\nSurfaces\n- Direct CLI: takokit <command>\n- Interactive TUI: takokit\n- GUI: takokit gui",
                self.storage_root, self.server
            ),
        }
    }

    fn move_selection(&mut self, delta: isize) {
        match self.tab {
            TuiTab::Models => {
                self.model_index = shifted_index(self.model_index, self.models.len(), delta)
            }
            TuiTab::Runners => {
                self.runner_index = shifted_index(self.runner_index, self.runners.len(), delta)
            }
            TuiTab::System => {}
        }
    }

    fn action_for_enter(&self) -> TuiAction {
        match self.tab {
            TuiTab::Models => self
                .selected_model()
                .map(|row| TuiAction::PlanModel(row.id.clone()))
                .unwrap_or(TuiAction::Refresh),
            TuiTab::Runners => self
                .selected_runner()
                .map(|row| TuiAction::ShowRunner(row.id.clone()))
                .unwrap_or(TuiAction::Refresh),
            TuiTab::System => TuiAction::Doctor,
        }
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
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
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
                self.command_mode = true;
                self.command_input.clear();
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
            KeyCode::Enter => Some(self.action_for_enter()),
            KeyCode::Char('p') => self
                .selected_model()
                .map(|row| TuiAction::PullModel(row.id.clone())),
            KeyCode::Char('i') => self
                .selected_runner()
                .map(|row| TuiAction::InstallRunner(row.id.clone())),
            KeyCode::Char('d') => Some(TuiAction::Doctor),
            KeyCode::Char('g') => Some(TuiAction::OpenGui),
            KeyCode::Char('s') => Some(TuiAction::StartServer),
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
}
