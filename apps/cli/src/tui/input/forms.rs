use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{
    app::{App, SpeakField, TranscribeField, TuiAction},
    clone::CloneField,
    convert::{ConvertField, F0_METHODS},
    editor::{edit_text, shifted_index},
};

pub(super) fn handle_speak(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        app.speak_field = if key.code == KeyCode::BackTab {
            app.speak_field.previous()
        } else {
            app.speak_field.next()
        };
        return None;
    }
    match app.speak_field {
        SpeakField::Model => match key.code {
            KeyCode::Left | KeyCode::Up => {
                app.speak_model_index =
                    shifted_index(app.speak_model_index, app.tts_models.len(), -1)
            }
            KeyCode::Right | KeyCode::Down => {
                app.speak_model_index =
                    shifted_index(app.speak_model_index, app.tts_models.len(), 1)
            }
            KeyCode::Enter => app.speak_field = SpeakField::Voice,
            _ => {}
        },
        SpeakField::Voice => {
            if edit_text(&mut app.speak_voice, &mut app.speak_voice_cursor, key) {
                return None;
            }
            if key.code == KeyCode::Enter {
                app.speak_field = SpeakField::Text;
            }
        }
        SpeakField::Text => {
            if edit_text(&mut app.speak_text, &mut app.speak_text_cursor, key) {
                return None;
            }
            if key.code == KeyCode::Enter {
                app.speak_field = SpeakField::Submit;
            }
        }
        SpeakField::Submit => {
            if key.code == KeyCode::Enter {
                return submit_speak(app);
            }
        }
    }
    None
}

pub(super) fn handle_transcribe(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        app.transcribe_field = if key.code == KeyCode::BackTab {
            app.transcribe_field.previous()
        } else {
            app.transcribe_field.next()
        };
        return None;
    }
    match app.transcribe_field {
        TranscribeField::Model => match key.code {
            KeyCode::Left | KeyCode::Up => {
                app.transcribe_model_index =
                    shifted_index(app.transcribe_model_index, app.stt_models.len(), -1)
            }
            KeyCode::Right | KeyCode::Down => {
                app.transcribe_model_index =
                    shifted_index(app.transcribe_model_index, app.stt_models.len(), 1)
            }
            KeyCode::Enter => app.transcribe_field = TranscribeField::Audio,
            _ => {}
        },
        TranscribeField::Audio => {
            if edit_text(
                &mut app.transcribe_audio,
                &mut app.transcribe_audio_cursor,
                key,
            ) {
                return None;
            }
            if key.code == KeyCode::Enter {
                app.transcribe_field = TranscribeField::Submit;
            }
        }
        TranscribeField::Submit => {
            if key.code == KeyCode::Enter {
                return submit_transcribe(app);
            }
        }
    }
    None
}

pub(super) fn handle_clone(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
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
            KeyCode::Left | KeyCode::Up => {
                app.clone_state.model_index = shifted_index(
                    app.clone_state.model_index,
                    app.clone_state.model_indexes.len(),
                    -1,
                )
            }
            KeyCode::Right | KeyCode::Down => {
                app.clone_state.model_index = shifted_index(
                    app.clone_state.model_index,
                    app.clone_state.model_indexes.len(),
                    1,
                )
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
            KeyCode::Enter => app.clone_state.field = CloneField::Submit,
            _ => {}
        },
        CloneField::Submit => {
            if key.code == KeyCode::Enter {
                return submit_clone(app);
            }
        }
    }
    None
}

