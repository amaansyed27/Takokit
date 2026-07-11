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

const IDENTITY_WAIT: Duration = Duration::from_secs(5);
const IDENTITY_POLL: Duration = Duration::from_millis(100);

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
    let startup_lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(store.daemon_start_lock_path())
        .with_context(|| format!("open {}", store.daemon_start_lock_path().display()))?;
    startup_lock
        .lock_exclusive()
        .with_context(|| format!("lock {}", store.daemon_start_lock_path().display()))?;
    if let Some(info) = verified_status(store, config)? {
        return Ok(info);
    }
    if daemon_lock_is_held(store)? {
        return wait_for_verified(store, config)?.ok_or_else(|| anyhow!(
            "daemon process owns the runtime lock but has not published a verified API identity within {} seconds; see {}",
            IDENTITY_WAIT.as_secs(), log_path(store).display()
        ));
    }
    if port_is_occupied(config) {
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
    if let Some(info) = wait_for_verified(store, config)? {
        return Ok(info);
    }
    if daemon_lock_is_held(store)? {
        return Err(anyhow!(
            "managed child acquired the runtime lock but failed to publish a verified API identity within {} seconds; see {}",
            IDENTITY_WAIT.as_secs(),
            log_path.display()
        ));
    }
    cleanup_proven_stale(store, config)?;
    Err(anyhow!(
        "managed child exited before acquiring ownership or publishing a verified API identity within {} seconds; see {}",
        IDENTITY_WAIT.as_secs(),
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
        if !port_is_occupied(config) {
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
    verify_identity(&info, &identity).with_context(|| {
        format!(
            "server at {} does not match the managed daemon runtime record",
            config.local_base_url()
        )
    })?;
    Ok(Some(info))
}

fn wait_for_verified(
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<Option<DaemonInfo>> {
    let deadline = std::time::Instant::now() + IDENTITY_WAIT;
    loop {
        if let Some(info) = verified_status(store, config)? {
            return Ok(Some(info));
        }
        if std::time::Instant::now() >= deadline {
            return Ok(None);
        }
        thread::sleep(IDENTITY_POLL);
    }
}
fn cleanup_proven_stale(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    if port_is_occupied(config) {
        return Ok(());
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(store.daemon_lock_path())?;
    if lock.try_lock_exclusive().is_err() {
        return Ok(());
    }
    if store.daemon_info_path().is_file() {
        let _ = fs::remove_file(store.daemon_info_path());
    }
    let _ = lock.unlock();
    Ok(())
}
fn read_info(store: &LocalStore) -> anyhow::Result<Option<DaemonInfo>> {
    if !store.daemon_info_path().is_file() {
        return Ok(None);
    }
    match serde_json::from_slice(&fs::read(store.daemon_info_path())?) {
        Ok(info) => Ok(Some(info)),
        Err(_) => Ok(None),
    }
}

fn verify_identity(info: &DaemonInfo, identity: &DaemonIdentity) -> anyhow::Result<()> {
    let expected_executable = canonical_root(&info.executable)?;
    let expected_root = canonical_root(&info.storage_root)?;
    let actual_executable = canonical_root(&identity.executable)?;
    let actual_root = canonical_root(&identity.storage_root)?;
    if identity.mode != DaemonMode::Managed {
        return Err(anyhow!(
            "identity mode mismatch: expected managed, got {:?}",
            identity.mode
        ));
    }
    if identity.instance_id != Some(info.instance_id) {
        return Err(anyhow!("identity instance_id mismatch"));
    }
    if identity.pid != info.pid {
        return Err(anyhow!("identity pid mismatch"));
    }
    if actual_executable != expected_executable {
        return Err(anyhow!("identity executable mismatch"));
    }
    if actual_root != expected_root {
        return Err(anyhow!("identity storage_root mismatch"));
    }
    if identity.host != info.host {
        return Err(anyhow!("identity host mismatch"));
    }
    if identity.port != info.port {
        return Err(anyhow!("identity port mismatch"));
    }
    Ok(())
}

fn daemon_lock_is_held(store: &LocalStore) -> anyhow::Result<bool> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(store.daemon_lock_path())?;
    match lock.try_lock_exclusive() {
        Ok(()) => {
            let _ = lock.unlock();
            Ok(false)
        }
        Err(_) => Ok(true),
    }
}
pub fn write_atomic(path: &std::path::Path, value: &DaemonInfo) -> anyhow::Result<()> {
    let temp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}
fn port_is_occupied(config: &RuntimeConfig) -> bool {
    let Ok(address) = config.bind_addr().parse::<std::net::SocketAddr>() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok()
}

#[allow(dead_code)]
fn takokit_health_responds(config: &RuntimeConfig) -> bool {
    ureq::get(&format!("{}/health", config.local_base_url()))
        .timeout(Duration::from_millis(200))
        .call()
        .map(|response| response.status() == 200)
        .unwrap_or(false)
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
#[cfg(test)]
mod tests;
