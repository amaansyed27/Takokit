mod app;
mod catalog;
mod clone;
mod editor;
mod input;
mod job;
mod ui;

use std::{io, time::Duration};

use app::{App, TuiAction};
use catalog::SystemAction;
use crossterm::event::{self, Event, KeyEventKind};
use job::CommandJob;
use takokit_core::RuntimeConfig;
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

use crate::workspace::CliWorkspace;

pub async fn run_launcher(
    config: &RuntimeConfig,
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    workspace: &CliWorkspace,
) -> anyhow::Result<()> {
    let mut state = App::new(
        config,
        store,
        package_registry,
        installed_registry,
        workspace,
    )?;
    let mut active_job: Option<CommandJob> = None;

    ratatui::run(|terminal| -> io::Result<()> {
        loop {
            state.tick = state.tick.wrapping_add(1);
            if let Some(result) = active_job.as_ref().and_then(CommandJob::poll) {
                state.running_label = None;
                state.last_label = Some(result.label.clone());
                state.set_status(if result.success {
                    result.output
                } else {
                    format!("Task failed.\n\n{}", result.output)
                });
                if let Err(error) =
                    state.reload(config, store, package_registry, installed_registry)
                {
                    state.status.push_str(&format!(
                        "\n\nThe task finished, but refreshing local state failed: {error:#}"
                    ));
                }
                active_job = None;
            }

            terminal.draw(|frame| ui::render(frame, &state))?;
            if !event::poll(Duration::from_millis(120))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind == KeyEventKind::Release {
                continue;
            }
            let Some(action) = state.handle_key(key) else {
                continue;
            };
            match action {
                TuiAction::Quit => {
                    if active_job.is_some() {
                        state.set_status(
                            "A task is still running. Let it finish before exiting so no installer or model pull is left detached.",
                        );
                    } else {
                        return Ok(());
                    }
                }
                TuiAction::Refresh => {
                    match state.reload(config, store, package_registry, installed_registry) {
                        Ok(()) => {
                            state.set_status("Local model, runner, and session state refreshed.")
                        }
                        Err(error) => state.set_status(format!("Refresh failed: {error:#}")),
                    }
                }
                TuiAction::OpenSession(id) => {
                    if active_job.is_some() {
                        state.set_status("Wait for the current task before switching sessions.");
                    } else if let Err(error) = state.activate_session(id) {
                        state.set_status(format!("Could not open session: {error:#}"));
                    }
                }
                TuiAction::NewSession => {
                    if active_job.is_some() {
                        state.set_status("Wait for the current task before creating a session.");
                    } else if let Err(error) = state.create_session() {
                        state.set_status(format!("Could not create session: {error:#}"));
                    } else {
                        state.tab = app::TuiTab::Sessions;
                    }
                }
                action => {
                    if active_job.is_some() {
                        state.set_status(
                            "Another task is already running. You can keep navigating while it finishes.",
                        );
                        continue;
                    }
                    let Some((label, args)) = task_for_action(&state, action) else {
                        continue;
                    };
                    let job = CommandJob::start(label.clone(), args);
                    state.running_label = Some(label.clone());
                    state.set_status(format!("{label}…"));
                    active_job = Some(job);
                }
            }
        }
    })?;
    Ok(())
}

fn task_for_action(app: &App, action: TuiAction) -> Option<(String, Vec<String>)> {
    let (label, command) = match action {
        TuiAction::PullModel(model) => (format!("Preparing {model}"), vec!["pull".into(), model]),
        TuiAction::RemoveModel(model) => (format!("Removing {model}"), vec!["rm".into(), model]),
        TuiAction::Speak { model, voice, text } => (
            format!("Generating speech with {model}"),
            vec![
                "speak".into(),
                text,
                "--model".into(),
                model,
                "--voice".into(),
                if voice.is_empty() {
                    "default".into()
                } else {
                    voice
                },
            ],
        ),
        TuiAction::Transcribe { model, audio } => (
            format!("Transcribing with {model}"),
            vec!["transcribe".into(), audio, "--model".into(), model],
        ),
        TuiAction::CloneVoice {
            model,
            name,
            sample,
        } => (
            format!("Creating voice profile {name}"),
            vec![
                "clone".into(),
                sample,
                "--name".into(),
                name,
                "--model".into(),
                model,
                "--consent".into(),
            ],
        ),
        TuiAction::PullRunner(runner) => (
            format!("Adding {runner}"),
            vec!["runner".into(), "pull".into(), runner],
        ),
        TuiAction::InstallRunner(runner) => (
            format!("Installing {runner}"),
            vec!["runner".into(), "install".into(), runner],
        ),
        TuiAction::RemoveRunner(runner) => (
            format!("Removing {runner}"),
            vec!["runner".into(), "rm".into(), runner],
        ),
        TuiAction::DoctorRunner(runner) => (
            format!("Checking {runner}"),
            vec!["runner".into(), "doctor".into(), runner],
        ),
        TuiAction::RunSystem(action) => system_task(action),
        TuiAction::Quit
        | TuiAction::Refresh
        | TuiAction::OpenSession(_)
        | TuiAction::NewSession => return None,
    };
    let mut args = app.workspace_args();
    args.extend(command);
    Some((label, args))
}

fn system_task(action: SystemAction) -> (String, Vec<String>) {
    match action {
        SystemAction::Status => ("Checking runtime status".into(), vec!["status".into()]),
        SystemAction::Doctor => ("Running diagnostics".into(), vec!["doctor".into()]),
        SystemAction::StartDaemon => (
            "Starting the local service".into(),
            vec!["daemon".into(), "start".into()],
        ),
        SystemAction::StopDaemon => (
            "Stopping the local service".into(),
            vec!["daemon".into(), "stop".into()],
        ),
        SystemAction::RestartDaemon => (
            "Restarting the local service".into(),
            vec!["daemon".into(), "restart".into()],
        ),
        SystemAction::Logs => (
            "Loading service logs".into(),
            vec!["daemon".into(), "logs".into()],
        ),
        SystemAction::OpenGui => ("Opening the GUI".into(), vec!["gui".into()]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_tasks_remain_semantic() {
        let (label, args) = system_task(SystemAction::Doctor);
        assert_eq!(label, "Running diagnostics");
        assert_eq!(args, vec!["doctor"]);
    }
}
