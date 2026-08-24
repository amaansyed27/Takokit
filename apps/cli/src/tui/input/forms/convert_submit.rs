use super::*;

pub(in crate::tui::input) fn submit_convert(app: &mut App) -> Option<TuiAction> {
    let Some(model) = app.selected_convert_model().cloned() else {
        app.set_status(
            "No voice-conversion model is installed. Install a conversion-capable model through the library first.",
        );
        return None;
    };
    if !model.executable {
        return Some(TuiAction::PullModel(model.id));
    }
    let source = normalize_path_field(&app.convert_state.source);
    let target = normalize_path_field(&app.convert_state.target);
    if source.is_empty() {
        app.set_status("Enter, browse, paste, or drag the source-audio path.");
        app.convert_state.field = ConvertField::Source;
        return None;
    }
    if target.is_empty() {
        app.set_status(
            "Enter or browse the target voice reference or RVC package/checkpoint path.",
        );
        app.convert_state.field = ConvertField::Target;
        return None;
    }
    if !app.convert_state.consent {
        app.set_status("Explicit source and target voice consent is required.");
        app.convert_state.field = ConvertField::Consent;
        return None;
    }

    let is_rvc = model.id == "rvc";
    let (f0_method, pitch_shift, index_rate, rms_mix_rate, protect, filter_radius) = if is_rvc {
        (
            app.convert_state.f0_method().to_string(),
            parse_number::<i32>(app, ConvertField::PitchShift, "pitch shift", -24, 24)?,
            parse_float(app, ConvertField::IndexRate, "index rate", 0.0, 1.0)?,
            parse_float(app, ConvertField::RmsMixRate, "RMS mix rate", 0.0, 1.0)?,
            parse_float(app, ConvertField::Protect, "protect", 0.0, 0.5)?,
            parse_number::<u32>(app, ConvertField::FilterRadius, "filter radius", 0, 7)?,
        )
    } else {
        ("rmvpe".to_string(), 0, 0.75, 0.25, 0.33, 3)
    };

    let action = TuiAction::ConvertVoice {
        model: model.id,
        source,
        target,
        f0_method,
        pitch_shift,
        index_rate,
        rms_mix_rate,
        protect,
        filter_radius,
    };
    app.screen = TuiScreen::Activity;
    app.output_scroll = 0;
    Some(action)
}

fn parse_float(
    app: &mut App,
    field: ConvertField,
    label: &str,
    minimum: f32,
    maximum: f32,
) -> Option<f32> {
    let source = match field {
        ConvertField::IndexRate => app.convert_state.index_rate.clone(),
        ConvertField::RmsMixRate => app.convert_state.rms_mix_rate.clone(),
        ConvertField::Protect => app.convert_state.protect.clone(),
        _ => return None,
    };
    match source.trim().parse::<f32>() {
        Ok(value) if value.is_finite() && (minimum..=maximum).contains(&value) => Some(value),
        _ => {
            app.set_status(format!("{label} must be between {minimum} and {maximum}."));
            app.convert_state.field = field;
            None
        }
    }
}

fn parse_number<T>(
    app: &mut App,
    field: ConvertField,
    label: &str,
    minimum: T,
    maximum: T,
) -> Option<T>
where
    T: std::str::FromStr + PartialOrd + Copy + std::fmt::Display,
{
    let source = match field {
        ConvertField::PitchShift => app.convert_state.pitch_shift.clone(),
        ConvertField::FilterRadius => app.convert_state.filter_radius.clone(),
        _ => return None,
    };
    match source.trim().parse::<T>() {
        Ok(value) if value >= minimum && value <= maximum => Some(value),
        _ => {
            app.set_status(format!("{label} must be between {minimum} and {maximum}."));
            app.convert_state.field = field;
            None
        }
    }
}
