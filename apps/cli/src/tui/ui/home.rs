use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::widgets::{centered_rect, render_menu};
use crate::tui::app::{App, HOME_ACTIONS, MANAGE_ACTIONS};

pub fn render_home(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let content = centered_rect(84, 92, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(10)])
        .split(content);

    let summary = if app.models.is_empty() {
        "No models are installed. Install one from the companion library site or CLI when you are ready."
            .to_string()
    } else {
        let ready = app.models.iter().filter(|model| model.executable).count();
        format!(
            "{} installed model{} · {ready} ready. Choose a task below.",
            app.models.len(),
            if app.models.len() == 1 { "" } else { "s" }
        )
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "What do you want to do?",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(summary),
            Line::from(Span::styled(
                "Use ↑/↓ and Enter, or press the visible number.",
                Style::default().add_modifier(Modifier::DIM),
            )),
        ]),
        rows[0],
    );
    render_menu(
        frame,
        rows[1],
        "Tasks",
        &HOME_ACTIONS,
        app.home_index,
    );
}

pub fn render_manage(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let content = centered_rect(78, 82, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(8)])
        .split(content);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Local runtime management",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from("Inspect only what exists on this machine."),
            Line::from(Span::styled(
                "Model discovery stays on the companion library site.",
                Style::default().add_modifier(Modifier::DIM),
            )),
        ]),
        rows[0],
    );
    render_menu(
        frame,
        rows[1],
        "Manage",
        &MANAGE_ACTIONS,
        app.manage_index,
    );
}
