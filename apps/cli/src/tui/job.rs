use std::{
    process::Command,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

#[derive(Debug)]
pub struct CommandResult {
    pub label: String,
    pub output: String,
    pub success: bool,
}

pub struct CommandJob {
    pub label: String,
    receiver: Receiver<CommandResult>,
}

impl CommandJob {
    pub fn start(label: impl Into<String>, args: Vec<String>) -> Self {
        let label = label.into();
        let worker_label = label.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = execute_cli(&args, worker_label);
            let _ = sender.send(result);
        });
        Self { label, receiver }
    }

    pub fn poll(&self) -> Option<CommandResult> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(CommandResult {
                label: self.label.clone(),
                output: "The background task stopped before returning a result.".to_string(),
                success: false,
            }),
        }
    }
}

fn execute_cli(args: &[String], label: String) -> CommandResult {
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            return CommandResult {
                label,
                output: format!("Takokit could not locate its executable: {error}"),
                success: false,
            }
        }
    };
    let output = match Command::new(executable).args(args).output() {
        Ok(output) => output,
        Err(error) => {
            return CommandResult {
                label,
                output: format!("Takokit could not start the task: {error}"),
                success: false,
            }
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let rendered = combine_output(&stdout, &stderr);
    if output.status.success() {
        CommandResult {
            label: label.clone(),
            output: if rendered.is_empty() {
                format!("{label} completed.")
            } else {
                rendered
            },
            success: true,
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
            success: false,
        }
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
}
