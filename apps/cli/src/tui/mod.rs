mod app;
mod catalog;
mod command;
mod input;
mod job;
mod ui;

use std::{io, time::Duration};

use app::{App, TuiAction};
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
    let mut state = App::new(
        config,
        store,
        package_registry,
        installed_registry,
        format!(
            "Ready. Type a command at any time, or select an item and press Enter. Storage: {}",
            store.root().display()
        ),
    )?;
    let mut active_job: Option<CommandJob> = None;

    ratatui::run(|mut terminal| -> io::Result<()> {
        loop {
            state.tick = state.tick.wrapping_add(1);
            if let Some(result) = active_job.as_ref().and_then(CommandJob::poll) {
                state.running_command = None;
                state.last_command = Some(result.command.clone());
                state.set_status(if result.success {
                    result.output
                } else {
                    format!("Command failed.\n\n{}", result.output)
                });
                if let Err(error) =
                    state.reload(config, store, package_registry, installed_registry)
                {
                    state.status.push_str(&format!(
                        "\n\nThe command finished, but refreshing shared state failed: {error:#}"
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
                            "A command is still running. Wait for it to finish before exiting so Takokit does not leave a detached installer or model pull behind.",
                        );
                    } else {
                        return Ok(());
                    }
                }
                TuiAction::Refresh => {
                    match state.reload(config, store, package_registry, installed_registry) {
                        Ok(()) => state.set_status(
                            "Catalog, installed state, and daemon-backed data refreshed.",
                        ),
                        Err(error) => state.set_status(format!("Refresh failed: {error:#}")),
                    }
                }
                TuiAction::RunCli(args) => {
                    if active_job.is_some() {
                        state.set_status(
                            "A command is already running. You can continue navigating or prepare the next command while it finishes.",
                        );
                        continue;
                    }
                    let job = CommandJob::start(args);
                    state.running_command = Some(job.label.clone());
                    state.set_status(format!(
                        "Running `takokit {}`. The TUI will remain open and show the result here.",
                        job.label
                    ));
                    active_job = Some(job);
                }
            }
        }
    })?;

    Ok(())
}
