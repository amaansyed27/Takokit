use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::widgets::{centered_rect, field, primary_button, set_input_cursor};
use crate::tui::app::{App, WorkspaceField};

pub fn render_workspace(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(84, 84, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
        ])
        .split(form);

    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(Span::styled(
                "Project workspace",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(
                "Sessions, transcripts and outputs live under <workspace>/.tako. Models, runners and adapters remain global under .takokit.",
            ),
            Line::from(
                "Tip: paste or drag a folder into the terminal. Relative paths are resolved from the current workspace.",
            ),
        ]))
        .wrap(Wrap { trim: false }),
        rows[0],
    );

    frame.render_widget(
        field(
            "Workspace path",
            app.workspace_input.as_str(),
            app.workspace_field == WorkspaceField::Path,
        ),
        rows[1],
    );
    frame.render_widget(
        primary_button(
            "Validate and switch workspace",
            app.workspace_field == WorkspaceField::Apply,
        ),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "Current: {}\n\nSwitching refreshes project sessions only. It does not move or duplicate installed models, and it does not create .tako until a session or workflow writes project data.",
            app.workspace_root
        ))
        .wrap(Wrap { trim: false })
        .style(Style::default().add_modifier(Modifier::DIM)),
        rows[3],
    );

    if app.workspace_field == WorkspaceField::Path {
        set_input_cursor(frame, rows[1], app.workspace_input_cursor);
    }
}
