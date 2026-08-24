use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::widgets::{centered_rect, field, primary_button, render_text_input, set_input_cursor};
mod helpers;
use helpers::{render_convert_value, render_intro};

use crate::tui::{
    app::{App, SpeakField, TranscribeField},
    clone::CloneField,
    convert::ConvertField,
};

pub fn render_speak(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(84, 94, area);
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
        "Speak · text → voice",
        "Enter text and choose a built-in voice or one you created from reference audio.",
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
    let saved_voice_count = app.compatible_saved_voice_count();
    let voice_label = if saved_voice_count == 0 {
        "Voice · type a voice ID"
    } else {
        "Voice · ↑/↓ saved voices · type ID"
    };
    render_text_input(
        frame,
        rows[2],
        voice_label,
        app.speak_voice.as_str(),
        "default",
        app.speak_field == SpeakField::Voice,
        app.speak_voice_cursor,
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
    let hint = if saved_voice_count == 0 {
        "Tab fields · Ctrl+Enter runs · Create Voice on Home saves cloned voices for compatible models"
            .to_string()
    } else {
        format!(
            "Tab fields · ↑/↓ on Voice cycles {saved_voice_count} saved cloned voice{} · Ctrl+Enter runs",
            if saved_voice_count == 1 { "" } else { "s" }
        )
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().add_modifier(Modifier::DIM)),
        rows[5],
    );

    if app.speak_field == SpeakField::Text {
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
        "Transcribe · audio → text",
        "Choose an installed STT model and a local audio file.",
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
    render_text_input(
        frame,
        rows[2],
        "Audio file · F2 browse",
        app.transcribe_audio.as_str(),
        r#"samples\audio.wav"#,
        app.transcribe_field == TranscribeField::Audio,
        app.transcribe_audio_cursor,
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
        Paragraph::new("F2 browse · Ctrl+U clear · Home/End edit · Ctrl+Enter runs · Esc home")
            .style(Style::default().add_modifier(Modifier::DIM)),
        rows[4],
    );
}

pub fn render_clone(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(86, 96, area);
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
        "Create Voice · reference audio → reusable cloned voice",
        "This saves the voice locally. Afterwards open Speak and select it in the Voice field.",
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
    render_text_input(
        frame,
        rows[2],
        "Voice name",
        app.clone_state.name.as_str(),
        "My voice",
        app.clone_state.field == CloneField::Name,
        app.clone_state.name_cursor,
    );
    render_text_input(
        frame,
        rows[3],
        "Reference audio · F2 browse",
        app.clone_state.sample.as_str(),
        r#"samples\reference.wav"#,
        app.clone_state.field == CloneField::Sample,
        app.clone_state.sample_cursor,
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
        Some(model) if model.executable => "Save cloned voice",
        Some(_) => "Repair model",
        None => "No cloning model installed",
    };
    frame.render_widget(
        primary_button(label, app.clone_state.field == CloneField::Submit),
        rows[5],
    );
    frame.render_widget(
        Paragraph::new(
            "F2 reference audio · Space consent · Ctrl+Enter saves · saved voice appears in Speak",
        )
        .style(Style::default().add_modifier(Modifier::DIM)),
        rows[6],
    );
}

pub fn render_convert(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let model = app.selected_convert_model();
    if !model.is_some_and(|model| model.id == "rvc") {
        render_reference_convert(frame, area, app, model);
        return;
    }

    let form = centered_rect(90, 100, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
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
        "Convert Voice · audio → target voice",
        "The spoken words stay the same. RVC changes the voice using the selected target package.",
    );
    let model_label = model
        .map(|model| format!("{}  ·  {}", model.title, model.state))
        .unwrap_or_else(|| "No installed conversion model".to_string());
    frame.render_widget(
        field(
            "Model · ↑/↓ change",
            model_label,
            app.convert_state.field == ConvertField::Model,
        ),
        rows[1],
    );
    render_text_input(
        frame,
        rows[2],
        "Source speech audio · F2 browse",
        app.convert_state.source.as_str(),
        r#"samples\source.wav"#,
        app.convert_state.field == ConvertField::Source,
        app.convert_state.source_cursor,
    );
    render_text_input(
        frame,
        rows[3],
        "Target RVC package · F2 browse",
        app.convert_state.target.as_str(),
        r#"voices\my-rvc-model"#,
        app.convert_state.field == ConvertField::Target,
        app.convert_state.target_cursor,
    );
    frame.render_widget(
        field(
            "F0 method · ↑/↓ change",
            app.convert_state.f0_method(),
            app.convert_state.field == ConvertField::F0Method,
        ),
        rows[4],
    );
    render_convert_value(
        frame,
        rows[5],
        app,
        ConvertField::PitchShift,
        "Pitch shift · -24..24",
        &app.convert_state.pitch_shift,
    );
    render_convert_value(
        frame,
        rows[6],
        app,
        ConvertField::IndexRate,
        "Index rate · 0..1",
        &app.convert_state.index_rate,
    );
    render_convert_value(
        frame,
        rows[7],
        app,
        ConvertField::RmsMixRate,
        "RMS mix rate · 0..1",
        &app.convert_state.rms_mix_rate,
    );
    render_convert_value(
        frame,
        rows[8],
        app,
        ConvertField::Protect,
        "Protect · 0..0.5",
        &app.convert_state.protect,
    );
    render_convert_value(
        frame,
        rows[9],
        app,
        ConvertField::FilterRadius,
        "Filter radius · 0..7",
        &app.convert_state.filter_radius,
    );
    frame.render_widget(
        field(
            "Consent · Space toggles",
            if app.convert_state.consent {
                "[x] I own these voices or have explicit permission."
            } else {
                "[ ] Explicit source and target permission is required."
            },
            app.convert_state.field == ConvertField::Consent,
        ),
        rows[10],
    );
    let label = match model {
        Some(model) if model.executable => "Convert voice",
        Some(_) => "Repair model",
        None => "No conversion model installed",
    };
    frame.render_widget(
        primary_button(label, app.convert_state.field == ConvertField::Submit),
        rows[11],
    );
    frame.render_widget(
        Paragraph::new(
            "Words should remain unchanged · voice should move toward the target · review by listening",
        )
        .style(Style::default().add_modifier(Modifier::DIM)),
        rows[12],
    );

    match app.convert_state.field {
        ConvertField::PitchShift => {
            set_input_cursor(frame, rows[5], app.convert_state.pitch_shift_cursor)
        }
        ConvertField::IndexRate => {
            set_input_cursor(frame, rows[6], app.convert_state.index_rate_cursor)
        }
        ConvertField::RmsMixRate => {
            set_input_cursor(frame, rows[7], app.convert_state.rms_mix_rate_cursor)
        }
        ConvertField::Protect => set_input_cursor(frame, rows[8], app.convert_state.protect_cursor),
        ConvertField::FilterRadius => {
            set_input_cursor(frame, rows[9], app.convert_state.filter_radius_cursor)
        }
        _ => {}
    }
}

fn render_reference_convert(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    model: Option<&crate::tui::catalog::ModelRow>,
) {
    let form = centered_rect(88, 78, area);
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
        "Convert Voice · audio → target voice",
        "The spoken words stay the same. The source voice should move toward the target reference voice.",
    );
    let model_label = model
        .map(|model| format!("{}  ·  {}", model.title, model.state))
        .unwrap_or_else(|| "No installed conversion model".to_string());
    frame.render_widget(
        field(
            "Model · ↑/↓ change",
            model_label,
            app.convert_state.field == ConvertField::Model,
        ),
        rows[1],
    );
    render_text_input(
        frame,
        rows[2],
        "Source speech audio · F2 browse",
        app.convert_state.source.as_str(),
        r#"samples\source.wav"#,
        app.convert_state.field == ConvertField::Source,
        app.convert_state.source_cursor,
    );
    render_text_input(
        frame,
        rows[3],
        "Target voice reference · F2 browse",
        app.convert_state.target.as_str(),
        r#"samples\reference.wav"#,
        app.convert_state.field == ConvertField::Target,
        app.convert_state.target_cursor,
    );
    frame.render_widget(
        field(
            "Consent · Space toggles",
            if app.convert_state.consent {
                "[x] I own these voices or have explicit permission."
            } else {
                "[ ] Explicit source and target permission is required."
            },
            app.convert_state.field == ConvertField::Consent,
        ),
        rows[4],
    );
    let label = match model {
        Some(model) if model.executable => "Convert voice",
        Some(_) => "Repair model",
        None => "No conversion model installed",
    };
    frame.render_widget(
        primary_button(label, app.convert_state.field == ConvertField::Submit),
        rows[5],
    );
    frame.render_widget(
        Paragraph::new(
            "F2 source/target audio · Space consent · Ctrl+Enter runs · P plays the result in Activity",
        )
        .style(Style::default().add_modifier(Modifier::DIM)),
        rows[6],
    );
}
