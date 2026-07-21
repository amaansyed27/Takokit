use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::widgets::{detail_panel, empty_state, render_rows};
use crate::tui::app::App;

pub fn render_models(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if app.models.is_empty() {
        empty_state(
            frame,
            area,
            "No models installed",
            "Install a model through the companion library site or CLI, then press R to refresh.",
        );
        return;
    }
    let rows = page_rows(area);
    render_intro(
        frame,
        rows[0],
        "Installed models",
        "Enter uses a ready model or repairs it. X removes it from this machine.",
    );
    render_rows(
        frame,
        rows[1],
        "Models",
        app.models
            .iter()
            .map(|model| (model.title.clone(), model.state.clone()))
            .collect(),
        app.model_index,
    );
    let detail = app
        .selected_model()
        .map(|model| model.detail.clone())
        .unwrap_or_else(|| "Select a model to inspect it.".to_string());
    frame.render_widget(detail_panel("Details", detail), rows[2]);
}

pub fn render_runners(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if app.runners.is_empty() {
        empty_state(
            frame,
            area,
            "No runners found",
            "Install or repair a model first, then press R to refresh runtime state.",
        );
        return;
    }
    let rows = page_rows(area);
    render_intro(
        frame,
        rows[0],
        "Runners",
        "Enter performs the sensible next action. D checks, I installs, and X removes.",
    );
    render_rows(
        frame,
        rows[1],
        "Runners",
        app.runners
            .iter()
            .map(|runner| (runner.title.clone(), runner.state.clone()))
            .collect(),
        app.runner_index,
    );
    let detail = app
        .selected_runner()
        .map(|runner| runner.detail.clone())
        .unwrap_or_else(|| "Select a runner to inspect it.".to_string());
    frame.render_widget(detail_panel("Details", detail), rows[2]);
}

pub fn render_system(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows = page_rows(area);
    render_intro(
        frame,
        rows[0],
        "System",
        "Run diagnostics, control the daemon, inspect logs, or open the GUI.",
    );
    render_rows(
        frame,
        rows[1],
        "Actions",
        app.system
            .iter()
            .map(|row| (row.title.to_string(), row.state.to_string()))
            .collect(),
        app.system_index,
    );
    let detail = app
        .selected_system()
        .map(|row| row.detail.to_string())
        .unwrap_or_else(|| "Select a system action.".to_string());
    frame.render_widget(detail_panel("Details", detail), rows[2]);
}

pub fn render_sessions(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if app.sessions.is_empty() {
        empty_state(
            frame,
            area,
            "No sessions yet",
            "Press N to create a session for generated outputs and task history.",
        );
        return;
    }
    let rows = page_rows(area);
    render_intro(
        frame,
        rows[0],
        "Sessions",
        "Enter opens the selected session. N creates a new one.",
    );
    let active = app.active_session();
    render_rows(
        frame,
        rows[1],
        "Sessions",
        app.sessions
            .iter()
            .map(|session| {
                (
                    session.title.clone(),
                    if session.id == active {
                        "active".to_string()
                    } else {
                        format!("{} events", session.event_count)
                    },
                )
            })
            .collect(),
        app.session_index,
    );
    let detail = app
        .selected_session()
        .map(|session| {
            format!(
                "{}\n\nID: {}\nCreated: {}\nUpdated: {}\nEvents: {}\nOutputs: {}\nLast task: {}\nLast model: {}\nWorkspace: {}",
                session.title,
                session.id,
                session.created_at,
                session.updated_at,
                session.event_count,
                session.output_count,
                session
                    .last_task
                    .map(|task| task.label())
                    .unwrap_or("none"),
                session.last_model.as_deref().unwrap_or("none"),
                session.workspace_root.display()
            )
        })
        .unwrap_or_else(|| "Select a session to inspect it.".to_string());
    frame.render_widget(detail_panel("Details", detail), rows[2]);
}

pub fn render_activity(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let title = app
        .running_label
        .as_deref()
        .map(|label| format!("Running · {label}"))
        .or_else(|| app.last_label.as_deref().map(|label| format!("Last result · {label}")))
        .unwrap_or_else(|| "Activity".to_string());
    frame.render_widget(
        Paragraph::new(app.status.as_str())
            .scroll((app.output_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(
                ratatui::widgets::Block::default()
                    .title(format!(" {title} "))
                    .borders(ratatui::widgets::Borders::ALL),
            ),
        area,
    );
}

fn page_rows(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(44),
            Constraint::Min(8),
        ])
        .split(area)
}

fn render_intro(frame: &mut Frame<'_>, area: Rect, title: &str, detail: &str) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                detail,
                Style::default().add_modifier(Modifier::DIM),
            )),
        ]),
        area,
    );
}
