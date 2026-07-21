use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{
    app::{App, TuiAction, TuiScreen, HOME_ACTIONS, MANAGE_ACTIONS},
    editor::shifted_index,
};

pub(super) fn handle_home(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    match key.code {
        KeyCode::Up => app.home_index = shifted_index(app.home_index, HOME_ACTIONS.len(), -1),
        KeyCode::Down | KeyCode::Tab => {
            app.home_index = shifted_index(app.home_index, HOME_ACTIONS.len(), 1)
        }
        KeyCode::Enter => open_home_item(app, app.home_index),
        KeyCode::Char(character @ '1'..='6') => {
            let index = character as usize - '1' as usize;
            app.home_index = index;
            open_home_item(app, index);
        }
        KeyCode::Char('r') => return Some(TuiAction::Refresh),
        _ => {}
    }
    None
}

pub(super) fn open_home_item(app: &mut App, index: usize) {
    app.screen = match index {
        0 => TuiScreen::Speak,
        1 => TuiScreen::Transcribe,
        2 => TuiScreen::Clone,
        3 => TuiScreen::Manage,
        4 => TuiScreen::Sessions,
        _ => TuiScreen::Activity,
    };
}

pub(super) fn handle_manage(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    match key.code {
        KeyCode::Up => app.manage_index = shifted_index(app.manage_index, MANAGE_ACTIONS.len(), -1),
        KeyCode::Down | KeyCode::Tab => {
            app.manage_index = shifted_index(app.manage_index, MANAGE_ACTIONS.len(), 1)
        }
        KeyCode::Enter => open_manage_item(app, app.manage_index),
        KeyCode::Char(character @ '1'..='3') => {
            let index = character as usize - '1' as usize;
            app.manage_index = index;
            open_manage_item(app, index);
        }
        KeyCode::Char('r') => return Some(TuiAction::Refresh),
        _ => {}
    }
    None
}

pub(super) fn open_manage_item(app: &mut App, index: usize) {
    app.screen = match index {
        0 => TuiScreen::Models,
        1 => TuiScreen::Runners,
        _ => TuiScreen::System,
    };
}

pub(super) fn handle_models(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    match key.code {
        KeyCode::Up => app.model_index = shifted_index(app.model_index, app.models.len(), -1),
        KeyCode::Down => app.model_index = shifted_index(app.model_index, app.models.len(), 1),
        KeyCode::Enter => return open_or_repair_selected_model(app),
        KeyCode::Char('p') => {
            return app
                .selected_model()
                .map(|model| TuiAction::PullModel(model.id.clone()));
        }
        KeyCode::Char('x') => {
            return app
                .selected_model()
                .map(|model| TuiAction::RemoveModel(model.id.clone()));
        }
        KeyCode::Char('r') => return Some(TuiAction::Refresh),
        _ => {}
    }
    None
}

pub(super) fn handle_runners(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    match key.code {
        KeyCode::Up => app.runner_index = shifted_index(app.runner_index, app.runners.len(), -1),
        KeyCode::Down => app.runner_index = shifted_index(app.runner_index, app.runners.len(), 1),
        KeyCode::Enter => return runner_primary_action(app),
        KeyCode::Char('p') => {
            return app
                .selected_runner()
                .map(|runner| TuiAction::PullRunner(runner.id.clone()));
        }
        KeyCode::Char('i') => {
            return app
                .selected_runner()
                .map(|runner| TuiAction::InstallRunner(runner.id.clone()));
        }
        KeyCode::Char('d') => {
            return app
                .selected_runner()
                .map(|runner| TuiAction::DoctorRunner(runner.id.clone()));
        }
        KeyCode::Char('x') => {
            return app
                .selected_runner()
                .map(|runner| TuiAction::RemoveRunner(runner.id.clone()));
        }
        KeyCode::Char('r') => return Some(TuiAction::Refresh),
        _ => {}
    }
    None
}

pub(super) fn handle_system(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    match key.code {
        KeyCode::Up => app.system_index = shifted_index(app.system_index, app.system.len(), -1),
        KeyCode::Down => app.system_index = shifted_index(app.system_index, app.system.len(), 1),
        KeyCode::Enter => {
            return app
                .selected_system()
                .map(|row| TuiAction::RunSystem(row.action));
        }
        KeyCode::Char('r') => return Some(TuiAction::Refresh),
        _ => {}
    }
    None
}

pub(super) fn handle_sessions(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    match key.code {
        KeyCode::Up => app.session_index = shifted_index(app.session_index, app.sessions.len(), -1),
        KeyCode::Down => {
            app.session_index = shifted_index(app.session_index, app.sessions.len(), 1)
        }
        KeyCode::Enter => {
            return app
                .selected_session()
                .map(|session| TuiAction::OpenSession(session.id));
        }
        KeyCode::Char('n') => return Some(TuiAction::NewSession),
        KeyCode::Char('r') => return Some(TuiAction::Refresh),
        _ => {}
    }
    None
}

pub(super) fn handle_activity(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    match key.code {
        KeyCode::PageUp | KeyCode::Up => app.output_scroll = app.output_scroll.saturating_sub(3),
        KeyCode::PageDown | KeyCode::Down => {
            app.output_scroll = app.output_scroll.saturating_add(3)
        }
        KeyCode::Home => app.output_scroll = 0,
        KeyCode::End => app.output_scroll = u16::MAX,
        KeyCode::Char('r') => return Some(TuiAction::Refresh),
        _ => {}
    }
    None
}

pub(super) fn open_or_repair_selected_model(app: &mut App) -> Option<TuiAction> {
    let model = app.selected_model()?.clone();
    if !model.executable {
        return Some(TuiAction::PullModel(model.id));
    }
    if model.tts {
        app.set_speak_model(&model.id);
        app.screen = TuiScreen::Speak;
        app.speak_field = crate::tui::app::SpeakField::Text;
        app.set_status("Model selected. Type the text you want Takokit to speak.");
    } else if model.stt {
        app.set_transcribe_model(&model.id);
        app.screen = TuiScreen::Transcribe;
        app.transcribe_field = crate::tui::app::TranscribeField::Audio;
        app.set_status("Model selected. Enter the local audio file path.");
    } else if model.voice_cloning {
        app.screen = TuiScreen::Clone;
        app.set_status("Model selected. Enter a profile name and reference audio path.");
    } else {
        app.set_status("This model is installed, but it has no interactive TUI task.");
    }
    None
}

pub(super) fn runner_primary_action(app: &App) -> Option<TuiAction> {
    let runner = app.selected_runner()?;
    Some(if runner.ready {
        TuiAction::DoctorRunner(runner.id.clone())
    } else if runner.installed {
        TuiAction::InstallRunner(runner.id.clone())
    } else {
        TuiAction::PullRunner(runner.id.clone())
    })
}
