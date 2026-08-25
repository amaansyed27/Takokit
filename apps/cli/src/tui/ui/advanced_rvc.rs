use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::widgets::{centered_rect, field, primary_button, render_text_input};
use crate::tui::{
    advanced_rvc::{AdvancedRvcField, ADVANCED_RVC_ACTIONS},
    app::App,
};

pub fn render_create_voice(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(82, 74, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(form);

    frame.render_widget(
        Paragraph::new("Create Voice\nChoose a fast reference clone or open the persistent Advanced RVC workflow.")
            .wrap(Wrap { trim: false }),
        rows[0],
    );
    frame.render_widget(
        field(
            "Instant Clone",
            "One reference recording → reusable voice for compatible TTS models",
            app.create_voice_index == 0,
        ),
        rows[1],
    );
    frame.render_widget(
        field(
            "Advanced RVC",
            "Multi-sample dataset → inspect → prepare → preflight → train → checkpoint → test",
            app.create_voice_index == 1,
        ),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new("↑/↓ choose · Enter open · Esc home")
            .style(Style::default().add_modifier(Modifier::DIM)),
        rows[3],
    );
}

pub fn render_advanced_rvc(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(94, 100, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(form);

    let state = &app.advanced_rvc;
    frame.render_widget(
        Paragraph::new(
            "Advanced RVC · persistent custom voice studio\nChoose a project and action. Long operations run through the normal cancellable CommandJob flow.",
        )
        .wrap(Wrap { trim: false }),
        rows[0],
    );

    let project = state
        .selected_project()
        .map(|project| {
            format!(
                "{} · {} · checkpoint {}",
                project.name,
                state_name(project.state),
                project
                    .active_checkpoint_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "not selected".to_string())
            )
        })
        .unwrap_or_else(|| {
            "No Advanced RVC project yet — choose New Voice or Import Existing".into()
        });
    frame.render_widget(
        field(
            "Project · ↑/↓ change",
            project,
            state.field == AdvancedRvcField::Project,
        ),
        rows[1],
    );

    render_text_input(
        frame,
        rows[2],
        "Name · used by New Voice / Import Existing",
        &state.name,
        "My custom voice",
        state.field == AdvancedRvcField::Name,
        state.name_cursor,
    );
    render_text_input(
        frame,
        rows[3],
        "Path · F2 browse · sample/checkpoint/package/test source",
        &state.path,
        r#"C:\Voice Project ü\sample.wav"#,
        state.field == AdvancedRvcField::Path,
        state.path_cursor,
    );
    render_text_input(
        frame,
        rows[4],
        "Index · optional .index path or index UUID when activating",
        &state.index,
        "optional",
        state.field == AdvancedRvcField::Index,
        state.index_cursor,
    );
    frame.render_widget(
        field(
            "Preset · ↑/↓ change",
            format!("{} · backend-owned verified RVC envelope", state.preset()),
            state.field == AdvancedRvcField::Preset,
        ),
        rows[5],
    );
    frame.render_widget(
        field(
            "Consent · Space toggles",
            if state.consent {
                "[x] I own this voice or have explicit permission."
            } else {
                "[ ] Required for New Voice and Import Existing."
            },
            state.field == AdvancedRvcField::Consent,
        ),
        rows[6],
    );

    let action = state.selected_action();
    frame.render_widget(
        field(
            "Action · ↑/↓ change",
            format!("{}\n{}", action.label(), action.hint()),
            state.field == AdvancedRvcField::Action,
        )
        .wrap(Wrap { trim: false }),
        rows[7],
    );
    frame.render_widget(
        primary_button(
            &format!("Run {}", action.label()),
            state.field == AdvancedRvcField::Action,
        ),
        rows[8],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "Tab fields · F2 browse · Ctrl+Enter run · {} lifecycle actions · Ctrl+C cancels active job · Esc back",
            ADVANCED_RVC_ACTIONS.len()
        ))
        .style(Style::default().add_modifier(Modifier::DIM)),
        rows[9],
    );
}

fn state_name(state: takokit_core::RvcVoiceProjectState) -> String {
    format!("{state:?}")
        .chars()
        .enumerate()
        .fold(String::new(), |mut value, (index, character)| {
            if index > 0 && character.is_uppercase() {
                value.push(' ');
            }
            value.push(character.to_ascii_lowercase());
            value
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_names_are_human_readable() {
        assert_eq!(
            state_name(takokit_core::RvcVoiceProjectState::ReadyToTrain),
            "ready to train"
        );
    }
}
