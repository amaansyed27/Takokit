mod forms;
mod navigation;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, TuiAction, TuiScreen};

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
            TuiScreen::Manage => navigation::handle_manage(self, key),
            TuiScreen::Models => navigation::handle_models(self, key),
            TuiScreen::Runners => navigation::handle_runners(self, key),
            TuiScreen::System => navigation::handle_system(self, key),
            TuiScreen::Sessions => navigation::handle_sessions(self, key),
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
        TuiScreen::Activity => None,
    }
}
