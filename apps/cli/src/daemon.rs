use anyhow::{anyhow, Context};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use takokit_core::{DaemonIdentity, DaemonMode, RuntimeConfig};
use takokit_server::{run_server_with_listener, AppState};
use takokit_store::LocalStore;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    pub instance_id: Uuid,
    pub pid: u32,
    pub executable: PathBuf,
    pub storage_root: PathBuf,
    pub host: String,
    pub port: u16,
    pub started_at: u64,
    pub mode: DaemonMode,
    pub log_path: PathBuf,
}
impl DaemonInfo {
    fn identity(&self) -> DaemonIdentity {
        DaemonIdentity {
            instance_id: Some(self.instance_id),
            mode: self.mode,
            pid: self.pid,
            executable: self.executable.clone(),
            storage_root: self.storage_root.clone(),
            host: self.host.clone(),
            port: self.port,
            started_at: self.started_at,
            log_path: Some(self.log_path.clone()),
        }
    }
}

pub fn start(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<DaemonInfo> {
    if let Some(info) = verified_status(store, config)? {
        return Ok(info);
    }
    if port_responds(config) {
        return Err(anyhow!("port {} is occupied by a direct Takokit server or another process; managed daemon will not take ownership", config.port));
    }
    cleanup_proven_stale(store, config)?;
    let instance_id = Uuid::new_v4();
    let executable = std::env::current_exe()?;
    let log_path = log_path(store);
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let mut command = Command::new(&executable);
    command
        .arg("serve")
        .arg("--daemon-child")
        .arg("--instance-id")
        .arg(instance_id.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    command.spawn().context("spawn managed Takokit daemon")?;
    for _ in 0..50 {
        if let Some(info) = verified_status(store, config)? {
            return Ok(info);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!(
        "Takokit managed daemon did not publish a verified identity within 5 seconds; see {}",
        log_path.display()
    ))
}

pub async fn child(
    store: LocalStore,
    config: RuntimeConfig,
    instance_id: Uuid,
) -> anyhow::Result<()> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(store.daemon_lock_path())?;
    lock.try_lock_exclusive().map_err(|_| {
        anyhow!(
            "another managed daemon owns {}",
            store.daemon_lock_path().display()
        )
    })?;
    let listener = TcpListener::bind(config.bind_addr())
        .await
        .with_context(|| format!("managed daemon could not bind {}", config.bind_addr()))?;
    let info = DaemonInfo {
        instance_id,
        pid: std::process::id(),
        executable: canonical_exe()?,
        storage_root: canonical_root(store.root())?,
        host: config.host.clone(),
        port: config.port,
        started_at: now(),
        mode: DaemonMode::Managed,
        log_path: log_path(&store),
    };
    write_atomic(&store.daemon_info_path(), &info)?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let state = AppState::new(config, store.clone()).managed(info.identity(), shutdown_tx);
    let result = run_server_with_listener(state, listener, Some(shutdown_rx)).await;
    if read_info(&store)?.is_some_and(|current| current.instance_id == instance_id) {
        let _ = fs::remove_file(store.daemon_info_path());
    }
    let _ = lock.unlock();
    result
}

pub fn status(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<Option<DaemonInfo>> {
    verified_status(store, config)
}
pub fn stop(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<bool> {
    let Some(info) = verified_status(store, config)? else {
        cleanup_proven_stale(store, config)?;
        return Ok(false);
    };
    let response = ureq::post(&format!("{}/v1/daemon/shutdown", config.local_base_url()))
        .send_json(serde_json::json!({"instance_id": info.instance_id}));
    if response.is_err() {
        return Err(anyhow!(
            "managed daemon refused graceful shutdown; ownership was not revoked"
        ));
    }
    for _ in 0..50 {
        if !port_responds(config) {
            cleanup_proven_stale(store, config)?;
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!("managed daemon did not stop gracefully; refusing PID termination without a fresh ownership check"))
}
pub fn logs(store: &LocalStore) -> PathBuf {
    log_path(store)
}
pub fn ensure_running(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<DaemonInfo> {
    start(store, config)
}

fn verified_status(
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<Option<DaemonInfo>> {
    let Some(info) = read_info(store)? else {
        return Ok(None);
    };
    let response = match ureq::get(&format!("{}/v1/daemon/identity", config.local_base_url()))
        .timeout(Duration::from_millis(300))
        .call()
    {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let identity: DaemonIdentity = response.into_json()?;
    if identity.mode == DaemonMode::Managed
        && identity.instance_id == Some(info.instance_id)
        && identity.pid == info.pid
        && identity.storage_root == info.storage_root
        && identity.port == info.port
    {
        Ok(Some(info))
    } else {
        Err(anyhow!(
            "server at {} does not match the managed daemon runtime record",
            config.local_base_url()
        ))
    }
}
fn cleanup_proven_stale(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    if port_responds(config) {
        return Ok(());
    }
    if read_info(store)?.is_some() {
        let _ = fs::remove_file(store.daemon_info_path());
    }
    Ok(())
}
fn read_info(store: &LocalStore) -> anyhow::Result<Option<DaemonInfo>> {
    if !store.daemon_info_path().is_file() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(
        store.daemon_info_path(),
    )?)?))
}
pub fn write_atomic(path: &std::path::Path, value: &DaemonInfo) -> anyhow::Result<()> {
    let temp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}
fn port_responds(config: &RuntimeConfig) -> bool {
    ureq::get(&format!("{}/health", config.local_base_url()))
        .timeout(Duration::from_millis(200))
        .call()
        .is_ok()
}
fn log_path(store: &LocalStore) -> PathBuf {
    store.logs_dir().join("daemon.log")
}
fn canonical_exe() -> anyhow::Result<PathBuf> {
    Ok(fs::canonicalize(std::env::current_exe()?)?)
}
fn canonical_root(path: &std::path::Path) -> anyhow::Result<PathBuf> {
    Ok(fs::canonicalize(path)?)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_runtime_record_round_trips() {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalStore::new(temp.path().to_path_buf());
        store.ensure_layout().unwrap();
        let info = DaemonInfo {
            instance_id: Uuid::new_v4(),
            pid: 42,
            executable: PathBuf::from("takokit"),
            storage_root: temp.path().to_path_buf(),
            host: "127.0.0.1".to_string(),
            port: 5050,
            started_at: 1,
            mode: DaemonMode::Managed,
            log_path: temp.path().join("daemon.log"),
        };
        write_atomic(&store.daemon_info_path(), &info).unwrap();
        assert_eq!(
            read_info(&store).unwrap().unwrap().instance_id,
            info.instance_id
        );
        assert!(!store.runtime_dir().join("daemon.json.tmp").exists());
    }
}
