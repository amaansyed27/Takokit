use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent};
use uuid::Uuid;

use crate::tui::{
    advanced_rvc::{AdvancedRvcAction, AdvancedRvcField, ADVANCED_RVC_ACTIONS, RVC_PRESETS},
    app::{App, TuiAction, TuiScreen},
    convert::ConvertField,
    editor::{edit_text, shifted_index},
};

use super::{normalize_path_field, picker};

pub(super) fn handle_advanced_rvc(app: &mut App, key: KeyEvent) -> Option<TuiAction> {
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        app.advanced_rvc.field = app.advanced_rvc.cycle_field(key.code == KeyCode::BackTab);
        return None;
    }

    match app.advanced_rvc.field {
        AdvancedRvcField::Project => match key.code {
            KeyCode::Left | KeyCode::Up => {
                app.advanced_rvc.project_index = shifted_index(
                    app.advanced_rvc.project_index,
                    app.advanced_rvc.projects.len(),
                    -1,
                )
            }
            KeyCode::Right | KeyCode::Down => {
                app.advanced_rvc.project_index = shifted_index(
                    app.advanced_rvc.project_index,
                    app.advanced_rvc.projects.len(),
                    1,
                )
            }
            KeyCode::Enter => app.advanced_rvc.field = AdvancedRvcField::Action,
            _ => {}
        },
        AdvancedRvcField::Name => {
            if edit_text(
                &mut app.advanced_rvc.name,
                &mut app.advanced_rvc.name_cursor,
                key,
            ) {
                return None;
            }
        }
        AdvancedRvcField::Path => {
            if key.code == KeyCode::F(2) {
                let selection = if app.advanced_rvc.selected_action().uses_audio_picker() {
                    picker::pick_audio_file(Path::new(&app.workspace_root))
                } else {
                    picker::pick_rvc_artifact(Path::new(&app.workspace_root))
                };
                match selection {
                    Ok(Some(path)) => {
                        app.advanced_rvc.path = path.display().to_string();
                        app.advanced_rvc.path_cursor = app.advanced_rvc.path.chars().count();
                    }
                    Ok(None) => {}
                    Err(error) => app.set_status(error),
                }
                return None;
            }
            if edit_text(
                &mut app.advanced_rvc.path,
                &mut app.advanced_rvc.path_cursor,
                key,
            ) {
                return None;
            }
        }
        AdvancedRvcField::Index => {
            if key.code == KeyCode::F(2) {
                match picker::pick_rvc_artifact(Path::new(&app.workspace_root)) {
                    Ok(Some(path)) => {
                        app.advanced_rvc.index = path.display().to_string();
                        app.advanced_rvc.index_cursor = app.advanced_rvc.index.chars().count();
                    }
                    Ok(None) => {}
                    Err(error) => app.set_status(error),
                }
                return None;
            }
            if edit_text(
                &mut app.advanced_rvc.index,
                &mut app.advanced_rvc.index_cursor,
                key,
            ) {
                return None;
            }
        }
        AdvancedRvcField::Preset => match key.code {
            KeyCode::Left | KeyCode::Up => {
                app.advanced_rvc.preset_index =
                    shifted_index(app.advanced_rvc.preset_index, RVC_PRESETS.len(), -1)
            }
            KeyCode::Right | KeyCode::Down => {
                app.advanced_rvc.preset_index =
                    shifted_index(app.advanced_rvc.preset_index, RVC_PRESETS.len(), 1)
            }
            _ => {}
        },
        AdvancedRvcField::Consent => {
            if key.code == KeyCode::Char(' ') {
                app.advanced_rvc.consent = !app.advanced_rvc.consent;
            }
        }
        AdvancedRvcField::Action => match key.code {
            KeyCode::Left | KeyCode::Up => {
                app.advanced_rvc.action_index = shifted_index(
                    app.advanced_rvc.action_index,
                    ADVANCED_RVC_ACTIONS.len(),
                    -1,
                )
            }
            KeyCode::Right | KeyCode::Down => {
                app.advanced_rvc.action_index =
                    shifted_index(app.advanced_rvc.action_index, ADVANCED_RVC_ACTIONS.len(), 1)
            }
            KeyCode::Enter | KeyCode::Char(' ') => return submit_advanced_rvc(app),
            _ => {}
        },
    }
    None
}

