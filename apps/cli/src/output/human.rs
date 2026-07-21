use super::yes_no;

pub(crate) fn print_serializable<T: serde::Serialize + ?Sized>(value: &T) -> anyhow::Result<()> {
    print_value(&serde_json::to_value(value)?)
}

pub(crate) fn print_value(value: &serde_json::Value) -> anyhow::Result<()> {
    if json_requested() {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        render_value(value, 0);
    }
    Ok(())
}

fn json_requested() -> bool {
    std::env::var("TAKOKIT_OUTPUT")
        .map(|value| value.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn render_value(value: &serde_json::Value, depth: usize) {
    if let Some(data) = value.get("data") {
        render_value(data, depth);
        return;
    }
    match value {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                println!("No entries.");
            } else {
                for item in items {
                    render_row(item);
                }
            }
        }
        serde_json::Value::Object(map) => {
            if map.contains_key("model_id") && map.contains_key("artifacts") {
                render_pull(map);
                return;
            }
            if map.contains_key("output_path") {
                render_output(map);
                return;
            }
            if map.contains_key("instance_id") && map.contains_key("pid") {
                println!("Daemon running");
                render_field("pid", map.get("pid"));
                println!(
                    "  {:<12} {}:{}",
                    "address",
                    text(map, "host"),
                    scalar(map.get("port"))
                );
                render_field("executable", map.get("executable"));
                render_field("storage", map.get("storage_root"));
                return;
            }
            if let Some(removed) = map.get("removed").and_then(|value| value.as_bool()) {
                println!(
                    "{} {}",
                    if removed { "Removed" } else { "Not installed:" },
                    text(map, "id")
                );
                return;
            }
            if let Some(summary) = map.get("summary") {
                render_row(summary);
                return;
            }
            for (key, item) in map {
                if item.is_null() {
                    continue;
                }
                match item {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        println!("{}{}:", "  ".repeat(depth), label(key));
                        render_value(item, depth + 1);
                    }
                    _ => println!(
                        "{}{:<14} {}",
                        "  ".repeat(depth),
                        label(key),
                        scalar(Some(item))
                    ),
                }
            }
        }
        _ => println!("{}", scalar(Some(value))),
    }
}

fn render_pull(map: &serde_json::Map<String, serde_json::Value>) {
    println!("Model {}", text(map, "model_id"));
    render_stage(map, "artifacts", "artifacts");
    render_stage(map, "runner_contract", "runner");
    render_stage(map, "runner_runtime", "runtime");
    render_stage(map, "adapter", "adapter");
    println!(
        "  {:<12} {}",
        "ready",
        yes_no(
            map.get("executable")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        )
    );
    if let Some(path) = map.get("logs_path").and_then(|value| value.as_str()) {
        println!("  {:<12} {path}", "logs");
    }
}

fn render_output(map: &serde_json::Map<String, serde_json::Value>) {
    let output = text(map, "output_path");
    if let Some(body) = map.get("text").and_then(|value| value.as_str()) {
        println!("Transcription complete");
        if !text(map, "model").is_empty() {
            println!("  {:<12} {}", "model", text(map, "model"));
        }
        println!("\n{body}");
        if !output.is_empty() {
            println!("\nSaved to {output}");
        }
    } else {
        println!("Audio ready");
        if !text(map, "model").is_empty() {
            println!("  {:<12} {}", "model", text(map, "model"));
        }
        if !text(map, "engine").is_empty() {
            println!("  {:<12} {}", "engine", text(map, "engine"));
        }
        if let Some(bytes) = map.get("bytes").and_then(|value| value.as_u64()) {
            println!("  {:<12} {}", "size", bytes_label(bytes));
        }
        if !output.is_empty() {
            println!("  {:<12} {output}", "output");
        }
    }
}

fn render_row(value: &serde_json::Value) {
    let Some(map) = value.as_object() else {
        println!("{}", scalar(Some(value)));
        return;
    };
    let primary = ["name", "title", "id", "model_id"]
        .iter()
        .find_map(|key| map.get(*key).and_then(|value| value.as_str()))
        .unwrap_or("entry");
    print!("{primary}");
    if let Some(id) = map
        .get("id")
        .and_then(|value| value.as_str())
        .filter(|id| *id != primary)
    {
        print!("  {id}");
    }
    if let Some(state) = ["status", "state", "lifecycle_state", "runtime_state"]
        .iter()
        .find_map(|key| map.get(*key).and_then(|value| value.as_str()))
    {
        print!("  [{state}]");
    }
    if let Some(model) = map.get("last_model").and_then(|value| value.as_str()) {
        print!("  model={model}");
    }
    if let Some(count) = map.get("event_count").and_then(|value| value.as_u64()) {
        print!("  events={count}");
    }
    println!();
}

fn render_stage(map: &serde_json::Map<String, serde_json::Value>, key: &str, name: &str) {
    let Some(item) = map.get(key) else {
        return;
    };
    if item.is_null() {
        return;
    }
    let state = item
        .get("state")
        .and_then(|value| value.as_str())
        .unwrap_or("ready");
    println!("  {name:<12} {state}");
    if let Some(detail) = item
        .get("detail")
        .and_then(|value| value.as_str())
        .filter(|detail| *detail != state)
    {
        println!("               {detail}");
    }
}

fn render_field(name: &str, value: Option<&serde_json::Value>) {
    println!("  {name:<12} {}", scalar(value));
}

fn text(map: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    map.get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

fn scalar(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(value)) => value.clone(),
        Some(serde_json::Value::Bool(value)) => value.to_string(),
        Some(serde_json::Value::Number(value)) => value.to_string(),
        Some(serde_json::Value::Null) | None => "-".to_string(),
        Some(value) => value.to_string(),
    }
}

fn label(value: &str) -> String {
    value.replace('_', " ")
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
