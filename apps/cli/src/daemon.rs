use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonInfo {
    pub pid: u32,
    pub host: String,
    pub port: u16,
    pub started_at: u64,
    pub executable: PathBuf,
    pub log_path: PathBuf,
}

pub fn start(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<DaemonInfo> {
    if health(config) {
        return status(store, config)?.ok_or_else(|| anyhow!("Takokit daemon is already healthy"));
    }
    clear_stale(store)?;
    let lock = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(store.daemon_lock_path())
        .with_context(|| {
            format!(
                "another Takokit daemon start is in progress or already owns {}",
                store.daemon_lock_path().display()
            )
        })?;
    drop(lock);
    let executable = std::env::current_exe()?;
    let log_path = store.logs_dir().join("daemon.log");
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let mut command = Command::new(&executable);
    command
        .arg("serve")
        .arg("--daemon-child")
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = std::fs::remove_file(store.daemon_lock_path());
            return Err(error.into());
        }
    };
    let info = DaemonInfo {
        pid: child.id(),
        host: config.host.clone(),
        port: config.port,
        started_at: now(),
        executable,
        log_path,
    };
    std::fs::write(store.daemon_pid_path(), info.pid.to_string())?;
    std::fs::write(store.daemon_info_path(), serde_json::to_vec_pretty(&info)?)?;
    for _ in 0..30 {
        if health(config) {
            return Ok(info);
        }
        thread::sleep(Duration::from_millis(200));
    }
    let _ = std::fs::remove_file(store.daemon_lock_path());
    Err(anyhow!(
        "Takokit daemon did not become healthy within 6 seconds; see {}",
        store.logs_dir().join("daemon.log").display()
    ))
}

pub fn status(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<Option<DaemonInfo>> {
    if !health(config) {
        return Ok(None);
    }
    let source = std::fs::read(store.daemon_info_path())
        .context("daemon is healthy but daemon.json is missing")?;
    Ok(Some(serde_json::from_slice(&source)?))
}

pub fn stop(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<bool> {
    let Some(info) = status(store, config)? else {
        clear_stale(store)?;
        return Ok(false);
    };
    if info.executable != std::env::current_exe()? {
        return Err(anyhow!(
            "refusing to terminate daemon with unexpected executable {}",
            info.executable.display()
        ));
    }
    #[cfg(windows)]
    let outcome = Command::new("taskkill")
        .args(["/PID", &info.pid.to_string()])
        .output()?;
    #[cfg(not(windows))]
    let outcome = Command::new("kill")
        .args(["-TERM", &info.pid.to_string()])
        .output()?;
    if !outcome.status.success() {
        return Err(anyhow!(
            "could not stop Takokit daemon pid {}; see {}",
            info.pid,
            info.log_path.display()
        ));
    }
    for _ in 0..20 {
        if !health(config) {
            clear_stale(store)?;
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!("Takokit daemon did not stop cleanly"))
}

pub fn logs(store: &LocalStore) -> PathBuf {
    store.logs_dir().join("daemon.log")
}

fn health(config: &RuntimeConfig) -> bool {
    ureq::get(&format!("{}/health", config.local_base_url()))
        .timeout(Duration::from_millis(250))
        .call()
        .map(|response| response.status() == 200)
        .unwrap_or(false)
}
fn clear_stale(store: &LocalStore) -> anyhow::Result<()> {
    for path in [
        store.daemon_pid_path(),
        store.daemon_lock_path(),
        store.daemon_info_path(),
    ] {
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
