use std::{process::Stdio, time::Duration};

use takokit_core::{RuntimeConfig, TakokitError};
use tokio::net::TcpStream;

pub async fn open_gui(config: &RuntimeConfig) -> anyhow::Result<()> {
    ensure_server(config).await?;

    let url = config.gui_url();
    match open::that(&url) {
        Ok(()) => println!("Opened Takokit local web GUI at {url}"),
        Err(error) => {
            println!("Takokit local web GUI: {url}");
            eprintln!("Could not open the browser automatically: {error}");
        }
    }

    Ok(())
}

pub async fn ensure_server(config: &RuntimeConfig) -> anyhow::Result<()> {
    if !server_available(config).await {
        start_server_process()?;
        wait_for_server(config).await?;
    }

    Ok(())
}

pub async fn server_available(config: &RuntimeConfig) -> bool {
    TcpStream::connect(config.bind_addr()).await.is_ok()
}

pub fn gui_dist_path() -> std::path::PathBuf {
    std::env::var("TAKOKIT_GUI_DIST")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gui/dist")
        })
}

async fn wait_for_server(config: &RuntimeConfig) -> anyhow::Result<()> {
    for _ in 0..20 {
        if server_available(config).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    Err(TakokitError::Storage(format!(
        "Takokit server did not become available at {}",
        config.local_base_url()
    ))
    .into())
}

fn start_server_process() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe)
        .arg("serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}