pub(super) fn handle_convert(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        app.convert_state.field = if key.code == KeyCode::BackTab {
            app.convert_state.field.previous()
        } else {
            app.convert_state.field.next()
        };
        return None;
    }
    match app.convert_state.field {
        ConvertField::Model => match key.code {
            KeyCode::Left | KeyCode::Up => {
                app.convert_state.model_index = shifted_index(
                    app.convert_state.model_index,
                    app.convert_state.model_indexes.len(),
                    -1,
                )
            }
            KeyCode::Right | KeyCode::Down => {
                app.convert_state.model_index = shifted_index(
                    app.convert_state.model_index,
                    app.convert_state.model_indexes.len(),
                    1,
                )
            }
            KeyCode::Enter => app.convert_state.field = ConvertField::Source,
            _ => {}
        },
        ConvertField::Source => {
            if edit_text(
                &mut app.convert_state.source,
                &mut app.convert_state.source_cursor,
                key,
            ) {
                return None;
            }
            if key.code == KeyCode::Enter {
                app.convert_state.field = ConvertField::Target;
            }
        }
        ConvertField::Target => {
            if edit_text(
                &mut app.convert_state.target,
                &mut app.convert_state.target_cursor,
                key,
            ) {
                return None;
            }
            if key.code == KeyCode::Enter {
                app.convert_state.field = ConvertField::F0Method;
            }
        }
        ConvertField::F0Method => match key.code {
            KeyCode::Left | KeyCode::Up => {
                app.convert_state.f0_method_index =
                    shifted_index(app.convert_state.f0_method_index, F0_METHODS.len(), -1)
            }
            KeyCode::Right | KeyCode::Down => {
                app.convert_state.f0_method_index =
                    shifted_index(app.convert_state.f0_method_index, F0_METHODS.len(), 1)
            }
            KeyCode::Enter => app.convert_state.field = ConvertField::PitchShift,
            _ => {}
        },
        ConvertField::PitchShift => {
            edit_numeric(
                &mut app.convert_state.pitch_shift,
                &mut app.convert_state.pitch_shift_cursor,
                key,
                &mut app.convert_state.field,
            );
        }
        ConvertField::IndexRate => {
            edit_numeric(
                &mut app.convert_state.index_rate,
                &mut app.convert_state.index_rate_cursor,
                key,
                &mut app.convert_state.field,
            );
        }
        ConvertField::RmsMixRate => {
            edit_numeric(
                &mut app.convert_state.rms_mix_rate,
                &mut app.convert_state.rms_mix_rate_cursor,
                key,
                &mut app.convert_state.field,
            );
        }
        ConvertField::Protect => {
            edit_numeric(
                &mut app.convert_state.protect,
                &mut app.convert_state.protect_cursor,
                key,
                &mut app.convert_state.field,
            );
        }
        ConvertField::FilterRadius => {
            edit_numeric(
                &mut app.convert_state.filter_radius,
                &mut app.convert_state.filter_radius_cursor,
                key,
                &mut app.convert_state.field,
            );
        }
        ConvertField::Consent => match key.code {
            KeyCode::Char(' ') => app.convert_state.consent = !app.convert_state.consent,
            KeyCode::Enter => app.convert_state.field = ConvertField::Submit,
            _ => {}
        },
        ConvertField::Submit => {
            if key.code == KeyCode::Enter {
                return submit_convert(app);
            }
        }
    }
    None
}

fn edit_numeric(value: &mut String, cursor: &mut usize, key: KeyEvent, field: &mut ConvertField) {
    if edit_text(value, cursor, key) {
        return;
    }
    if key.code == KeyCode::Enter {
        *field = field.next();
    }
}

pub(super) fn submit_speak(app: &mut App) -> Option<TuiAction> {
    let Some(model) = app.selected_speak_model().cloned() else {
        app.set_status("No TTS model is installed. Install one through the library site or CLI.");
        return None;
    };
    if !model.executable {
        return Some(TuiAction::PullModel(model.id));
    }
    let text = app.speak_text.trim().to_string();
    if text.is_empty() {
        app.set_status("Type some text before generating speech.");
        app.speak_field = SpeakField::Text;
        return None;
    }
    Some(TuiAction::Speak {
        model: model.id,
        voice: app.speak_voice.trim().to_string(),
        text,
    })
}

