use anyhow::Context;
use std::{path::PathBuf, process::Command};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;

use crate::{distribution, workspace::CliWorkspace};

pub async fn open_gui(
    store: &LocalStore,
    config: &RuntimeConfig,
    workspace: &CliWorkspace,
) -> anyhow::Result<()> {
    ensure_server(store, config).await?;
    let url = format!("{}?{}", config.gui_url(), workspace.gui_query());

    if let Some(desktop) = distribution::desktop_executable() {
        let mut command = Command::new(&desktop);
        command.arg("--url").arg(&url);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        command
            .spawn()
            .with_context(|| format!("launch installed Takokit desktop app at {}", desktop.display()))?;
        println!("Takokit desktop: {url}");
        return Ok(());
    }

    match open::that(&url) {
        Ok(()) => println!("Opened Takokit local web GUI at {url}"),
        Err(error) => {
            println!("Takokit local web GUI: {url}");
            eprintln!("Could not open the browser automatically: {error}");
        }
    }
    Ok(())
}

pub async fn ensure_server(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    let _ = crate::daemon::ensure_running(store, config)?;
    Ok(())
}

pub fn gui_dist_path() -> PathBuf {
    if let Some(path) = std::env::var_os("TAKOKIT_GUI_DIST") {
        return PathBuf::from(path);
    }
    if let Some(root) = distribution::application_root() {
        let candidate = root.join("resources").join("gui");
        if candidate.join("index.html").is_file() {
            return candidate;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gui/dist")
}
