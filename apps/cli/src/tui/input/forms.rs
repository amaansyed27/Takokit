use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{
    app::{App, SpeakField, TranscribeField, TuiAction},
    clone::CloneField,
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
