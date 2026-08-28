use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;

use crate::{distribution, workspace::CliWorkspace};

const GUI_READY_TIMEOUT: Duration = Duration::from_secs(10);
const GUI_READY_POLL: Duration = Duration::from_millis(100);

pub async fn open_gui(
    store: &LocalStore,
    config: &RuntimeConfig,
    workspace: &CliWorkspace,
) -> anyhow::Result<()> {
    ensure_server(store, config).await?;
    wait_for_gui_ready_at(&config.gui_url(), GUI_READY_TIMEOUT, GUI_READY_POLL)?;

    let url = format!("{}?{}", config.gui_url(), workspace.gui_query());
    let _ = launch_browser(&url, |target| open::that(target));
    Ok(())
}

pub async fn ensure_server(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    let _ = crate::daemon::ensure_running(store, config)?;
    Ok(())
}

fn wait_for_gui_ready_at(url: &str, timeout: Duration, poll: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last_error = "no response".to_string();

    loop {
        match ureq::get(url).timeout(Duration::from_secs(1)).call() {
            Ok(response) if response.status() == 200 => return Ok(()),
            Ok(response) => last_error = format!("HTTP {}", response.status()),
            Err(error) => last_error = error.to_string(),
        }

        if Instant::now() >= deadline {
            return Err(anyhow::anyhow!(
                "Takokit daemon started but the GUI did not become ready at {url} within {} ms: {last_error}",
                timeout.as_millis()
            ));
        }
        std::thread::sleep(poll);
    }
}

fn launch_browser<F>(url: &str, opener: F) -> bool
where
    F: FnOnce(&str) -> std::io::Result<()>,
{
    match opener(url) {
        Ok(()) => {
            println!("Opened Takokit local web GUI at {url}");
            true
        }
        Err(error) => {
            println!("Takokit local web GUI: {url}");
            eprintln!("Could not open the browser automatically: {error}");
            false
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    #[test]
    fn gui_readiness_accepts_http_200() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                )
                .unwrap();
        });

        let url = format!("http://{address}/gui");
        wait_for_gui_ready_at(&url, Duration::from_secs(2), Duration::from_millis(10)).unwrap();
        server.join().unwrap();
    }

    #[test]
    fn gui_readiness_timeout_reports_the_url() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let url = format!("http://{address}/gui");

        let error = wait_for_gui_ready_at(
            &url,
            Duration::from_millis(120),
            Duration::from_millis(20),
        )
        .unwrap_err();
        assert!(error.to_string().contains(&url));
    }

    #[test]
    fn browser_launch_success_is_reported() {
        assert!(launch_browser("http://127.0.0.1:5050/gui", |_| Ok(())));
    }

    #[test]
    fn browser_launch_failure_falls_back_without_failing_gui_startup() {
        assert!(!launch_browser("http://127.0.0.1:5050/gui", |_| {
            Err(std::io::Error::other("browser unavailable"))
        }));
    }
}
