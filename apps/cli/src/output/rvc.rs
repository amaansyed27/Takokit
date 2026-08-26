use serde_json::{Map, Value};

pub(super) fn is_training_job(map: &Map<String, Value>) -> bool {
    map.contains_key("voice_id")
        && map.contains_key("config")
        && map.contains_key("status")
        && map.contains_key("stage")
        && map.contains_key("checkpoint_ids")
}

pub(super) fn format_training_job(map: &Map<String, Value>) -> String {
    let mut lines = vec!["RVC training".to_string()];
    lines.push(format!("  {:<20} {}", "status", text(map, "status")));
    lines.push(format!("  {:<20} {}", "stage", text(map, "stage")));

    if let Some(config) = map.get("config").and_then(Value::as_object) {
        let total_epochs = config.get("epochs").and_then(Value::as_u64);
        let current_epoch = map.get("current_epoch").and_then(Value::as_u64);
        match (current_epoch, total_epochs) {
            (Some(current), Some(total)) => {
                lines.push(format!("  {:<20} {current} / {total}", "epoch"));
            }
            (None, Some(total)) => {
                lines.push(format!("  {:<20} {total}", "total epochs"));
            }
            _ => {}
        }
        for (key, label) in [
            ("preset", "quality preset"),
            ("batch_size", "batch size"),
            ("device", "device"),
            ("precision", "precision"),
        ] {
            if let Some(value) = config.get(key).filter(|value| !value.is_null()) {
                lines.push(format!("  {label:<20} {}", scalar(value)));
            }
        }
    }

    if let Some(failure) = map.get("failure").and_then(Value::as_object) {
        if let Some(message) = failure.get("message").and_then(Value::as_str) {
            lines.push(format!("  {:<20} {message}", "failure"));
        }
    }
    lines.join("\n")
}

pub(super) fn is_voice_conversion_report(map: &Map<String, Value>) -> bool {
    map.contains_key("execution_status")
        && map.contains_key("quality_status")
        && map.contains_key("effective_settings")
        && map.contains_key("checkpoint")
}

pub(super) fn format_voice_conversion(map: &Map<String, Value>) -> String {
    let mut lines = Vec::new();
    let model = text(map, "model");
    let is_rvc = model.eq_ignore_ascii_case("rvc");

    lines.push("Voice conversion complete".to_string());
    lines.push(format!(
        "  {:<20} {}",
        "execution",
        text(map, "execution_status")
    ));
    lines.push(format!(
        "  {:<20} {}",
        "perceptual quality",
        text(map, "quality_status")
    ));
    lines.push(format!(
        "  {:<20} {}",
        "listening review",
        if map
            .get("quality_review_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "required"
        } else {
            "not required"
        }
    ));
    field(&mut lines, map, "model", "model");
    field(&mut lines, map, "output_path", "output");
    if let Some(bytes) = map.get("bytes").and_then(Value::as_u64) {
        lines.push(format!("  {:<20} {} bytes", "size", bytes));
    }

    if is_rvc {
        if let Some(settings) = map.get("effective_settings").and_then(Value::as_object) {
            lines.push(String::new());
            lines.push("Effective RVC settings".to_string());
            for (key, label) in [
                ("f0_method", "F0 method"),
                ("pitch_shift", "pitch shift"),
                ("index_rate", "index rate"),
                ("rms_mix_rate", "RMS mix rate"),
                ("protect", "protect"),
                ("filter_radius", "filter radius"),
            ] {
                if let Some(value) = settings.get(key) {
                    lines.push(format!("  {label:<20} {}", scalar(value)));
                }
            }
        }
    }

    if let Some(checkpoint) = map.get("checkpoint").and_then(Value::as_object) {
        lines.push(String::new());
        if is_rvc {
            lines.push("Target package evidence".to_string());
            for (key, label) in [
                ("checkpoint_path", "checkpoint"),
                ("checkpoint_sha256", "checkpoint SHA-256"),
                ("index_path", "index"),
                ("index_sha256", "index SHA-256"),
                ("pairing_status", "pairing"),
                ("target_reference_path", "target reference"),
            ] {
                if let Some(value) = checkpoint.get(key).filter(|value| !value.is_null()) {
                    lines.push(format!("  {label:<20} {}", scalar(value)));
                }
            }
            lines.push(format!(
                "  {:<20} {}",
                "quality baseline",
                if checkpoint
                    .get("quality_baseline_ready")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "package is ready for human comparison"
                } else {
                    "not established; execution cannot be promoted to a quality pass"
                }
            ));
        } else {
            lines.push("Target reference".to_string());
            if let Some(value) = checkpoint
                .get("target_reference_path")
                .filter(|value| !value.is_null())
            {
                lines.push(format!("  {:<20} {}", "reference audio", scalar(value)));
            }
            lines.push(format!(
                "  {:<20} {}",
                "quality baseline",
                if checkpoint
                    .get("quality_baseline_ready")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "reference is ready for human comparison"
                } else {
                    "not established; execution cannot be promoted to a quality pass"
                }
            ));
        }
    }

    if let Some(notice) = map.get("quality_notice").and_then(Value::as_str) {
        lines.push(String::new());
        lines.push(format!("Review: {notice}"));
    }
    lines.push("Quality checklist: words unchanged and intelligible; timbre materially changed; target similarity present; no severe robotic, metallic, tearing, octave-jump or dropout artefacts.".to_string());
    lines.join("\n")
}

