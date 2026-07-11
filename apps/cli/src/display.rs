//! Human-readable CLI output, kept separate from command execution.

use super::{capability_labels, yes_no};
use takokit_core::ModelInfo;
use takokit_package::InstalledModelRecord;

pub(crate) fn format_model_show(
    info: &ModelInfo,
    installed_record: Option<&InstalledModelRecord>,
) -> String {
    let mut lines = vec![
        format!("{} ({})", info.name, info.id),
        format!("family: {}", info.family),
        format!("version: {}", info.version),
        format!("backend: {}", info.backend),
        format!("runner: {}", info.runner),
        format!("installed: {}", info.installed),
        format!("runner installed: {}", info.runner_installed),
        format!("runner runtime: {}", info.runner_runtime_state),
        format!("lifecycle: {}", info.lifecycle_state),
        format!("status: {}", info.execution_status),
        format!("executable today: {}", yes_no(info.executable)),
        format!("license: {}", info.license),
    ];
    if let Some(warning) = &info.license_warning {
        lines.push(format!("license warning: {warning}"));
    }
    lines.push(format!(
        "capabilities: {}",
        capability_labels(&info.capabilities)
    ));
    lines.push(format!("hardware: {}", info.hardware_notes));
    lines.push(format!("artifacts: {}", info.artifact_count));
    if let Some(record) = installed_record {
        lines.extend([
            format!("installed status: {:?}", record.status),
            format!("installed at: {}", record.installed_at),
            format!("source: {}", record.source),
            format!("installed artifacts: {}", record.artifacts.len()),
        ]);
    } else {
        lines.push("installed status: not installed".to_string());
    }
    lines.push(if info.missing.is_empty() {
        "missing: none".to_string()
    } else {
        format!("missing: {}", info.missing.join("; "))
    });
    lines.push(format!("next command: {}", info.next_command));
    lines.push(format!("description: {}", info.summary));
    lines.join("\n")
}
