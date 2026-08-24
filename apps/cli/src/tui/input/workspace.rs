use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{
    app::{App, TuiAction, WorkspaceField},
    editor::edit_text,
};

use super::{normalize_path_field, picker};

pub(super) fn handle_workspace(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        app.workspace_field = if key.code == KeyCode::BackTab {
            app.workspace_field.previous()
        } else {
            app.workspace_field.next()
        };
        return None;
    }

    match app.workspace_field {
        WorkspaceField::Path => {
            if key.code == KeyCode::F(2) {
                match picker::pick_folder(Path::new(&app.workspace_root)) {
                    Ok(Some(path)) => {
                        app.workspace_input = path.display().to_string();
                        app.workspace_input_cursor = app.workspace_input.chars().count();
                    }
                    Ok(None) => {}
                    Err(error) => app.set_status(error),
                }
                return None;
            }
            if edit_text(
                &mut app.workspace_input,
                &mut app.workspace_input_cursor,
                key,
            ) {
                return None;
            }
            if key.code == KeyCode::Enter {
                app.workspace_field = WorkspaceField::Apply;
            }
        }
        WorkspaceField::Apply => {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                let path = normalize_path_field(&app.workspace_input);
                if path.is_empty() {
                    app.workspace_field = WorkspaceField::Path;
                    app.set_status("Browse, paste, drag, or enter a workspace folder path.");
                    return None;
                }
                return Some(TuiAction::ChangeWorkspace(path));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_submit_action_preserves_unicode_and_spaces() {
        let value = "D:\\Voice Projects\\आवाज़".to_string();
        let action = TuiAction::ChangeWorkspace(value.clone());
        assert_eq!(action, TuiAction::ChangeWorkspace(value));
    }

    #[test]
    fn workspace_accepts_dragged_quoted_path() {
        assert_eq!(
            normalize_path_field(r#""D:\Voice Projects\आवाज़""#),
            r#"D:\Voice Projects\आवाज़"#
        );
    }
}
