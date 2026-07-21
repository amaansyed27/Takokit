use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::widgets::{centered_rect, field, primary_button, set_input_cursor};
use crate::tui::{
    app::{App, SpeakField, TranscribeField},
    clone::CloneField,
};

pub fn render_speak(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(82, 94, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(form);

    render_intro(
        frame,
        rows[0],
        "Generate speech",
        "Select an installed TTS model, enter text, and save a local WAV.",
    );
    let model = app.selected_speak_model();
    let model_label = model
        .map(|model| format!("{}  ·  {}", model.title, model.state))
        .unwrap_or_else(|| "No installed TTS model".to_string());
    frame.render_widget(
        field(
            "Model · ↑/↓ change",
            model_label,
            app.speak_field == SpeakField::Model,
        ),
        rows[1],
    );
    frame.render_widget(
        field(
            "Voice",
            app.speak_voice.as_str(),
            app.speak_field == SpeakField::Voice,
        ),
        rows[2],
    );
    frame.render_widget(
        field(
            "Text",
            if app.speak_text.is_empty() {
                "Type what Takokit should say…"
            } else {
                app.speak_text.as_str()
            },
            app.speak_field == SpeakField::Text,
        )
        .wrap(Wrap { trim: false }),
        rows[3],
    );
    let label = match model {
        Some(model) if model.executable => "Generate speech",
        Some(_) => "Repair model",
        None => "No TTS model installed",
    };
    frame.render_widget(
        primary_button(label, app.speak_field == SpeakField::Submit),
        rows[4],
    );
    frame.render_widget(
        Paragraph::new("Tab moves between fields · Ctrl+Enter runs · Esc returns home")
            .style(Style::default().add_modifier(Modifier::DIM)),
        rows[5],
    );

    if app.speak_field == SpeakField::Voice {
        set_input_cursor(frame, rows[2], app.speak_voice_cursor);
    } else if app.speak_field == SpeakField::Text {
        set_input_cursor(frame, rows[3], app.speak_text_cursor);
    }
}

pub fn render_transcribe(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(82, 82, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(form);

    render_intro(
        frame,
        rows[0],
        "Transcribe audio",
        "Choose an installed STT model and enter a local audio-file path.",
    );
    let model = app.selected_transcribe_model();
    let model_label = model
        .map(|model| format!("{}  ·  {}", model.title, model.state))
        .unwrap_or_else(|| "No installed STT model".to_string());
    frame.render_widget(
        field(
            "Model · ↑/↓ change",
            model_label,
            app.transcribe_field == TranscribeField::Model,
        ),
        rows[1],
    );
    frame.render_widget(
        field(
            "Audio file",
            if app.transcribe_audio.is_empty() {
                r#"C:\path\to\audio.wav"#
            } else {
                app.transcribe_audio.as_str()
            },
            app.transcribe_field == TranscribeField::Audio,
        ),
        rows[2],
    );
    let label = match model {
        Some(model) if model.executable => "Transcribe audio",
        Some(_) => "Repair model",
        None => "No STT model installed",
    };
    frame.render_widget(
        primary_button(label, app.transcribe_field == TranscribeField::Submit),
        rows[3],
    );
    frame.render_widget(
        Paragraph::new("Tab moves between fields · Ctrl+Enter runs · Esc returns home")
            .style(Style::default().add_modifier(Modifier::DIM)),
        rows[4],
    );

    if app.transcribe_field == TranscribeField::Audio {
        set_input_cursor(frame, rows[2], app.transcribe_audio_cursor);
    }
}

pub fn render_clone(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(82, 96, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(form);

    render_intro(
        frame,
        rows[0],
        "Clone a voice",
        "Create a reusable local voice profile from consented reference audio.",
    );
    let model = app.selected_clone_model();
    let model_label = model
        .map(|model| format!("{}  ·  {}", model.title, model.state))
        .unwrap_or_else(|| "No installed cloning model".to_string());
    frame.render_widget(
        field(
            "Model · ↑/↓ change",
            model_label,
            app.clone_state.field == CloneField::Model,
        ),
        rows[1],
    );
    frame.render_widget(
        field(
            "Profile name",
            if app.clone_state.name.is_empty() {
                "My voice"
            } else {
                app.clone_state.name.as_str()
            },
            app.clone_state.field == CloneField::Name,
        ),
        rows[2],
    );
    frame.render_widget(
        field(
            "Reference audio",
            if app.clone_state.sample.is_empty() {
                r#"C:\path\to\reference.wav"#
            } else {
                app.clone_state.sample.as_str()
            },
            app.clone_state.field == CloneField::Sample,
        ),
        rows[3],
    );
    frame.render_widget(
        field(
            "Consent · Space toggles",
            if app.clone_state.consent {
                "[x] I own this voice or have explicit permission."
            } else {
                "[ ] Explicit permission is required."
            },
            app.clone_state.field == CloneField::Consent,
        ),
        rows[4],
    );
    let label = match model {
        Some(model) if model.executable => "Create voice profile",
        Some(_) => "Repair model",
        None => "No cloning model installed",
    };
    frame.render_widget(
        primary_button(label, app.clone_state.field == CloneField::Submit),
        rows[5],
    );
    frame.render_widget(
        Paragraph::new("Tab moves between fields · Space confirms consent · Ctrl+Enter runs")
            .style(Style::default().add_modifier(Modifier::DIM)),
        rows[6],
    );

    match app.clone_state.field {
        CloneField::Name => set_input_cursor(frame, rows[2], app.clone_state.name_cursor),
        CloneField::Sample => set_input_cursor(frame, rows[3], app.clone_state.sample_cursor),
        _ => {}
    }
}

fn render_intro(frame: &mut Frame<'_>, area: Rect, title: &str, detail: &str) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(detail),
        ]),
        area,
    );
}
