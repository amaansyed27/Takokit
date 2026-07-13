mod app;
mod command;
mod ui;

use std::process::Command;

use app::{App, TuiAction};
use takokit_core::RuntimeConfig;
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

pub async fn run_launcher(
    config: &RuntimeConfig,
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let mut status = format!(
        "Ready. All palette commands use the same CLI and daemon backend. Storage: {}",
        store.root().display()
    );

    loop {
        let mut state = App::new(config, store, package_registry, installed_registry, status)?;
        let mut selected_action = None;
        ratatui::run(|mut terminal| -> std::io::Result<()> {
            selected_action = Some(app::run(&mut terminal, &mut state)?);
            Ok(())
        })?;

        match selected_action.unwrap_or(TuiAction::Quit) {
            TuiAction::Quit => return Ok(()),
            TuiAction::Refresh => {
                status = "Catalog, installed state and daemon-backed data refreshed.".to_string()
            }
            TuiAction::RunCli(args) => {
                status = execute_cli(&args).unwrap_or_else(|error| format!("Error: {error:#}"));
            }
        }
    }
}

fn execute_cli(args: &[String]) -> anyhow::Result<String> {
    let executable = std::env::current_exe()?;
    let output = Command::new(&executable).args(args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let rendered = combine_output(&stdout, &stderr);

    if output.status.success() {
        if rendered.is_empty() {
            Ok(format!("Completed: takokit {}", display_args(args)))
        } else {
            Ok(rendered)
        }
    } else {
        anyhow::bail!(
            "takokit {} exited with {}{}",
            display_args(args),
            output.status,
            if rendered.is_empty() {
                String::new()
            } else {
                format!("\n\n{rendered}")
            }
        )
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

fn display_args(args: &[String]) -> String {
    args.iter()
        .map(|argument| {
            if argument.chars().any(char::is_whitespace) {
                format!("\"{}\"", argument.replace('"', "\\\""))
            } else {
                argument.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_cli_streams_without_losing_completion_timing() {
        assert_eq!(
            combine_output("json", "Completed in 1.2s"),
            "json\n\nCompleted in 1.2s"
        );
        assert_eq!(combine_output("", "failure"), "failure");
    }

    #[test]
    fn displays_arguments_with_spaces_as_quoted_values() {
        assert_eq!(
            display_args(&["speak".into(), "Hello world".into()]),
            "speak \"Hello world\""
        );
    }
}
