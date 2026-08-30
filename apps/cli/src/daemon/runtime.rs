use super::*;

pub(super) fn verify_identity(info: &DaemonInfo, identity: &DaemonIdentity) -> anyhow::Result<()> {
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

pub(crate) fn build_freshness(response: &DaemonBuildIdentity) -> BuildFreshness {
    if response.build_id.trim().is_empty() {
        BuildFreshness::Missing
    } else if response.build_id == current_build_id() {
        BuildFreshness::Current
    } else {
        BuildFreshness::Mismatched
    }
}

pub(super) fn startup_lock(store: &LocalStore) -> anyhow::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(store.daemon_start_lock_path())
        .with_context(|| format!("open {}", store.daemon_start_lock_path().display()))
}

pub(super) fn daemon_lock_is_held(store: &LocalStore) -> anyhow::Result<bool> {
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

pub fn write_atomic(path: &Path, value: &DaemonInfo) -> anyhow::Result<()> {
    let temp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}

pub(super) fn port_is_occupied(config: &RuntimeConfig) -> bool {
    let Ok(address) = config.bind_addr().parse::<std::net::SocketAddr>() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok()
}

#[allow(dead_code)]
pub(super) fn takokit_health_responds(config: &RuntimeConfig) -> bool {
    ureq::get(&format!("{}/health", config.local_base_url()))
        .timeout(Duration::from_millis(200))
        .call()
        .map(|response| response.status() == 200)
        .unwrap_or(false)
}

pub(super) fn log_path(store: &LocalStore) -> PathBuf {
    store.logs_dir().join("daemon.log")
}

pub(super) fn managed_daemon_executable() -> anyhow::Result<PathBuf> {
    let current = std::env::current_exe()?;
    Ok(preferred_daemon_executable(&current))
}

pub(super) fn preferred_daemon_executable(current: &Path) -> PathBuf {
    let Some(stem) = current.file_stem().and_then(|name| name.to_str()) else {
        return current.to_path_buf();
    };
    if !stem.eq_ignore_ascii_case("tako") && !stem.eq_ignore_ascii_case("Takokit") {
        return current.to_path_buf();
    }
    let mut canonical = current.to_path_buf();
    canonical.set_file_name("takokit-server");
    if let Some(extension) = current.extension() {
        canonical.set_extension(extension);
    }
    if canonical.is_file() {
        canonical
    } else {
        current.to_path_buf()
    }
}

pub(super) fn canonical_exe() -> anyhow::Result<PathBuf> {
    Ok(fs::canonicalize(std::env::current_exe()?)?)
}

pub(super) fn canonical_root(path: &Path) -> anyhow::Result<PathBuf> {
    Ok(fs::canonicalize(path)?)
}
