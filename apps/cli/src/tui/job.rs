use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
    thread,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    Starting,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug)]
pub struct CommandResult {
    pub label: String,
    pub output: String,
    pub state: OperationState,
}

impl CommandResult {
    pub fn success(&self) -> bool {
        self.state == OperationState::Succeeded
    }
}

pub struct CommandJob {
    pub label: String,
    receiver: Receiver<CommandResult>,
    pid: Arc<AtomicU32>,
    cancellation_requested: Arc<AtomicBool>,
}

impl CommandJob {
    pub fn start(label: impl Into<String>, args: Vec<String>) -> Self {
        let label = label.into();
        let worker_label = label.clone();
        let (sender, receiver) = mpsc::channel();
        let pid = Arc::new(AtomicU32::new(0));
        let worker_pid = pid.clone();
        let cancellation_requested = Arc::new(AtomicBool::new(false));
        let worker_cancellation = cancellation_requested.clone();
        thread::spawn(move || {
            let result = execute_cli(&args, worker_label, worker_pid, worker_cancellation);
            let _ = sender.send(result);
        });
        Self {
            label,
            receiver,
            pid,
            cancellation_requested,
        }
    }

    pub fn poll(&self) -> Option<CommandResult> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(CommandResult {
                label: self.label.clone(),
                output: "The background task stopped before returning a result.".to_string(),
                state: OperationState::Failed,
            }),
        }
    }

    pub fn cancel(&self) {
        self.cancellation_requested.store(true, Ordering::SeqCst);
        let pid = self.pid.load(Ordering::SeqCst);
        if pid != 0 {
            terminate_process_tree(pid);
        }
    }
}

fn execute_cli(
    args: &[String],
    label: String,
    pid: Arc<AtomicU32>,
    cancellation_requested: Arc<AtomicBool>,
) -> CommandResult {
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            return CommandResult {
                label,
                output: format!("Takokit could not locate its executable: {error}"),
                state: OperationState::Failed,
            }
        }
    };
    let normalized_args = args
        .iter()
        .map(|arg| normalize_terminal_argument(arg))
        .collect::<Vec<_>>();
    let workspace_dir = normalized_args
        .windows(2)
        .find(|pair| pair[0] == "--workspace")
        .map(|pair| PathBuf::from(&pair[1]))
        .filter(|path| path.is_dir());

    let mut command = Command::new(executable);
    command
        .args(&normalized_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(workspace_dir) = workspace_dir {
        command.current_dir(workspace_dir);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return CommandResult {
                label,
                output: format!("Takokit could not start the task: {error}"),
                state: OperationState::Failed,
            }
        }
    };
    pid.store(child.id(), Ordering::SeqCst);
    if cancellation_requested.load(Ordering::SeqCst) {
        let _ = child.kill();
    }
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            return CommandResult {
                label,
                output: format!("Takokit could not wait for the task: {error}"),
                state: OperationState::Failed,
            }
        }
    };
    pid.store(0, Ordering::SeqCst);
    let stdout_raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stdout = render_tui_stdout(&stdout_raw);
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let rendered = combine_output(&stdout, &stderr);
    if cancellation_requested.load(Ordering::SeqCst) {
        return CommandResult {
            label: label.clone(),
            output: if rendered.is_empty() {
                format!("{label} was cancelled.")
            } else {
                format!("{label} was cancelled.\n\n{rendered}")
            },
            state: OperationState::Cancelled,
        };
    }
    if output.status.success() {
        CommandResult {
            label: label.clone(),
            output: if rendered.is_empty() {
                format!("{label} completed.")
            } else {
                rendered
            },
            state: OperationState::Succeeded,
        }
    } else {
        CommandResult {
            label: label.clone(),
            output: format!(
                "{label} failed.{}",
                if rendered.is_empty() {
                    String::new()
                } else {
                    format!("\n\n{rendered}")
                }
            ),
            state: OperationState::Failed,
        }
    }
}

fn render_tui_stdout(stdout: &str) -> String {
    if stdout.is_empty() {
        return String::new();
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout) else {
        return stdout.to_string();
    };
    let Some(map) = value.as_object() else {
        return stdout.to_string();
    };

    if map.contains_key("consent_affirmed")
        && map.contains_key("sample_path")
        && map.contains_key("model_id")
    {
        return render_voice_profile(map);
    }
    if map.contains_key("execution_status")
        && map.contains_key("target_voice")
        && map.contains_key("output_path")
    {
        return render_voice_conversion(map);
    }
    if map.contains_key("output_path") && map.contains_key("engine") {
        return render_audio_output(map);
    }
    if map.contains_key("text") && map.contains_key("model") {
        return render_transcription(map);
    }
    stdout.to_string()
}

fn render_voice_profile(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let name = value_text(map, "name", "saved voice");
    let id = value_text(map, "id", "unknown");
    let model = value_text(map, "model_id", "unknown");
    let sample = value_text(map, "sample_path", "unknown");
    format!(
        "Cloned voice saved\n\n  Name       {name}\n  Voice ID   {id}\n  Model      {model}\n  Reference  {sample}\n\nNext: open Speak, select {model}, move to Voice, and use ↑/↓ to choose {id}."
    )
}

