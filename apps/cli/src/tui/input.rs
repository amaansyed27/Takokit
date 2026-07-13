use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    app::{App, SpeakField, TranscribeField, TuiAction, TuiTab},
    editor::{edit_text, shifted_index},
};

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(TuiAction::Quit);
        }
        if self.show_help {
            if matches!(key.code, KeyCode::Esc | KeyCode::F(1) | KeyCode::Enter) {
                self.show_help = false;
            }
            return None;
        }
        if self.slash_open {
            return self.handle_slash_key(key);
        }
        if key.code == KeyCode::F(1) {
            self.show_help = true;
            return None;
        }
        if key.code == KeyCode::Char('/') && self.slash_menu_available() {
            self.slash_open = true;
            self.slash_input.clear();
            self.slash_cursor = 0;
            return None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Left => {
                    self.tab = self.tab.previous();
                    return None;
                }
                KeyCode::Right => {
                    self.tab = self.tab.next();
                    return None;
                }
                KeyCode::Enter => return self.primary_action(),
                KeyCode::Char('r') => return Some(TuiAction::Refresh),
                _ => {}
            }
        }
        if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
            self.output_scroll = match key.code {
                KeyCode::PageUp => self.output_scroll.saturating_sub(5),
                _ => self.output_scroll.saturating_add(5),
            };
            return None;
        }
        match self.tab {
            TuiTab::Models => self.handle_models_key(key),
            TuiTab::Speak => self.handle_speak_key(key),
            TuiTab::Transcribe => self.handle_transcribe_key(key),
            TuiTab::Sessions => self.handle_sessions_key(key),
            TuiTab::Runners => self.handle_runners_key(key),
            TuiTab::System => self.handle_system_key(key),
        }
    }

    fn handle_slash_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Esc => self.close_slash(),
            KeyCode::Enter => return self.submit_slash(),
            _ => {
                let _ = edit_text(&mut self.slash_input, &mut self.slash_cursor, key);
            }
        }
        None
    }

    fn submit_slash(&mut self) -> Option<TuiAction> {
        let command = self.slash_input.trim().to_ascii_lowercase();
        self.close_slash();
        match command.as_str() {
            "sessions" | "history" => self.tab = TuiTab::Sessions,
            "new" | "new-session" => return Some(TuiAction::NewSession),
            "models" => self.tab = TuiTab::Models,
            "speak" => self.tab = TuiTab::Speak,
            "transcribe" | "stt" => self.tab = TuiTab::Transcribe,
            "runners" => self.tab = TuiTab::Runners,
            "system" | "doctor" => self.tab = TuiTab::System,
            "help" | "?" => self.show_help = true,
            "" => {}
            _ => self.set_status(format!(
                "Unknown shortcut /{command}. Use /sessions, /new, /models, /speak, /transcribe, /runners, /system, or /help."
            )),
        }
        None
    }

    fn close_slash(&mut self) {
        self.slash_open = false;
        self.slash_input.clear();
        self.slash_cursor = 0;
    }

    fn slash_menu_available(&self) -> bool {
        match self.tab {
            TuiTab::Speak => !matches!(self.speak_field, SpeakField::Voice | SpeakField::Text),
            TuiTab::Transcribe => self.transcribe_field != TranscribeField::Audio,
            _ => true,
        }
    }

    fn handle_models_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Left => self.tab = self.tab.previous(),
            KeyCode::Right | KeyCode::Tab => self.tab = self.tab.next(),
            KeyCode::Up => {
                self.model_index = shifted_index(self.model_index, self.models.len(), -1)
            }
            KeyCode::Down => {
                self.model_index = shifted_index(self.model_index, self.models.len(), 1)
            }
            KeyCode::Enter => return self.open_or_install_selected_model(),
            KeyCode::Char('p') => {
                return self
                    .selected_model()
                    .map(|model| TuiAction::PullModel(model.id.clone()))
            }
            KeyCode::Char('x') => {
                return self
                    .selected_model()
                    .map(|model| TuiAction::RemoveModel(model.id.clone()))
            }
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn handle_speak_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
            self.speak_field = if key.code == KeyCode::BackTab {
                self.speak_field.previous()
            } else {
                self.speak_field.next()
            };
            return None;
        }
        match self.speak_field {
            SpeakField::Model => match key.code {
                KeyCode::Left | KeyCode::Up => {
                    self.speak_model_index =
                        shifted_index(self.speak_model_index, self.tts_models.len(), -1)
                }
                KeyCode::Right | KeyCode::Down => {
                    self.speak_model_index =
                        shifted_index(self.speak_model_index, self.tts_models.len(), 1)
                }
                KeyCode::Enter => self.speak_field = SpeakField::Voice,
                KeyCode::Esc => self.tab = TuiTab::Models,
                _ => {}
            },
            SpeakField::Voice => {
                if edit_text(&mut self.speak_voice, &mut self.speak_voice_cursor, key) {
                    return None;
                }
                match key.code {
                    KeyCode::Enter => self.speak_field = SpeakField::Text,
                    KeyCode::Esc => self.speak_field = SpeakField::Model,
                    _ => {}
                }
            }
            SpeakField::Text => {
                if edit_text(&mut self.speak_text, &mut self.speak_text_cursor, key) {
                    return None;
                }
                match key.code {
                    KeyCode::Enter => self.speak_field = SpeakField::Primary,
                    KeyCode::Esc => self.speak_field = SpeakField::Model,
                    _ => {}
                }
            }
            SpeakField::Primary => match key.code {
                KeyCode::Enter => return self.primary_action(),
                KeyCode::Esc | KeyCode::Left | KeyCode::Up => {
                    self.speak_field = SpeakField::Text
                }
                _ => {}
            },
        }
        None
    }

    fn handle_transcribe_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
            self.transcribe_field = if key.code == KeyCode::BackTab {
                self.transcribe_field.previous()
            } else {
                self.transcribe_field.next()
            };
            return None;
        }
        match self.transcribe_field {
            TranscribeField::Model => match key.code {
                KeyCode::Left | KeyCode::Up => {
                    self.transcribe_model_index =
                        shifted_index(self.transcribe_model_index, self.stt_models.len(), -1)
                }
                KeyCode::Right | KeyCode::Down => {
                    self.transcribe_model_index =
                        shifted_index(self.transcribe_model_index, self.stt_models.len(), 1)
                }
                KeyCode::Enter => self.transcribe_field = TranscribeField::Audio,
                KeyCode::Esc => self.tab = TuiTab::Models,
                _ => {}
            },
            TranscribeField::Audio => {
                if edit_text(
                    &mut self.transcribe_audio,
                    &mut self.transcribe_audio_cursor,
                    key,
                ) {
                    return None;
                }
                match key.code {
                    KeyCode::Enter => self.transcribe_field = TranscribeField::Primary,
                    KeyCode::Esc => self.transcribe_field = TranscribeField::Model,
                    _ => {}
                }
            }
            TranscribeField::Primary => match key.code {
                KeyCode::Enter => return self.primary_action(),
                KeyCode::Esc | KeyCode::Left | KeyCode::Up => {
                    self.transcribe_field = TranscribeField::Audio
                }
                _ => {}
            },
        }
        None
    }

    fn handle_sessions_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Left => self.tab = self.tab.previous(),
            KeyCode::Right | KeyCode::Tab => self.tab = self.tab.next(),
            KeyCode::Up => {
                self.session_index = shifted_index(self.session_index, self.sessions.len(), -1)
            }
            KeyCode::Down => {
                self.session_index = shifted_index(self.session_index, self.sessions.len(), 1)
            }
            KeyCode::Enter => {
                return self
                    .selected_session()
                    .map(|session| TuiAction::OpenSession(session.id))
            }
            KeyCode::Char('n') => return Some(TuiAction::NewSession),
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn handle_runners_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Left => self.tab = self.tab.previous(),
            KeyCode::Right | KeyCode::Tab => self.tab = self.tab.next(),
            KeyCode::Up => {
                self.runner_index = shifted_index(self.runner_index, self.runners.len(), -1)
            }
            KeyCode::Down => {
                self.runner_index = shifted_index(self.runner_index, self.runners.len(), 1)
            }
            KeyCode::Enter => return self.runner_primary_action(),
            KeyCode::Char('p') => {
                return self
                    .selected_runner()
                    .map(|runner| TuiAction::PullRunner(runner.id.clone()))
            }
            KeyCode::Char('i') => {
                return self
                    .selected_runner()
                    .map(|runner| TuiAction::InstallRunner(runner.id.clone()))
            }
            KeyCode::Char('d') => {
                return self
                    .selected_runner()
                    .map(|runner| TuiAction::DoctorRunner(runner.id.clone()))
            }
            KeyCode::Char('x') => {
                return self
                    .selected_runner()
                    .map(|runner| TuiAction::RemoveRunner(runner.id.clone()))
            }
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn handle_system_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Left => self.tab = self.tab.previous(),
            KeyCode::Right | KeyCode::Tab => self.tab = self.tab.next(),
            KeyCode::Up => {
                self.system_index = shifted_index(self.system_index, self.system.len(), -1)
            }
            KeyCode::Down => {
                self.system_index = shifted_index(self.system_index, self.system.len(), 1)
            }
            KeyCode::Enter => {
                return self
                    .selected_system()
                    .map(|row| TuiAction::RunSystem(row.action))
            }
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn open_or_install_selected_model(&mut self) -> Option<TuiAction> {
        let model = self.selected_model()?.clone();
        if !model.executable {
            return Some(TuiAction::PullModel(model.id));
        }
        if model.tts {
            self.set_speak_model(&model.id);
            self.tab = TuiTab::Speak;
            self.speak_field = SpeakField::Text;
            self.set_status("Model selected. Type the text you want Takokit to speak.");
        } else if model.stt {
            self.set_transcribe_model(&model.id);
            self.tab = TuiTab::Transcribe;
            self.transcribe_field = TranscribeField::Audio;
            self.set_status("Model selected. Enter the local audio file path.");
        } else {
            self.set_status("This model is ready, but its interactive task is not available yet.");
        }
        None
    }

    fn runner_primary_action(&self) -> Option<TuiAction> {
        let runner = self.selected_runner()?;
        Some(if runner.ready {
            TuiAction::DoctorRunner(runner.id.clone())
        } else if runner.installed {
            TuiAction::InstallRunner(runner.id.clone())
        } else {
            TuiAction::PullRunner(runner.id.clone())
        })
    }

    fn primary_action(&mut self) -> Option<TuiAction> {
        match self.tab {
            TuiTab::Models => self.open_or_install_selected_model(),
            TuiTab::Speak => {
                let model = self.selected_speak_model()?.clone();
                if !model.executable {
                    return Some(TuiAction::PullModel(model.id));
                }
                let text = self.speak_text.trim().to_string();
                if text.is_empty() {
                    self.set_status("Type some text before generating speech.");
                    self.speak_field = SpeakField::Text;
                    return None;
                }
                Some(TuiAction::Speak {
                    model: model.id,
                    voice: self.speak_voice.trim().to_string(),
                    text,
                })
            }
            TuiTab::Transcribe => {
                let model = self.selected_transcribe_model()?.clone();
                if !model.executable {
                    return Some(TuiAction::PullModel(model.id));
                }
                let audio = self.transcribe_audio.trim().to_string();
                if audio.is_empty() {
                    self.set_status("Enter the path to a local audio file first.");
                    self.transcribe_field = TranscribeField::Audio;
                    return None;
                }
                Some(TuiAction::Transcribe {
                    model: model.id,
                    audio,
                })
            }
            TuiTab::Sessions => self
                .selected_session()
                .map(|session| TuiAction::OpenSession(session.id)),
            TuiTab::Runners => self.runner_primary_action(),
            TuiTab::System => self
                .selected_system()
                .map(|row| TuiAction::RunSystem(row.action)),
        }
    }
}
