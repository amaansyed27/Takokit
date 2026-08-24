use super::*;

pub(in crate::tui::input) fn submit_clone(app: &mut App) -> Option<TuiAction> {
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
    let sample = normalize_path_field(&app.clone_state.sample);
    if name.is_empty() {
        app.set_status("Enter a voice name before saving the cloned voice.");
        app.clone_state.field = CloneField::Name;
        return None;
    }
    if sample.is_empty() {
        app.set_status("Enter, browse, paste, or drag a local reference-audio path.");
        app.clone_state.field = CloneField::Sample;
        return None;
    }
    if !app.clone_state.consent {
        app.set_status("Explicit voice-owner consent is required.");
        app.clone_state.field = CloneField::Consent;
        return None;
    }
    let action = TuiAction::CloneVoice {
        model: model.id,
        name,
        sample,
    };
    app.screen = TuiScreen::Activity;
    app.output_scroll = 0;
    Some(action)
}
