mod app;
mod input;
mod job;
mod ui;

use std::{io, time::Duration};

use app::{App, SystemAction, TuiAction};
use crossterm::event::{self, Event, KeyEventKind};
use job::CommandJob;
use takokit_core::RuntimeConfig;
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

pub async fn run_launcher(
    config: &RuntimeConfig,
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let mut state = App::new(config, store, package_registry, installed_registry)?;
    let mut active_job: Option<CommandJob> = None;

    ratatui::run(|mut terminal| -> io::Result<()> {
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
                TuiAction::Refresh => match state.reload(
                    config,
                    store,
                    package_registry,
                    installed_registry,
                ) {
                    Ok(()) => state.set_status("Local model and runner state refreshed."),
                    Err(error) => state.set_status(format!("Refresh failed: {error:#}")),
                },
                action => {
                    if active_job.is_some() {
                        state.set_status(
                            "Another task is already running. You can keep navigating while it finishes.",
                        );
                        continue;
                    }
                    let Some((label, args)) = task_for_action(action) else {
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

fn task_for_action(action: TuiAction) -> Option<(String, Vec<String>)> {
    match action {
        TuiAction::PullModel(model) => Some((
            format!("Preparing {model}"),
            vec!["pull".into(), model],
        )),
        TuiAction::RemoveModel(model) => Some((
            format!("Removing {model}"),
            vec!["rm".into(), model],
        )),
        TuiAction::Speak { model, voice, text } => Some((
            format!("Generating speech with {model}"),
            vec![
                "speak".into(),
                text,
                "--model".into(),
                model,
                "--voice".into(),
                if voice.is_empty() { "default".into() } else { voice },
            ],
        )),
        TuiAction::Transcribe { model, audio } => Some((
            format!("Transcribing with {model}"),
            vec!["transcribe".into(), audio, "--model".into(), model],
        )),
        TuiAction::PullRunner(runner) => Some((
            format!("Adding {runner}"),
            vec!["runner".into(), "pull".into(), runner],
        )),
        TuiAction::InstallRunner(runner) => Some((
            format!("Installing {runner}"),
            vec!["runner".into(), "install".into(), runner],
        )),
        TuiAction::RemoveRunner(runner) => Some((
            format!("Removing {runner}"),
            vec!["runner".into(), "rm".into(), runner],
        )),
        TuiAction::DoctorRunner(runner) => Some((
            format!("Checking {runner}"),
            vec!["runner".into(), "doctor".into(), runner],
        )),
        TuiAction::RunSystem(action) => Some(system_task(action)),
        TuiAction::Quit | TuiAction::Refresh => None,
    }
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
    fn semantic_actions_map_to_the_shared_cli_backend() {
        let (_, args) = task_for_action(TuiAction::Transcribe {
            model: "whisper-tiny".into(),
            audio: "sample.wav".into(),
        })
        .unwrap();
        assert_eq!(
            args,
            vec!["transcribe", "sample.wav", "--model", "whisper-tiny"]
        );
    }
}
