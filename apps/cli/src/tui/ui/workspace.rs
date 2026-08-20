use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::widgets::{centered_rect, primary_button, render_text_input};
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
                "F2 opens a folder picker. You can also paste/drag a path, use Ctrl+U to clear, or enter a relative path.",
            ),
        ]))
        .wrap(Wrap { trim: false }),
        rows[0],
    );

    render_text_input(
        frame,
        rows[1],
        "Workspace path · F2 browse",
        app.workspace_input.as_str(),
        r#"C:\Users\you\Documents\Takokit"#,
        app.workspace_field == WorkspaceField::Path,
        app.workspace_input_cursor,
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
}