pub(super) fn submit_transcribe(app: &mut App) -> Option<TuiAction> {
    let Some(model) = app.selected_transcribe_model().cloned() else {
        app.set_status("No STT model is installed. Install one through the library site or CLI.");
        return None;
    };
    if !model.executable {
        return Some(TuiAction::PullModel(model.id));
    }
    let audio = app.transcribe_audio.trim().to_string();
    if audio.is_empty() {
        app.set_status("Enter the path to a local audio file first.");
        app.transcribe_field = TranscribeField::Audio;
        return None;
    }
    Some(TuiAction::Transcribe {
        model: model.id,
        audio,
    })
}

pub(super) fn submit_clone(app: &mut App) -> Option<TuiAction> {
    let Some(model) = app.selected_clone_model().cloned() else {
        app.set_status(
            "No voice-cloning model is installed. Install one through the library site or CLI.",
        );
        return None;
    };
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

pub(super) fn submit_convert(app: &mut App) -> Option<TuiAction> {
    let Some(model) = app.selected_convert_model().cloned() else {
        app.set_status(
            "No voice-conversion model is installed. Install RVC through the library first.",
        );
        return None;
    };
    if !model.executable {
        return Some(TuiAction::PullModel(model.id));
    }
    let source = app.convert_state.source.trim().to_string();
    let target = app.convert_state.target.trim().to_string();
    if source.is_empty() {
        app.set_status("Enter the source-audio path.");
        app.convert_state.field = ConvertField::Source;
        return None;
    }
    if target.is_empty() {
        app.set_status("Enter the target RVC package or checkpoint path.");
        app.convert_state.field = ConvertField::Target;
        return None;
    }
    if !app.convert_state.consent {
        app.set_status("Explicit source and target voice consent is required.");
        app.convert_state.field = ConvertField::Consent;
        return None;
    }

    let pitch_shift = parse_number::<i32>(app, ConvertField::PitchShift, "pitch shift", -24, 24)?;
    let index_rate = parse_float(app, ConvertField::IndexRate, "index rate", 0.0, 1.0)?;
    let rms_mix_rate = parse_float(app, ConvertField::RmsMixRate, "RMS mix rate", 0.0, 1.0)?;
    let protect = parse_float(app, ConvertField::Protect, "protect", 0.0, 0.5)?;
    let filter_radius =
        parse_number::<u32>(app, ConvertField::FilterRadius, "filter radius", 0, 7)?;

    Some(TuiAction::ConvertVoice {
        model: model.id,
        source,
        target,
        f0_method: app.convert_state.f0_method().to_string(),
        pitch_shift,
        index_rate,
        rms_mix_rate,
        protect,
        filter_radius,
    })
}

fn parse_float(
    app: &mut App,
    field: ConvertField,
    label: &str,
    minimum: f32,
    maximum: f32,
) -> Option<f32> {
    let source = match field {
        ConvertField::IndexRate => app.convert_state.index_rate.clone(),
        ConvertField::RmsMixRate => app.convert_state.rms_mix_rate.clone(),
        ConvertField::Protect => app.convert_state.protect.clone(),
        _ => return None,
    };
    match source.trim().parse::<f32>() {
        Ok(value) if value.is_finite() && (minimum..=maximum).contains(&value) => Some(value),
        _ => {
            app.set_status(format!("{label} must be between {minimum} and {maximum}."));
            app.convert_state.field = field;
            None
        }
    }
}

fn parse_number<T>(
    app: &mut App,
    field: ConvertField,
    label: &str,
    minimum: T,
    maximum: T,
) -> Option<T>
where
    T: std::str::FromStr + PartialOrd + Copy + std::fmt::Display,
{
    let source = match field {
        ConvertField::PitchShift => app.convert_state.pitch_shift.clone(),
        ConvertField::FilterRadius => app.convert_state.filter_radius.clone(),
        _ => return None,
    };
    match source.trim().parse::<T>() {
        Ok(value) if value >= minimum && value <= maximum => Some(value),
        _ => {
            app.set_status(format!("{label} must be between {minimum} and {maximum}."));
            app.convert_state.field = field;
            None
        }
    }
}
