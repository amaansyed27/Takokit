use anyhow::{anyhow, Context};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use takokit_core::{DaemonBuildIdentity, DaemonIdentity, DaemonMode, RuntimeConfig};
use takokit_package::run_automatic_uv_cleanup;
use takokit_server::{run_server_with_listener, AppState};
use takokit_store::LocalStore;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use uuid::Uuid;

mod runtime;
mod server_control;
pub(crate) use runtime::build_freshness;
pub use runtime::write_atomic;
use runtime::{
    canonical_exe, canonical_root, daemon_lock_is_held, log_path, managed_daemon_executable,
    port_is_occupied, startup_lock, verify_identity,
};
#[cfg(test)]
use runtime::{preferred_daemon_executable, takokit_health_responds};
pub(crate) use server_control::*;

const IDENTITY_WAIT: Duration = Duration::from_secs(5);
const IDENTITY_POLL: Duration = Duration::from_millis(100);
const SHUTDOWN_ATTEMPTS: usize = 100;
const BUILD_ID: &str = env!("TAKOKIT_BUILD_ID");

pub(crate) fn current_build_id() -> &'static str {
    BUILD_ID
}

#[cfg(windows)]
pub(crate) mod windows_handle_inheritance {
    use std::ffi::c_void;

    type Handle = *mut c_void;

    const STD_INPUT_HANDLE: u32 = -10_i32 as u32;
    const STD_OUTPUT_HANDLE: u32 = -11_i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12_i32 as u32;
    const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;
    const INVALID_HANDLE_VALUE: Handle = -1_isize as Handle;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const WAIT_OBJECT_0: u32 = 0;
    const ERROR_INVALID_PARAMETER: u32 = 87;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(n_std_handle: u32) -> Handle;
        fn GetHandleInformation(handle: Handle, flags: *mut u32) -> i32;
        fn SetHandleInformation(handle: Handle, mask: u32, flags: u32) -> i32;
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> Handle;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn CloseHandle(handle: Handle) -> i32;
        fn GetLastError() -> u32;
    }

    pub struct StandardHandleInheritanceGuard {
        restored: Vec<(Handle, u32)>,
    }

    impl Drop for StandardHandleInheritanceGuard {
        fn drop(&mut self) {
            for (handle, flags) in self.restored.drain(..) {
                unsafe {
                    let _ = SetHandleInformation(handle, HANDLE_FLAG_INHERIT, flags);
                }
            }
        }
    }

    pub fn suppress() -> StandardHandleInheritanceGuard {
        let mut restored = Vec::new();
        for kind in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let handle = unsafe { GetStdHandle(kind) };
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                continue;
            }
            let mut flags = 0;
            if unsafe { GetHandleInformation(handle, &mut flags) } == 0 {
                continue;
            }
            if flags & HANDLE_FLAG_INHERIT == 0 {
                continue;
            }
            if unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) } != 0 {
                restored.push((handle, flags & HANDLE_FLAG_INHERIT));
            }
        }
        StandardHandleInheritanceGuard { restored }
    }

    pub fn process_has_exited(pid: u32) -> bool {
        let handle = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
        if handle.is_null() {
            return unsafe { GetLastError() } == ERROR_INVALID_PARAMETER;
        }
        let status = unsafe { WaitForSingleObject(handle, 0) };
        unsafe {
            let _ = CloseHandle(handle);
        }
        status == WAIT_OBJECT_0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuildFreshness {
    Current,
    Missing,
    Mismatched,
}

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
    #[serde(default)]
    pub build_id: String,
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
    ensure_fresh_running(store, config)
}

