use super::*;

#[test]
fn nested_screens_have_obvious_parents() {
    assert_eq!(TuiScreen::Models.parent(), TuiScreen::Manage);
    assert_eq!(TuiScreen::ModelLibrary.parent(), TuiScreen::Manage);
    assert_eq!(TuiScreen::Convert.parent(), TuiScreen::Home);
    assert_eq!(TuiScreen::Workspace.parent(), TuiScreen::Home);
    assert_eq!(TuiScreen::Home.parent(), TuiScreen::Home);
}

#[test]
fn manage_keeps_installed_and_library_as_separate_screens() {
    assert_eq!(MANAGE_ACTIONS[0].0, "Installed models");
    assert_eq!(MANAGE_ACTIONS[1].0, "Model library");
    assert_eq!(MANAGE_ACTIONS[2].0, "Runners");
    assert_eq!(MANAGE_ACTIONS[3].0, "System");
}

#[test]
fn home_starts_with_primary_tasks_and_has_workspace_access() {
    assert_eq!(HOME_ACTIONS[0].0, "Speak");
    assert_eq!(HOME_ACTIONS[1].0, "Transcribe");
    assert_eq!(HOME_ACTIONS[2].0, "Create voice");
    assert_eq!(HOME_ACTIONS[3].0, "Convert voice");
    assert!(HOME_ACTIONS.iter().any(|item| item.0 == "Workspace"));
}

#[test]
fn session_position_handles_an_uninitialized_workspace() {
    assert_eq!(session_position(&[], None), 0);
}

#[test]
fn workspace_field_navigation_is_bounded() {
    assert_eq!(WorkspaceField::Path.next(), WorkspaceField::Apply);
    assert_eq!(WorkspaceField::Apply.next(), WorkspaceField::Path);
}
