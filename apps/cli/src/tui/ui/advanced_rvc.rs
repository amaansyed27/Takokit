use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::widgets::{centered_rect, field, primary_button, render_text_input};
use crate::tui::{
    advanced_rvc::{AdvancedRvcAction, AdvancedRvcField},
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
        Paragraph::new(
            "Create Voice\nChoose an instant reference clone or train a reusable local voice from recordings.",
        )
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
            "Train a voice",
            "Add recordings → check them → choose quality → train → test or clone audio",
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
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(form);

    let state = &app.advanced_rvc;
    let action = state.selected_action();
    frame.render_widget(
        Paragraph::new(
            "Train a voice · local RVC studio\nChoose what you want to do. Takokit manages preparation, checkpoints and indexes automatically.",
        )
        .wrap(Wrap { trim: false }),
        rows[0],
    );

    let project = state
        .selected_project()
        .map(|project| format!("{} · {}", project.name, state_name(project.state)))
        .unwrap_or_else(|| "No trained voice yet — choose Create trained voice".into());
    frame.render_widget(
        field(
            "Voice · ↑/↓ change",
            project,
            state.field == AdvancedRvcField::Project,
        ),
        rows[1],
    );

    frame.render_widget(
        field(
            "What do you want to do? · ↑/↓ change",
            format!("{}\n{}", action.label(), action.hint()),
            state.field == AdvancedRvcField::Action,
        )
        .wrap(Wrap { trim: false }),
        rows[2],
    );

    if action.requires_name() {
        render_text_input(
            frame,
            rows[3],
            "Voice name",
            &state.name,
            "For example, Studio narrator",
            state.field == AdvancedRvcField::Name,
            state.name_cursor,
        );
    } else {
        frame.render_widget(
            field(
                "Voice name",
                "Uses the selected trained voice above.",
                false,
            ),
            rows[3],
        );
    }

    if action.requires_path() {
        render_text_input(
            frame,
            rows[4],
            path_label(action),
            &state.path,
            path_placeholder(action),
            state.field == AdvancedRvcField::Path,
            state.path_cursor,
        );
    } else {
        frame.render_widget(
            field("Audio / file", "No file is needed for this action.", false),
            rows[4],
        );
    }

    if action.shows_index_input() {
        render_text_input(
            frame,
            rows[5],
            "Legacy RVC index · optional",
            &state.index,
            "Only needed when importing a legacy .pth + .index pair",
            state.field == AdvancedRvcField::Index,
            state.index_cursor,
        );
    } else {
        frame.render_widget(
            field(
                "Model files",
                "Managed automatically — no checkpoint or index selection required.",
                false,
            ),
            rows[5],
        );
    }

    if action.shows_training_quality() {
        frame.render_widget(
            field(
                "Training quality · ↑/↓ change",
                format!(
                    "{} · Balanced is the recommended default",
                    preset_label(state.preset())
                ),
                state.field == AdvancedRvcField::Preset,
            ),
            rows[6],
        );
    } else {
        frame.render_widget(
            field(
                "Training quality",
                "Choose a quality level when you start training.",
                false,
            ),
            rows[6],
        );
    }

    if action.requires_consent() {
        frame.render_widget(
            field(
                "Voice permission · Space toggles",
                if state.consent {
                    "[x] I own this voice or have explicit permission to use it."
                } else {
                    "[ ] Permission is required before creating or importing a voice."
                },
                state.field == AdvancedRvcField::Consent,
            ),
            rows[7],
        );
    } else {
        frame.render_widget(
            field(
                "Voice permission",
                "Permission was recorded when this trained voice was created or imported.",
                false,
            ),
            rows[7],
        );
    }

    frame.render_widget(
        primary_button(
            &format!("Run {}", action.label()),
            state.field == AdvancedRvcField::Action,
        ),
        rows[8],
    );
    frame.render_widget(
        Paragraph::new(
            "Tab fields · F2 browse when needed · Ctrl+Enter run · Ctrl+C cancels active training · Esc back",
        )
        .style(Style::default().add_modifier(Modifier::DIM)),
        rows[9],
    );
}

fn path_label(action: AdvancedRvcAction) -> &'static str {
    match action {
        AdvancedRvcAction::ImportExisting => "Voice package / legacy model · F2 browse",
        AdvancedRvcAction::AddSample => "Recording · F2 browse",
        AdvancedRvcAction::TestVoice => "Test speech · F2 browse",
        _ => "Audio / file",
    }
}

fn path_placeholder(action: AdvancedRvcAction) -> &'static str {
    match action {
        AdvancedRvcAction::ImportExisting => "Choose a .takovoice package or legacy .pth model",
        AdvancedRvcAction::AddSample => "Choose a clean recording of this speaker",
        AdvancedRvcAction::TestVoice => "Choose speech from another speaker",
        _ => "Choose a file",
    }
}

fn preset_label(preset: &str) -> &'static str {
    match preset {
        "quick" => "Quick test",
        "high-quality" => "High quality",
        "custom" => "Custom",
        _ => "Balanced",
    }
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

    #[test]
    fn normal_training_copy_hides_checkpoint_management() {
        assert_eq!(
            path_label(AdvancedRvcAction::AddSample),
            "Recording · F2 browse"
        );
        assert_eq!(preset_label("balanced"), "Balanced");
    }
}