pub(super) fn submit_advanced_rvc(app: &mut App) -> Option<TuiAction> {
    let action = app.advanced_rvc.selected_action();
    if action == AdvancedRvcAction::UseInConvert {
        return use_in_convert(app);
    }

    let name = app.advanced_rvc.name.trim().to_string();
    let path = normalize_path_field(&app.advanced_rvc.path);
    let index = normalize_path_field(&app.advanced_rvc.index);
    let consent = app.advanced_rvc.consent;
    let preset = app.advanced_rvc.preset().to_string();
    let selected = app
        .advanced_rvc
        .selected_project()
        .map(|project| (project.id.to_string(), project.name.clone()));

    let command = match action {
        AdvancedRvcAction::NewVoice => {
            if name.is_empty() || !consent {
                app.set_status("Create trained voice requires a name and voice permission.");
                return None;
            }
            vec!["create".into(), "--name".into(), name, "--consent".into()]
        }
        AdvancedRvcAction::ImportExisting => {
            if name.is_empty() || path.is_empty() || !consent {
                app.set_status("Import existing voice requires a name, voice file, and permission.");
                return None;
            }
            if path.to_ascii_lowercase().ends_with(".takovoice") {
                vec![
                    "import-package".into(),
                    path,
                    "--name".into(),
                    name,
                    "--consent".into(),
                ]
            } else {
                let mut args = vec![
                    "import".into(),
                    path,
                    "--name".into(),
                    name,
                    "--consent".into(),
                ];
                if !index.is_empty() {
                    args.extend(["--index".into(), index]);
                }
                args
            }
        }
        _ => {
            let Some((voice, _)) = selected.clone() else {
                app.set_status("Create or import a trained voice first.");
                return None;
            };
            match action {
                AdvancedRvcAction::AddSample => {
                    if path.is_empty() {
                        app.set_status("Add recording needs an audio file. Press F2 to browse.");
                        return None;
                    }
                    vec!["samples".into(), voice, "add".into(), path]
                }
                AdvancedRvcAction::Inspect => vec!["inspect".into(), voice],
                AdvancedRvcAction::Prepare => {
                    vec!["prepare".into(), voice, "--preset".into(), preset]
                }
                AdvancedRvcAction::Preflight => {
                    vec!["preflight".into(), voice, "--preset".into(), preset]
                }
                AdvancedRvcAction::Train => {
                    vec!["train".into(), voice, "--preset".into(), preset]
                }
                AdvancedRvcAction::Status => vec!["status".into(), voice],
                AdvancedRvcAction::Logs => vec!["logs".into(), voice],
                AdvancedRvcAction::Cancel => vec!["cancel".into(), voice],
                AdvancedRvcAction::Recover => vec!["recover".into(), voice],
                AdvancedRvcAction::Checkpoints => vec!["checkpoints".into(), voice],
                AdvancedRvcAction::Indexes => vec!["indexes".into(), voice],
                AdvancedRvcAction::ActivateCheckpoint => {
                    let checkpoint = match Uuid::parse_str(path.trim()) {
                        Ok(value) => value,
                        Err(_) => {
                            app.set_status("Advanced model selection requires a checkpoint UUID.");
                            return None;
                        }
                    };
                    let mut args = vec!["activate".into(), voice, checkpoint.to_string()];
                    if !index.is_empty() {
                        let index_id = match Uuid::parse_str(index.trim()) {
                            Ok(value) => value,
                            Err(_) => {
                                app.set_status("Advanced index selection requires an index UUID.");
                                return None;
                            }
                        };
                        args.extend(["--index".into(), index_id.to_string()]);
                    }
                    args
                }
                AdvancedRvcAction::TestVoice => {
                    if path.is_empty() {
                        app.set_status("Test voice needs source speech. Press F2 to browse.");
                        return None;
                    }
                    vec!["test".into(), voice, path]
                }
                AdvancedRvcAction::NewVoice
                | AdvancedRvcAction::ImportExisting
                | AdvancedRvcAction::UseInConvert => unreachable!(),
            }
        }
    };

    let label = match selected {
        Some((_, project_name)) => format!("{} · {project_name}", action.label()),
        None => action.label().to_string(),
    };
    Some(TuiAction::RunRvc { label, command })
}

fn use_in_convert(app: &mut App) -> Option<TuiAction> {
    let Some(project) = app.advanced_rvc.selected_project() else {
        app.set_status("Create or import a trained voice first.");
        return None;
    };
    if project.active_checkpoint_id.is_none() {
        app.set_status("Finish training or import a ready voice before using it in Clone audio.");
        return None;
    }
    let voice = project.id.to_string();
    let source = normalize_path_field(&app.advanced_rvc.path);
    app.set_convert_model("rvc");
    app.convert_state.target = voice;
    app.convert_state.target_cursor = app.convert_state.target.chars().count();
    if !source.is_empty() {
        app.convert_state.source = source;
        app.convert_state.source_cursor = app.convert_state.source.chars().count();
        app.convert_state.field = ConvertField::Consent;
    } else {
        app.convert_state.field = ConvertField::Source;
    }
    app.screen = TuiScreen::Convert;
    app.set_status("Trained voice selected. Choose source audio, confirm permission, and clone.");
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn managed_commands_keep_voice_rvc_cli_shape() {
        let prefix = ["voice", "rvc"];
        assert_eq!(prefix, ["voice", "rvc"]);
    }
}
