use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, SpeakField, TranscribeField, TuiAction, TuiTab};

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
        if key.code == KeyCode::F(1) {
            self.show_help = true;
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
            TuiTab::Runners => self.handle_runners_key(key),
            TuiTab::System => self.handle_system_key(key),
        }
    }

    fn handle_models_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        match key.code {
            KeyCode::Left => self.tab = self.tab.previous(),
            KeyCode::Right | KeyCode::Tab => self.tab = self.tab.next(),
            KeyCode::Up => self.model_index = shifted_index(self.model_index, self.models.len(), -1),
            KeyCode::Down => self.model_index = shifted_index(self.model_index, self.models.len(), 1),
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
                    self.speak_model_index = shifted_index(
                        self.speak_model_index,
                        self.tts_models.len(),
                        -1,
                    )
                }
                KeyCode::Right | KeyCode::Down => {
                    self.speak_model_index = shifted_index(
                        self.speak_model_index,
                        self.tts_models.len(),
                        1,
                    )
                }
                KeyCode::Enter => self.speak_field = SpeakField::Voice,
                KeyCode::Esc => self.tab = TuiTab::Models,
                _ => {}
            },
            SpeakField::Voice => {
                if edit_text(
                    &mut self.speak_voice,
                    &mut self.speak_voice_cursor,
                    key,
                ) {
                    return None;
                }
                match key.code {
                    KeyCode::Enter => self.speak_field = SpeakField::Text,
                    KeyCode::Esc => self.speak_field = SpeakField::Model,
                    _ => {}
                }
            }
            SpeakField::Text => {
                if edit_text(
                    &mut self.speak_text,
                    &mut self.speak_text_cursor,
                    key,
                ) {
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
                KeyCode::Esc => self.speak_field = SpeakField::Text,
                KeyCode::Left | KeyCode::Up => self.speak_field = SpeakField::Text,
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
                KeyCode::Esc => self.transcribe_field = TranscribeField::Audio,
                KeyCode::Left | KeyCode::Up => self.transcribe_field = TranscribeField::Audio,
                _ => {}
            },
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
            KeyCode::Up => self.system_index = shifted_index(self.system_index, self.system.len(), -1),
            KeyCode::Down => self.system_index = shifted_index(self.system_index, self.system.len(), 1),
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
            TuiTab::Runners => self.runner_primary_action(),
            TuiTab::System => self
                .selected_system()
                .map(|row| TuiAction::RunSystem(row.action)),
        }
    }
}

fn edit_text(value: &mut String, cursor: &mut usize, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            let mut characters = value.chars().collect::<Vec<_>>();
            characters.insert((*cursor).min(characters.len()), character);
            *value = characters.into_iter().collect();
            *cursor += 1;
            true
        }
        KeyCode::Left => {
            *cursor = cursor.saturating_sub(1);
            true
        }
        KeyCode::Right => {
            *cursor = (*cursor + 1).min(value.chars().count());
            true
        }
        KeyCode::Home => {
            *cursor = 0;
            true
        }
        KeyCode::End => {
            *cursor = value.chars().count();
            true
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                let mut characters = value.chars().collect::<Vec<_>>();
                *cursor -= 1;
                characters.remove(*cursor);
                *value = characters.into_iter().collect();
            }
            true
        }
        KeyCode::Delete => {
            let mut characters = value.chars().collect::<Vec<_>>();
            if *cursor < characters.len() {
                characters.remove(*cursor);
                *value = characters.into_iter().collect();
            }
            true
        }
        _ => false,
    }
}

fn shifted_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).rem_euclid(len as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_wraps_in_both_directions() {
        assert_eq!(shifted_index(0, 3, -1), 2);
        assert_eq!(shifted_index(2, 3, 1), 0);
        assert_eq!(shifted_index(0, 0, 1), 0);
    }

    #[test]
    fn text_editor_inserts_and_deletes_at_cursor() {
        let mut value = "ac".to_string();
        let mut cursor = 1;
        assert!(edit_text(
            &mut value,
            &mut cursor,
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE)
        ));
        assert_eq!(value, "abc");
        assert!(edit_text(
            &mut value,
            &mut cursor,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)
        ));
        assert_eq!(value, "ac");
    }
}