fn render_voice_conversion(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let model = value_text(map, "model", "unknown");
    let output = value_text(map, "output_path", "unknown");
    let execution = value_text(map, "execution_status", "unknown").replace('_', " ");
    let quality = value_text(map, "quality_status", "not evaluated").replace('_', " ");
    let bytes = map
        .get("bytes")
        .and_then(serde_json::Value::as_u64)
        .map(bytes_label)
        .unwrap_or_else(|| "unknown".to_string());
    let target = map
        .get("checkpoint")
        .and_then(serde_json::Value::as_object)
        .and_then(|checkpoint| checkpoint.get("target_reference_path"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| map.get("target_voice").and_then(serde_json::Value::as_str))
        .unwrap_or("unknown");
    let target_label = if model == "rvc" {
        "Target package"
    } else {
        "Target reference"
    };
    let mut result = format!(
        "Voice conversion complete\n\n  Model             {model}\n  Execution         {execution}\n  Listening quality {quality}\n  Output            {output}\n  Size              {bytes}\n  {target_label:<17} {target}\n"
    );
    if model == "rvc" {
        if let Some(settings) = map
            .get("effective_settings")
            .and_then(serde_json::Value::as_object)
        {
            result.push_str("\nRVC settings");
            for (key, label) in [
                ("f0_method", "F0"),
                ("pitch_shift", "Pitch"),
                ("index_rate", "Index"),
                ("rms_mix_rate", "RMS mix"),
                ("protect", "Protect"),
                ("filter_radius", "Filter"),
            ] {
                if let Some(value) = settings.get(key) {
                    result.push_str(&format!("\n  {label:<10} {}", scalar(value)));
                }
            }
            result.push('\n');
        }
    }
    result.push_str(
        "\nReview by listening: the words should remain unchanged, while the voice should move toward the target. Press P to play the result or O to open the output folder.",
    );
    result
}

fn render_audio_output(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let model = value_text(map, "model", "unknown");
    let engine = value_text(map, "engine", "unknown");
    let output = value_text(map, "output_path", "unknown");
    let bytes = map
        .get("bytes")
        .and_then(serde_json::Value::as_u64)
        .map(bytes_label)
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "Speech ready\n\n  Model   {model}\n  Engine  {engine}\n  Size    {bytes}\n  Output  {output}\n\nPress P to play the newest audio output or O to open the output folder."
    )
}

fn render_transcription(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let model = value_text(map, "model", "unknown");
    let text = value_text(map, "text", "");
    format!("Transcription complete\n\n  Model  {model}\n\n{text}")
}

fn value_text(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: &str,
) -> String {
    map.get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn bytes_label(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn normalize_terminal_argument(argument: &str) -> String {
    let value = argument.trim();
    if value.len() >= 2 {
        let quoted = (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''));
        if quoted {
            let inner = &value[1..value.len() - 1];
            if looks_like_path(inner) {
                return inner.to_string();
            }
        }
    }
    argument.to_string()
}

fn looks_like_path(value: &str) -> bool {
    let path = Path::new(value);
    path.is_absolute()
        || value.starts_with("./")
        || value.starts_with(".\\")
        || value.starts_with("../")
        || value.starts_with("..\\")
        || value.starts_with('~')
}

#[cfg(windows)]
fn terminate_process_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(windows))]
fn terminate_process_tree(pid: u32) {
    let status = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if !status.is_ok_and(|status| status.success()) {
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn combine_output(stdout: &str, stderr: &str) -> String {
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout.to_string(),
        (true, false) => stderr.to_string(),
        (false, false) => format!("{stdout}\n\n{stderr}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_cli_streams_without_losing_completion_timing() {
        assert_eq!(
            combine_output("result", "Completed in 1.2s"),
            "result\n\nCompleted in 1.2s"
        );
        assert_eq!(combine_output("", "failure"), "failure");
    }

    #[test]
    fn operation_states_distinguish_success_failure_and_cancellation() {
        assert_ne!(OperationState::Succeeded, OperationState::Failed);
        assert_ne!(OperationState::Failed, OperationState::Cancelled);
        assert_eq!(OperationState::Starting, OperationState::Starting);
        assert_eq!(OperationState::Running, OperationState::Running);
    }

    #[test]
    fn terminal_dragged_paths_drop_shell_quotes_without_touching_normal_text() {
        let absolute = if cfg!(windows) {
            r#""C:\Voice Projects\sample.wav""#
        } else {
            r#""/tmp/Voice Projects/sample.wav""#
        };
        let expected = if cfg!(windows) {
            r#"C:\Voice Projects\sample.wav"#
        } else {
            "/tmp/Voice Projects/sample.wav"
        };
        assert_eq!(normalize_terminal_argument(absolute), expected);
        assert_eq!(
            normalize_terminal_argument(r#""say this exactly""#),
            r#""say this exactly""#
        );
    }

    #[test]
    fn voice_profile_result_explains_the_next_speak_step() {
        let rendered = render_tui_stdout(
            r#"{"id":"my-voice","name":"My Voice","model_id":"openvoice","sample_path":"voice.wav","consent_affirmed":true}"#,
        );
        assert!(rendered.contains("Cloned voice saved"));
        assert!(rendered.contains("use ↑/↓ to choose my-voice"));
        assert!(!rendered.contains("consent_affirmed"));
    }

    #[test]
    fn openvoice_conversion_result_uses_reference_language_not_rvc_internals() {
        let rendered = render_tui_stdout(
            r#"{"model":"openvoice","output_path":"out.wav","bytes":1024,"execution_status":"passed","quality_status":"not_evaluated","target_voice":"reference.wav","checkpoint":{"target_reference_path":"reference.wav"},"effective_settings":{"f0_method":"rmvpe"}}"#,
        );
        assert!(rendered.contains("Target reference"));
        assert!(rendered.contains("words should remain unchanged"));
        assert!(!rendered.contains("RVC settings"));
        assert!(!rendered.contains("checkpoint"));
    }
}
