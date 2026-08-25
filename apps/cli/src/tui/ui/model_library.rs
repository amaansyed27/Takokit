use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::widgets::{detail_panel, empty_state, render_rows};
use crate::tui::app::App;

pub fn render_model_library(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if app.library_models.is_empty() {
        let (title, detail) = if let Some(error) = app.library_error.as_deref() {
            (
                "Model library unavailable",
                format!("Takokit could not load the canonical model registry. Press R to retry.\n\n{error}"),
            )
        } else {
            (
                "Model library is empty",
                "The canonical Takokit registry returned no model releases. Press R to refresh."
                    .to_string(),
            )
        };
        empty_state(frame, area, title, &detail);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(44),
            Constraint::Min(8),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Model library",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Browse Takokit's canonical registry. Enter/P pulls an available model or repairs an incomplete one.",
                Style::default().add_modifier(Modifier::DIM),
            )),
        ]),
        rows[0],
    );
    render_rows(
        frame,
        rows[1],
        "Available models",
        app.library_models
            .iter()
            .map(|model| (model.title.clone(), model.state.clone()))
            .collect(),
        app.library_model_index,
    );
    let detail = app
        .library_models
        .get(app.library_model_index)
        .map(|model| model.detail.clone())
        .unwrap_or_else(|| "Select a registry model to inspect it.".to_string());
    frame.render_widget(detail_panel("Details", detail), rows[2]);
}
