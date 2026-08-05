use std::{
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
            let result = execute_cli(
                &args,
                worker_label,
                worker_pid,
                worker_cancellation,
            );
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
    let mut child = match Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
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
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
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
}
