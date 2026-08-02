use serde_json::{Map, Value};

pub(crate) fn is_model_removal_report(map: &Map<String, Value>) -> bool {
    map.contains_key("model_id")
        && map.contains_key("dry_run")
        && map.contains_key("removed")
        && map.contains_key("reclaimed_bytes")
        && map.contains_key("deleted")
        && map.contains_key("retained")
}

pub(crate) fn format_model_removal(map: &Map<String, Value>) -> String {
    let model_id = text(map, "model_id");
    let dry_run = map.get("dry_run").and_then(Value::as_bool).unwrap_or(false);
    let removed = map.get("removed").and_then(Value::as_bool).unwrap_or(false);
    let reclaimed = map
        .get("reclaimed_bytes")
        .and_then(Value::as_u64)
        .unwrap_or_default();

    let mut lines = Vec::new();
    if dry_run {
        lines.push(format!("Removal preview for {model_id}"));
        lines.push(format!("  estimated reclaim  {}", bytes_label(reclaimed)));
        render_items(&mut lines, "will remove", map.get("deleted"));
        render_items(&mut lines, "will retain", map.get("retained"));
        lines.push("  no files were deleted".to_string());
    } else if removed {
        lines.push(format!("Removed {model_id}"));
        lines.push(format!("  reclaimed          {}", bytes_label(reclaimed)));
        render_items(&mut lines, "deleted", map.get("deleted"));
        render_items(&mut lines, "retained", map.get("retained"));
    } else {
        lines.push(format!("Removal incomplete for {model_id}"));
    }
    lines.join("\n")
}

fn render_items(lines: &mut Vec<String>, heading: &str, value: Option<&Value>) {
    let items = value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    lines.push(format!("  {heading}:"));
    if items.is_empty() {
        lines.push("    none".to_string());
        return;
    }
    for item in items {
        let kind = item.get("kind").and_then(Value::as_str).unwrap_or("item");
        let id = item.get("id").and_then(Value::as_str).unwrap_or("-");
        let path = item.get("path").and_then(Value::as_str).unwrap_or("-");
        let reason = item.get("reason").and_then(Value::as_str).unwrap_or("");
        let size = item
            .get("logical_bytes")
            .and_then(Value::as_u64)
            .map(bytes_label)
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!("    {kind} {id} ({size})"));
        lines.push(format!("      {path}"));
        if !reason.is_empty() {
            lines.push(format!("      {reason}"));
        }
    }
}

fn text(map: &Map<String, Value>, key: &str) -> String {
    map.get(key)
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string()
}

fn bytes_label(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(dry_run: bool, removed: bool) -> Map<String, Value> {
        serde_json::json!({
            "model_id": "qwen2-5-omni",
            "dry_run": dry_run,
            "removed": removed,
            "reclaimed_bytes": 12_032_000_000_u64,
            "deleted": [{
                "kind": "model",
                "id": "qwen2-5-omni",
                "path": "C:/Users/Amaan/.takokit/models/qwen2-5-omni",
                "logical_bytes": 12_032_000_000_u64,
                "reason": "selected model data is exclusively owned by this installation"
            }],
            "retained": [{
                "kind": "runner-runtime",
                "id": "takokit-python-managed",
                "path": "C:/Users/Amaan/.takokit/runners/python-managed",
                "logical_bytes": 1_000_u64,
                "reason": "retained because another installed model requires this runner"
            }]
        })
        .as_object()
        .expect("removal report object")
        .clone()
    }

    #[test]
    fn dry_run_is_rendered_as_a_preview() {
        let output = format_model_removal(&report(true, false));
        assert!(output.contains("Removal preview for qwen2-5-omni"));
        assert!(output.contains("estimated reclaim"));
        assert!(output.contains("will remove:"));
        assert!(output.contains("will retain:"));
        assert!(output.contains("no files were deleted"));
        assert!(!output.contains("Not installed"));
    }

    #[test]
    fn completed_removal_uses_model_id_and_reclaimed_size() {
        let output = format_model_removal(&report(false, true));
        assert!(output.contains("Removed qwen2-5-omni"));
        assert!(output.contains("reclaimed"));
        assert!(output.contains("deleted:"));
        assert!(output.contains("retained:"));
    }
}