fn field(lines: &mut Vec<String>, map: &Map<String, Value>, key: &str, label: &str) {
    if let Some(value) = map.get(key).filter(|value| !value.is_null()) {
        lines.push(format!("  {label:<20} {}", scalar(value)));
    }
}

fn text(map: &Map<String, Value>, key: &str) -> String {
    map.get(key)
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .replace('_', " ")
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_job_renders_real_epoch_over_configured_total() {
        let value = serde_json::json!({
            "id": "job",
            "voice_id": "voice",
            "status": "running",
            "stage": "train",
            "current_epoch": 183,
            "checkpoint_ids": [],
            "config": {
                "preset": "balanced",
                "epochs": 200,
                "batch_size": 8,
                "device": "auto",
                "precision": "auto"
            }
        });
        let map = value.as_object().expect("training job object");
        assert!(is_training_job(map));
        let rendered = format_training_job(map);
        assert!(rendered.contains("epoch                183 / 200"));
        assert!(rendered.contains("status               running"));
        assert!(rendered.contains("stage                train"));
    }

    #[test]
    fn execution_pass_does_not_render_as_quality_pass() {
        let value = serde_json::json!({
            "model": "rvc",
            "output_path": "conversion.wav",
            "bytes": 1024,
            "execution_status": "passed",
            "quality_status": "not_evaluated",
            "quality_review_required": true,
            "quality_notice": "Listen before promotion.",
            "effective_settings": {
                "f0_method": "rmvpe",
                "pitch_shift": 0,
                "index_rate": 0.75,
                "rms_mix_rate": 0.25,
                "protect": 0.33,
                "filter_radius": 3
            },
            "checkpoint": {
                "checkpoint_path": "voice.pth",
                "checkpoint_sha256": "abc",
                "pairing_status": "single_index_unverified",
                "quality_baseline_ready": false
            }
        });
        let map = value.as_object().expect("conversion object");
        let rendered = format_voice_conversion(map);
        assert!(rendered.contains("execution            passed"));
        assert!(rendered.contains("perceptual quality   not evaluated"));
        assert!(rendered.contains("Effective RVC settings"));
        assert!(rendered.contains("cannot be promoted to a quality pass"));
    }

    #[test]
    fn openvoice_uses_reference_audio_language() {
        let value = serde_json::json!({
            "model": "openvoice",
            "output_path": "conversion.wav",
            "bytes": 2048,
            "execution_status": "passed",
            "quality_status": "not_evaluated",
            "quality_review_required": true,
            "effective_settings": {
                "f0_method": "rmvpe",
                "pitch_shift": 0
            },
            "checkpoint": {
                "checkpoint_path": "owned-reference.wav",
                "checkpoint_sha256": "not-applicable",
                "pairing_status": "not_applicable",
                "target_reference_path": "owned-reference.wav",
                "quality_baseline_ready": true
            }
        });
        let map = value.as_object().expect("conversion object");
        let rendered = format_voice_conversion(map);
        assert!(rendered.contains("Target reference"));
        assert!(rendered.contains("reference audio      owned-reference.wav"));
        assert!(!rendered.contains("Effective RVC settings"));
        assert!(!rendered.contains("checkpoint SHA-256"));
    }
}
