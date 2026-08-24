mod forms;
mod navigation;
mod picker;
mod workspace;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    app::{App, SpeakField, TranscribeField, TuiAction, TuiScreen, WorkspaceField},
    clone::CloneField,
    convert::ConvertField,
    editor::insert_text,
};

impl App {
    pub(super) fn handle_paste(&mut self, text: &str) {
        let multiline = normalize_multiline_paste(text);
        let single_line = normalize_single_line_paste(text);

        match self.screen {
            TuiScreen::Speak => match self.speak_field {
                SpeakField::Voice => insert_text(
                    &mut self.speak_voice,
                    &mut self.speak_voice_cursor,
                    &single_line,
                ),
                SpeakField::Text => insert_text(
                    &mut self.speak_text,
                    &mut self.speak_text_cursor,
                    &multiline,
                ),
                _ => {}
            },
            TuiScreen::Transcribe if self.transcribe_field == TranscribeField::Audio => {
                insert_text(
                    &mut self.transcribe_audio,
                    &mut self.transcribe_audio_cursor,
                    &single_line,
                );
            }
            TuiScreen::Clone => match self.clone_state.field {
                CloneField::Name => insert_text(
                    &mut self.clone_state.name,
                    &mut self.clone_state.name_cursor,
                    &single_line,
                ),
                CloneField::Sample => insert_text(
                    &mut self.clone_state.sample,
                    &mut self.clone_state.sample_cursor,
                    &single_line,
                ),
                _ => {}
            },
            TuiScreen::Convert => match self.convert_state.field {
                ConvertField::Source => insert_text(
                    &mut self.convert_state.source,
                    &mut self.convert_state.source_cursor,
                    &single_line,
                ),
                ConvertField::Target => insert_text(
                    &mut self.convert_state.target,
                    &mut self.convert_state.target_cursor,
                    &single_line,
                ),
                ConvertField::PitchShift => insert_text(
                    &mut self.convert_state.pitch_shift,
                    &mut self.convert_state.pitch_shift_cursor,
                    &single_line,
                ),
                ConvertField::IndexRate => insert_text(
                    &mut self.convert_state.index_rate,
                    &mut self.convert_state.index_rate_cursor,
                    &single_line,
                ),
                ConvertField::RmsMixRate => insert_text(
                    &mut self.convert_state.rms_mix_rate,
                    &mut self.convert_state.rms_mix_rate_cursor,
                    &single_line,
                ),
                ConvertField::Protect => insert_text(
                    &mut self.convert_state.protect,
                    &mut self.convert_state.protect_cursor,
                    &single_line,
                ),
                ConvertField::FilterRadius => insert_text(
                    &mut self.convert_state.filter_radius,
                    &mut self.convert_state.filter_radius_cursor,
                    &single_line,
                ),
                _ => {}
            },
            TuiScreen::Workspace if self.workspace_field == WorkspaceField::Path => {
                insert_text(
                    &mut self.workspace_input,
                    &mut self.workspace_input_cursor,
                    &single_line,
                );
            }
            _ => {}
        }
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(TuiAction::Quit);
        }
        if self.pending_confirmation.is_some() {
            return match key.code {
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_pending(),
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.cancel_confirmation();
                    self.set_status("Destructive operation cancelled.");
                    None
                }
                _ => None,
            };
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
        if !self.screen.accepts_text() && key.code == KeyCode::Char('w') {
            self.screen = TuiScreen::Workspace;
            self.workspace_field = WorkspaceField::Path;
            return None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Enter {
            return submit_current(self);
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
            TuiScreen::Home => navigation::handle_home(self, key),
            TuiScreen::Speak => forms::handle_speak(self, key),
            TuiScreen::Transcribe => forms::handle_transcribe(self, key),
            TuiScreen::Clone => forms::handle_clone(self, key),
            TuiScreen::Convert => forms::handle_convert(self, key),
            TuiScreen::Manage => navigation::handle_manage(self, key),
            TuiScreen::Models => navigation::handle_models(self, key),
            TuiScreen::Runners => navigation::handle_runners(self, key),
            TuiScreen::System => navigation::handle_system(self, key),
            TuiScreen::Sessions => navigation::handle_sessions(self, key),
            TuiScreen::Workspace => workspace::handle_workspace(self, key),
            TuiScreen::Activity => navigation::handle_activity(self, key),
        }
    }
}

fn submit_current(app: &mut App) -> Option<TuiAction> {
    match app.screen {
        TuiScreen::Home => {
            navigation::open_home_item(app, app.home_index);
            None
        }
        TuiScreen::Speak => forms::submit_speak(app),
        TuiScreen::Transcribe => forms::submit_transcribe(app),
        TuiScreen::Clone => forms::submit_clone(app),
        TuiScreen::Convert => forms::submit_convert(app),
        TuiScreen::Manage => {
            navigation::open_manage_item(app, app.manage_index);
            None
        }
        TuiScreen::Models => navigation::open_or_repair_selected_model(app),
        TuiScreen::Runners => navigation::runner_primary_action(app),
        TuiScreen::System => app
            .selected_system()
            .map(|row| TuiAction::RunSystem(row.action)),
        TuiScreen::Sessions => app
            .selected_session()
            .map(|session| TuiAction::OpenSession(session.id)),
        TuiScreen::Workspace => {
            let path = normalize_path_field(&app.workspace_input);
            if path.is_empty() {
                app.workspace_field = WorkspaceField::Path;
                app.set_status("Paste, drag, browse, or enter a workspace folder path.");
                None
            } else {
                Some(TuiAction::ChangeWorkspace(path))
            }
        }
        TuiScreen::Activity => None,
    }
}

fn normalize_multiline_paste(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn normalize_single_line_paste(value: &str) -> String {
    normalize_multiline_paste(value)
        .replace('\n', " ")
        .replace('\t', " ")
        .trim()
        .to_string()
}

pub(super) fn normalize_path_field(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_screen_accepts_text() {
        assert!(TuiScreen::Workspace.accepts_text());
    }

    #[test]
    fn pasted_path_quotes_are_removed() {
        assert_eq!(
            normalize_path_field(r#""C:\Voice Projects\Demo""#),
            r#"C:\Voice Projects\Demo"#
        );
        assert_eq!(normalize_path_field("../other-project"), "../other-project");
    }

    #[test]
    fn multiline_paste_keeps_paragraph_boundaries() {
        assert_eq!(
            normalize_multiline_paste("First line\r\nSecond ü\rThird"),
            "First line\nSecond ü\nThird"
        );
    }

    #[test]
    fn single_line_paste_cannot_inject_navigation_keys() {
        assert_eq!(
            normalize_single_line_paste("C:\\Voice Project ü\\sample.wav\r\n"),
            "C:\\Voice Project ü\\sample.wav"
        );
        assert_eq!(normalize_single_line_paste("hello\tworld"), "hello world");
    }
}
