use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::{
    app::{App, TuiAction},
    editor::{edit_text, shifted_index},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneField {
    Model,
    Name,
    Sample,
    Consent,
    Primary,
}

impl CloneField {
    fn next(self) -> Self {
        match self {
            Self::Model => Self::Name,
            Self::Name => Self::Sample,
            Self::Sample => Self::Consent,
            Self::Consent => Self::Primary,
            Self::Primary => Self::Model,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Model => Self::Primary,
            Self::Name => Self::Model,
            Self::Sample => Self::Name,
            Self::Consent => Self::Sample,
            Self::Primary => Self::Consent,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloneState {
    pub model_indexes: Vec<usize>,
    pub model_index: usize,
    pub field: CloneField,
    pub name: String,
    pub name_cursor: usize,
    pub sample: String,
    pub sample_cursor: usize,
    pub consent: bool,
}

impl CloneState {
    pub fn new(app_models: &[super::catalog::ModelRow]) -> Self {
        let model_indexes = app_models
            .iter()
            .enumerate()
            .filter_map(|(index, model)| model.voice_cloning.then_some(index))
            .collect::<Vec<_>>();
        let model_index = model_indexes
            .iter()
            .position(|index| app_models[*index].id == "chatterbox")
            .unwrap_or(0);
        Self {
            model_indexes,
            model_index,
            field: CloneField::Name,
            name: String::new(),
            name_cursor: 0,
            sample: String::new(),
            sample_cursor: 0,
            consent: false,
        }
    }

    pub fn reload_models(&mut self, app_models: &[super::catalog::ModelRow]) {
        let selected = self
            .model_indexes
            .get(self.model_index)
            .and_then(|index| app_models.get(*index))
            .map(|model| model.id.clone());
        self.model_indexes = app_models
            .iter()
            .enumerate()
            .filter_map(|(index, model)| model.voice_cloning.then_some(index))
            .collect();
        self.model_index = selected
            .and_then(|id| {
                self.model_indexes
                    .iter()
                    .position(|index| app_models[*index].id == id)
            })
            .unwrap_or(0);
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        app.clone_state.field = if key.code == KeyCode::BackTab {
            app.clone_state.field.previous()
        } else {
            app.clone_state.field.next()
        };
        return None;
    }
    match app.clone_state.field {
        CloneField::Model => match key.code {
            KeyCode::Up | KeyCode::Left => {
                app.clone_state.model_index = shifted_index(
                    app.clone_state.model_index,
                    app.clone_state.model_indexes.len(),
                    -1,
                );
            }
            KeyCode::Down | KeyCode::Right => {
                app.clone_state.model_index = shifted_index(
                    app.clone_state.model_index,
                    app.clone_state.model_indexes.len(),
                    1,
                );
            }
            KeyCode::Enter => app.clone_state.field = CloneField::Name,
            _ => {}
        },
        CloneField::Name => {
            if edit_text(
                &mut app.clone_state.name,
                &mut app.clone_state.name_cursor,
                key,
            ) {
                return None;
            }
            if key.code == KeyCode::Enter {
                app.clone_state.field = CloneField::Sample;
            }
        }
        CloneField::Sample => {
            if edit_text(
                &mut app.clone_state.sample,
                &mut app.clone_state.sample_cursor,
                key,
            ) {
                return None;
            }
            if key.code == KeyCode::Enter {
                app.clone_state.field = CloneField::Consent;
            }
        }
        CloneField::Consent => match key.code {
            KeyCode::Char(' ') => app.clone_state.consent = !app.clone_state.consent,
            KeyCode::Enter => app.clone_state.field = CloneField::Primary,
            _ => {}
        },
        CloneField::Primary => {
            if key.code == KeyCode::Enter {
                return submit(app);
            }
        }
    }
    None
}

fn submit(app: &mut App) -> Option<TuiAction> {
    let model = app.selected_clone_model()?.clone();
    if !model.executable {
        return Some(TuiAction::PullModel(model.id));
    }
    let name = app.clone_state.name.trim().to_string();
    let sample = app.clone_state.sample.trim().to_string();
    if name.is_empty() {
        app.set_status("Enter a profile name before creating the voice.");
        app.clone_state.field = CloneField::Name;
        return None;
    }
    if sample.is_empty() {
        app.set_status("Enter a local reference-audio path.");
        app.clone_state.field = CloneField::Sample;
        return None;
    }
    if !app.clone_state.consent {
        app.set_status("Explicit voice-owner consent is required.");
        app.clone_state.field = CloneField::Consent;
        return None;
    }
    Some(TuiAction::CloneVoice {
        model: model.id,
        name,
        sample,
    })
}

pub fn render(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(82, 100, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(form);
    frame.render_widget(
        Paragraph::new("Create a reusable local voice profile from consented reference audio."),
        rows[0],
    );
    let model = app.selected_clone_model();
    render_field(
        frame,
        rows[1],
        " Model · ↑/↓ change ",
        &model
            .map(|model| format!("{} · {}", model.title, model.state))
            .unwrap_or_else(|| "No cloning model available".to_string()),
        app.clone_state.field == CloneField::Model,
    );
    render_field(
        frame,
        rows[2],
        " Profile name ",
        if app.clone_state.name.is_empty() {
            "My voice"
        } else {
            &app.clone_state.name
        },
        app.clone_state.field == CloneField::Name,
    );
    render_field(
        frame,
        rows[3],
        " Reference audio ",
        if app.clone_state.sample.is_empty() {
            r#"C:\path\to\reference.wav"#
        } else {
            &app.clone_state.sample
        },
        app.clone_state.field == CloneField::Sample,
    );
    render_field(
        frame,
        rows[4],
        " Consent · Space toggle ",
        if app.clone_state.consent {
            "[x] I own this voice or have explicit permission."
        } else {
            "[ ] Explicit permission is required."
        },
        app.clone_state.field == CloneField::Consent,
    );
    let label = match model {
        Some(model) if model.executable => " Create voice profile ",
        Some(_) => " Repair selected model ",
        None => " No cloning model installed ",
    };
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(if app.clone_state.field == CloneField::Primary {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            })
            .block(Block::default().borders(Borders::ALL)),
        rows[5],
    );
    frame.render_widget(
        Paragraph::new(
            "Profiles are stored in ~/.takokit/voices and the creation event is saved in the active .tako session.",
        )
        .wrap(Wrap { trim: false }),
        rows[6],
    );
    match app.clone_state.field {
        CloneField::Name => set_cursor(frame, rows[2], app.clone_state.name_cursor),
        CloneField::Sample => set_cursor(frame, rows[3], app.clone_state.sample_cursor),
        _ => {}
    }
}

fn render_field(frame: &mut Frame<'_>, area: Rect, title: &str, value: &str, focused: bool) {
    frame.render_widget(
        Paragraph::new(value).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(if focused {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                }),
        ),
        area,
    );
}

fn set_cursor(frame: &mut Frame<'_>, area: Rect, cursor: usize) {
    let x = area
        .x
        .saturating_add(1 + cursor as u16)
        .min(area.right().saturating_sub(2));
    frame.set_cursor_position((x, area.y.saturating_add(1)));
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