pub fn ensure_fresh_running(
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<DaemonInfo> {
    let startup_lock = startup_lock(store)?;
    startup_lock
        .lock_exclusive()
        .with_context(|| format!("lock {}", store.daemon_start_lock_path().display()))?;

    let result = (|| {
        if let Some((info, response)) = verified_runtime(store, config)? {
            match build_freshness(&response) {
                BuildFreshness::Current => return Ok(info),
                BuildFreshness::Missing => {
                    stop_locked(store, config)?.then_some(()).ok_or_else(|| {
                        anyhow!("legacy managed daemon could not be stopped for replacement")
                    })?;
                }
                BuildFreshness::Mismatched => {
                    let stale_build = response.build_id.clone();
                    stop_locked(store, config)?.then_some(()).ok_or_else(|| {
                        anyhow!(
                            "managed daemon build {stale_build} could not be stopped for replacement"
                        )
                    })?;
                }
            }
        }

        let info = start_locked(store, config)?;
        let (_, response) = verified_runtime(store, config)?.ok_or_else(|| {
            anyhow!(
                "replacement daemon started but did not publish a verified identity; see {}",
                log_path(store).display()
            )
        })?;
        if build_freshness(&response) != BuildFreshness::Current {
            return Err(anyhow!(
                "replacement daemon build verification failed: expected {}, got {}",
                current_build_id(),
                if response.build_id.is_empty() {
                    "<missing>"
                } else {
                    response.build_id.as_str()
                }
            ));
        }
        Ok(info)
    })();

    let _ = startup_lock.unlock();
    result
}

fn start_locked(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<DaemonInfo> {
    if let Some((info, response)) = verified_runtime(store, config)? {
        if build_freshness(&response) == BuildFreshness::Current {
            return Ok(info);
        }
    }
    if daemon_lock_is_held(store)? {
        return wait_for_verified(store, config)?
            .map(|(info, _)| info)
            .ok_or_else(|| anyhow!(
                "daemon process owns the runtime lock but has not published a verified API identity within {} seconds; see {}",
                IDENTITY_WAIT.as_secs(), log_path(store).display()
            ));
    }
    if port_is_occupied(config) {
        return Err(anyhow!("port {} is occupied by a direct Takokit server or another process; managed daemon will not take ownership", config.port));
    }
    cleanup_proven_stale(store, config)?;
    let instance_id = Uuid::new_v4();
    let executable = managed_daemon_executable()?;
    let log_path = log_path(store);
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let mut command = Command::new(&executable);
    command
        .arg("serve")
        .arg("--host")
        .arg(&config.host)
        .arg("--port")
        .arg(config.port.to_string())
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
    let spawn_result = {
        #[cfg(windows)]
        let _guard = windows_handle_inheritance::suppress();
        command.spawn()
    };
    spawn_result
        .with_context(|| format!("spawn managed Takokit daemon with {}", executable.display()))?;
    if let Some((info, _)) = wait_for_verified(store, config)? {
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
    crate::distribution::start_automatic_update_check();
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
        build_id: current_build_id().to_string(),
    };
    write_atomic(&store.daemon_info_path(), &info)?;
    let cleanup_root = store.root().to_path_buf();
    thread::spawn(move || {
        let _ = run_automatic_uv_cleanup(&cleanup_root);
    });
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let state = AppState::new_with_build_id(config, store.clone(), current_build_id())
        .managed(info.identity(), shutdown_tx);
    let result = run_server_with_listener(state, listener, Some(shutdown_rx)).await;
    if read_info(&store)?.is_some_and(|current| current.instance_id == instance_id) {
        let _ = fs::remove_file(store.daemon_info_path());
    }
    let _ = lock.unlock();
    result
}

pub fn status(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<Option<DaemonInfo>> {
    verified_runtime(store, config).map(|runtime| runtime.map(|(info, _)| info))
}

pub fn stop(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<bool> {
    let startup_lock = startup_lock(store)?;
    startup_lock
        .lock_exclusive()
        .with_context(|| format!("lock {}", store.daemon_start_lock_path().display()))?;
    let result = stop_locked(store, config);
    let _ = startup_lock.unlock();
    result
}

fn stop_locked(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<bool> {
    let Some((info, _)) = verified_runtime(store, config)? else {
        cleanup_proven_stale(store, config)?;
        return Ok(false);
    };
    let response = ureq::post(&format!(
        "{}/api/v1/daemon/shutdown",
        config.local_base_url()
    ))
    .send_json(serde_json::json!({"instance_id": info.instance_id}));
    if response.is_err() {
        return Err(anyhow!(
            "managed daemon refused graceful shutdown; ownership was not revoked"
        ));
    }
    for _ in 0..SHUTDOWN_ATTEMPTS {
        let port_released = !port_is_occupied(config);
        let ownership_released = !daemon_lock_is_held(store)?;
        #[cfg(windows)]
        let process_exited = windows_handle_inheritance::process_has_exited(info.pid);
        #[cfg(not(windows))]
        let process_exited = ownership_released;

        if port_released && ownership_released && process_exited {
            cleanup_proven_stale(store, config)?;
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!(
        "managed daemon accepted shutdown but process {} did not fully exit within 10 seconds; {} may still be locked",
        info.pid,
        info.executable.display()
    ))
}

pub fn logs(store: &LocalStore) -> PathBuf {
    log_path(store)
}

pub fn ensure_running(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<DaemonInfo> {
    ensure_fresh_running(store, config)
}

fn verified_runtime(
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<Option<(DaemonInfo, DaemonBuildIdentity)>> {
    let Some(info) = read_info(store)? else {
        return Ok(None);
    };
    let response = match remote_identity(config) {
        Ok(identity) => identity,
        Err(_) => return Ok(None),
    };
    verify_identity(&info, &response.identity).with_context(|| {
        format!(
            "server at {} does not match the managed daemon runtime record",
            config.local_base_url()
        )
    })?;
    Ok(Some((info, response)))
}

fn remote_identity(config: &RuntimeConfig) -> anyhow::Result<DaemonBuildIdentity> {
    ureq::get(&format!(
        "{}/api/v1/daemon/identity",
        config.local_base_url()
    ))
    .timeout(Duration::from_millis(300))
    .call()
    .map_err(|error| anyhow!("read daemon identity: {error}"))?
    .into_json()
    .map_err(Into::into)
}

fn wait_for_verified(
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<Option<(DaemonInfo, DaemonBuildIdentity)>> {
    let deadline = std::time::Instant::now() + IDENTITY_WAIT;
    loop {
        if let Some(runtime) = verified_runtime(store, config)? {
            return Ok(Some(runtime));
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

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests;
