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
        "Voice saved\nName: {name}\nID: {id}\nModel: {model}\nReference: {sample}\n\nNext: open Speak, choose {model}, then select {name}."
    )
}

fn render_voice_conversion(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let output = value_text(map, "output_path", "unknown");
    let engine = value_text(map, "engine", "unknown");
    let target = value_text(map, "target_voice", "unknown");
    let status = value_text(map, "execution_status", "unknown");
    format!(
        "Voice conversion complete\nOutput: {output}\nTarget: {target}\nEngine: {engine}\nStatus: {status}\n\nNext: open Activity or Files to use the output."
    )
}

fn render_audio_output(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let output = value_text(map, "output_path", "unknown");
    let engine = value_text(map, "engine", "unknown");
    format!("Audio generated\nOutput: {output}\nEngine: {engine}\n\nNext: open Activity or Files.")
}

fn render_transcription(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let text = value_text(map, "text", "");
    let model = value_text(map, "model", "unknown");
    format!("Transcription complete\nModel: {model}\n\n{text}")
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

fn combine_output(stdout: &str, stderr: &str) -> String {
    match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n\n{stderr}"),
        (false, true) => stdout.to_string(),
        (true, false) => stderr.to_string(),
        (true, true) => String::new(),
    }
}

fn normalize_terminal_argument(arg: &str) -> String {
    let normalized = arg.trim();
    if normalized.len() >= 2 {
        let bytes = normalized.as_bytes();
        let quoted = matches!(
            (bytes.first(), bytes.last()),
            (Some(b'"'), Some(b'"')) | (Some(b'\''), Some(b'\''))
        );
        if quoted {
            return normalized[1..normalized.len() - 1].to_string();
        }
    }
    normalized.to_string()
}

fn terminate_process_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_quoted_terminal_arguments() {
        assert_eq!(
            normalize_terminal_argument(r#""C:\Voice Project ü\sample.wav""#),
            r#"C:\Voice Project ü\sample.wav"#
        );
        assert_eq!(normalize_terminal_argument("'hello world'"), "hello world");
    }

    #[test]
    fn combines_streams_without_losing_stderr() {
        assert_eq!(combine_output("out", "err"), "out\n\nerr");
        assert_eq!(combine_output("", "err"), "err");
    }
}
