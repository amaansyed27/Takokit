use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    app::{
        App, SpeakField, TranscribeField, TuiAction, TuiScreen, HOME_ACTIONS, MANAGE_ACTIONS,
    },
    clone::CloneField,
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
        if key.code == KeyCode::F(1)
            || (!self.screen.accepts_text() && key.code == KeyCode::Char('?'))
        {
            self.show_help = true;
            return None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Enter {
            return self.submit_current();
        }
        if key.code == KeyCode::Esc {
            if self.screen == TuiScreen::Home {
                return Some(TuiAction::Quit);
            }
            self.screen = self.screen.parent();
            return None;
        }
        if !self.screen.accepts_text() && key.code == KeyCode::Char('q') {
            return Some(TuiAction::Quit);
        }

        match self.screen {
            TuiScreen::Home => self.handle_home_key(key),
            TuiScreen::Speak => self.handle_speak_key(key),
            TuiScreen::Transcribe => self.handle_transcribe_key(key),
            TuiScreen::Clone => self.handle_clone_key(key),
            TuiScreen::Manage => self.handle_manage_key(key),
            TuiScreen::Models => self.handle_models_key(key),
            TuiScreen::Runners => self.handle_runners_key(key),
            TuiScreen::System => self.handle_system_key(key),
            TuiScreen::Sessions => self.handle_sessions_key(key),
            TuiScreen::Activity => self.handle_activity_key(key),
        }
    }

    fn handle_home_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Up => {
                self.home_index = shifted_index(self.home_index, HOME_ACTIONS.len(), -1)
            }
            KeyCode::Down | KeyCode::Tab => {
                self.home_index = shifted_index(self.home_index, HOME_ACTIONS.len(), 1)
            }
            KeyCode::Enter => self.open_home_item(self.home_index),
            KeyCode::Char(character @ '1'..='6') => {
                let index = character as usize - '1' as usize;
                self.home_index = index;
                self.open_home_item(index);
            }
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn open_home_item(&mut self, index: usize) {
        self.screen = match index {
            0 => TuiScreen::Speak,
            1 => TuiScreen::Transcribe,
            2 => TuiScreen::Clone,
            3 => TuiScreen::Manage,
            4 => TuiScreen::Sessions,
            _ => TuiScreen::Activity,
        };
    }

    fn handle_manage_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Up => {
                self.manage_index = shifted_index(self.manage_index, MANAGE_ACTIONS.len(), -1)
            }
            KeyCode::Down | KeyCode::Tab => {
                self.manage_index = shifted_index(self.manage_index, MANAGE_ACTIONS.len(), 1)
            }
            KeyCode::Enter => self.open_manage_item(self.manage_index),
            KeyCode::Char(character @ '1'..='3') => {
                let index = character as usize - '1' as usize;
                self.manage_index = index;
                self.open_manage_item(index);
            }
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn open_manage_item(&mut self, index: usize) {
        self.screen = match index {
            0 => TuiScreen::Models,
            1 => TuiScreen::Runners,
            _ => TuiScreen::System,
        };
    }

    fn handle_models_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Up => {
                self.model_index = shifted_index(self.model_index, self.models.len(), -1)
            }
            KeyCode::Down => {
                self.model_index = shifted_index(self.model_index, self.models.len(), 1)
            }
            KeyCode::Enter => return self.open_or_repair_selected_model(),
            KeyCode::Char('p') => {
                return self
                    .selected_model()
                    .map(|model| TuiAction::PullModel(model.id.clone()));
            }
            KeyCode::Char('x') => {
                return self
                    .selected_model()
                    .map(|model| TuiAction::RemoveModel(model.id.clone()));
            }
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn handle_runners_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
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
                    .map(|runner| TuiAction::PullRunner(runner.id.clone()));
            }
            KeyCode::Char('i') => {
                return self
                    .selected_runner()
                    .map(|runner| TuiAction::InstallRunner(runner.id.clone()));
            }
            KeyCode::Char('d') => {
                return self
                    .selected_runner()
                    .map(|runner| TuiAction::DoctorRunner(runner.id.clone()));
            }
            KeyCode::Char('x') => {
                return self
                    .selected_runner()
                    .map(|runner| TuiAction::RemoveRunner(runner.id.clone()));
            }
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn handle_system_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Up => {
                self.system_index = shifted_index(self.system_index, self.system.len(), -1)
            }
            KeyCode::Down => {
                self.system_index = shifted_index(self.system_index, self.system.len(), 1)
            }
            KeyCode::Enter => {
                return self
                    .selected_system()
                    .map(|row| TuiAction::RunSystem(row.action));
            }
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn handle_sessions_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Up => {
                self.session_index = shifted_index(self.session_index, self.sessions.len(), -1)
            }
            KeyCode::Down => {
                self.session_index = shifted_index(self.session_index, self.sessions.len(), 1)
            }
            KeyCode::Enter => {
                return self
                    .selected_session()
                    .map(|session| TuiAction::OpenSession(session.id));
            }
            KeyCode::Char('n') => return Some(TuiAction::NewSession),
            KeyCode::Char('r') => return Some(TuiAction::Refresh),
            _ => {}
        }
        None
    }

    fn handle_activity_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::PageUp | KeyCode::Up => {
                self.output_scroll = self.output_scroll.saturating_sub(3)
            }
            KeyCode::PageDown | KeyCode::Down => {
                self.output_scroll = self.output_scroll.saturating_add(3)
            }
            KeyCode::Home => self.output_scroll = 0,
            KeyCode::End => self.output_scroll = u16::MAX,
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
                _ => {}
            },
            SpeakField::Voice => {
                if edit_text(&mut self.speak_voice, &mut self.speak_voice_cursor, key) {
                    return None;
                }
                if key.code == KeyCode::Enter {
                    self.speak_field = SpeakField::Text;
                }
            }
            SpeakField::Text => {
                if edit_text(&mut self.speak_text, &mut self.speak_text_cursor, key) {
                    return None;
                }
                if key.code == KeyCode::Enter {
                    self.speak_field = SpeakField::Submit;
                }
            }
            SpeakField::Submit => {
                if key.code == KeyCode::Enter {
                    return self.submit_speak();
                }
            }
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
                    self.transcribe_model_index = shifted_index(
                        self.transcribe_model_index,
                        self.stt_models.len(),
                        -1,
                    )
                }
                KeyCode::Right | KeyCode::Down => {
                    self.transcribe_model_index = shifted_index(
                        self.transcribe_model_index,
                        self.stt_models.len(),
                        1,
                    )
                }
                KeyCode::Enter => self.transcribe_field = TranscribeField::Audio,
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
                if key.code == KeyCode::Enter {
                    self.transcribe_field = TranscribeField::Submit;
                }
            }
            TranscribeField::Submit => {
                if key.code == KeyCode::Enter {
                    return self.submit_transcribe();
                }
            }
        }
        None
    }

    fn handle_clone_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
            self.clone_state.field = if key.code == KeyCode::BackTab {
                self.clone_state.field.previous()
            } else {
                self.clone_state.field.next()
            };
            return None;
        }
        match self.clone_state.field {
            CloneField::Model => match key.code {
                KeyCode::Left | KeyCode::Up => {
                    self.clone_state.model_index = shifted_index(
                        self.clone_state.model_index,
                        self.clone_state.model_indexes.len(),
                        -1,
                    )
                }
                KeyCode::Right | KeyCode::Down => {
                    self.clone_state.model_index = shifted_index(
                        self.clone_state.model_index,
                        self.clone_state.model_indexes.len(),
                        1,
                    )
                }
                KeyCode::Enter => self.clone_state.field = CloneField::Name,
                _ => {}
            },
            CloneField::Name => {
                if edit_text(
                    &mut self.clone_state.name,
                    &mut self.clone_state.name_cursor,
                    key,
                ) {
                    return None;
                }
                if key.code == KeyCode::Enter {
                    self.clone_state.field = CloneField::Sample;
                }
            }
            CloneField::Sample => {
                if edit_text(
                    &mut self.clone_state.sample,
                    &mut self.clone_state.sample_cursor,
                    key,
                ) {
                    return None;
                }
                if key.code == KeyCode::Enter {
                    self.clone_state.field = CloneField::Consent;
                }
            }
            CloneField::Consent => match key.code {
                KeyCode::Char(' ') => self.clone_state.consent = !self.clone_state.consent,
                KeyCode::Enter => self.clone_state.field = CloneField::Submit,
                _ => {}
            },
            CloneField::Submit => {
                if key.code == KeyCode::Enter {
                    return self.submit_clone();
                }
            }
        }
        None
    }

    fn open_or_repair_selected_model(&mut self) -> Option<TuiAction> {
        let model = self.selected_model()?.clone();
        if !model.executable {
            return Some(TuiAction::PullModel(model.id));
        }
        if model.tts {
            self.set_speak_model(&model.id);
            self.screen = TuiScreen::Speak;
            self.speak_field = SpeakField::Text;
            self.set_status("Model selected. Type the text you want Takokit to speak.");
        } else if model.stt {
            self.set_transcribe_model(&model.id);
            self.screen = TuiScreen::Transcribe;
            self.transcribe_field = TranscribeField::Audio;
            self.set_status("Model selected. Enter the local audio file path.");
        } else if model.voice_cloning {
            self.screen = TuiScreen::Clone;
            self.set_status("Model selected. Enter a profile name and reference audio path.");
        } else {
            self.set_status("This model is installed, but it has no interactive TUI task.");
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

    fn submit_speak(&mut self) -> Option<TuiAction> {
        let Some(model) = self.selected_speak_model().cloned() else {
            self.set_status("No TTS model is installed. Install one through the library site or CLI.");
            return None;
        };
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

    fn submit_transcribe(&mut self) -> Option<TuiAction> {
        let Some(model) = self.selected_transcribe_model().cloned() else {
            self.set_status("No STT model is installed. Install one through the library site or CLI.");
            return None;
        };
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

    fn submit_clone(&mut self) -> Option<TuiAction> {
        let Some(model) = self.selected_clone_model().cloned() else {
            self.set_status(
                "No voice-cloning model is installed. Install one through the library site or CLI.",
            );
            return None;
        };
        if !model.executable {
            return Some(TuiAction::PullModel(model.id));
        }
        let name = self.clone_state.name.trim().to_string();
        let sample = self.clone_state.sample.trim().to_string();
        if name.is_empty() {
            self.set_status("Enter a profile name before creating the voice.");
            self.clone_state.field = CloneField::Name;
            return None;
        }
        if sample.is_empty() {
            self.set_status("Enter a local reference-audio path.");
            self.clone_state.field = CloneField::Sample;
            return None;
        }
        if !self.clone_state.consent {
            self.set_status("Explicit voice-owner consent is required.");
            self.clone_state.field = CloneField::Consent;
            return None;
        }
        Some(TuiAction::CloneVoice {
            model: model.id,
            name,
            sample,
        })
    }

    fn submit_current(&mut self) -> Option<TuiAction> {
        match self.screen {
            TuiScreen::Home => {
                self.open_home_item(self.home_index);
                None
            }
            TuiScreen::Speak => self.submit_speak(),
            TuiScreen::Transcribe => self.submit_transcribe(),
            TuiScreen::Clone => self.submit_clone(),
            TuiScreen::Manage => {
                self.open_manage_item(self.manage_index);
                None
            }
            TuiScreen::Models => self.open_or_repair_selected_model(),
            TuiScreen::Runners => self.runner_primary_action(),
            TuiScreen::System => self
                .selected_system()
                .map(|row| TuiAction::RunSystem(row.action)),
            TuiScreen::Sessions => self
                .selected_session()
                .map(|session| TuiAction::OpenSession(session.id)),
            TuiScreen::Activity => None,
        }
    }
}
