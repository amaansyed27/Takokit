use super::*;

#[derive(Debug, Clone)]
pub(crate) enum ServerInspection {
    Stopped,
    Verified(DaemonIdentity),
    ForeignPort,
}

pub(crate) async fn run_foreground(store: LocalStore, config: RuntimeConfig) -> anyhow::Result<()> {
    let listener = TcpListener::bind(config.bind_addr())
        .await
        .with_context(|| format!("failed to bind Takokit server at {}", config.bind_addr()))?;
    let identity = DaemonIdentity {
        instance_id: Some(Uuid::new_v4()),
        mode: DaemonMode::Direct,
        pid: std::process::id(),
        executable: canonical_exe()?,
        storage_root: canonical_root(store.root())?,
        host: config.host.clone(),
        port: config.port,
        started_at: now(),
        log_path: None,
    };
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let state = AppState::new_with_build_id(config, store, current_build_id())
        .with_shutdown(identity, shutdown_tx);
    run_server_with_listener(state, listener, Some(shutdown_rx)).await
}

pub(crate) fn inspect_server(
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<ServerInspection> {
    if !port_is_occupied(config) {
        return Ok(ServerInspection::Stopped);
    }
    let Ok(response) = remote_identity(config) else {
        return Ok(ServerInspection::ForeignPort);
    };
    if trusted_current_runtime(store, config, &response)? {
        Ok(ServerInspection::Verified(response.identity))
    } else {
        Ok(ServerInspection::ForeignPort)
    }
}

fn trusted_current_runtime(
    store: &LocalStore,
    config: &RuntimeConfig,
    response: &DaemonBuildIdentity,
) -> anyhow::Result<bool> {
    let identity = &response.identity;
    if build_freshness(response) != BuildFreshness::Current
        || identity.instance_id.is_none()
        || identity.host != config.host
        || identity.port != config.port
        || canonical_root(&identity.storage_root)? != canonical_root(store.root())?
    {
        return Ok(false);
    }
    let current = canonical_exe()?;
    let current_dir = current
        .parent()
        .ok_or_else(|| anyhow!("Takokit executable has no parent"))?;
    let remote = canonical_root(&identity.executable)?;
    let remote_dir = remote
        .parent()
        .ok_or_else(|| anyhow!("Takokit server executable has no parent"))?;
    let trusted_name = remote
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                "tako.exe" | "takokit-server.exe" | "tako" | "takokit-server"
            )
        });
    Ok(trusted_name && canonical_root(remote_dir)? == canonical_root(current_dir)?)
}

pub(crate) fn stop_verified_server(
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<bool> {
    let identity = match inspect_server(store, config)? {
        ServerInspection::Stopped => {
            cleanup_proven_stale(store, config)?;
            return Ok(false);
        }
        ServerInspection::ForeignPort => {
            return Err(anyhow!(
                "port {} is occupied by another application; Takokit did not stop it",
                config.port
            ));
        }
        ServerInspection::Verified(identity) => identity,
    };
    let instance_id = identity
        .instance_id
        .ok_or_else(|| anyhow!("verified Takokit server did not publish an instance id"))?;
    ureq::post(&format!(
        "{}/api/v1/daemon/shutdown",
        config.local_base_url()
    ))
    .send_json(serde_json::json!({"instance_id": instance_id}))
    .map_err(|error| anyhow!("Takokit server refused graceful shutdown: {error}"))?;
    for _ in 0..SHUTDOWN_ATTEMPTS {
        if !port_is_occupied(config) && process_exited(identity.pid) {
            cleanup_proven_stale(store, config)?;
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!(
        "Takokit server accepted shutdown but process {} or port {} remained active",
        identity.pid,
        config.port
    ))
}

#[cfg(windows)]
fn process_exited(pid: u32) -> bool {
    windows_handle_inheritance::process_has_exited(pid)
}

#[cfg(not(windows))]
fn process_exited(_pid: u32) -> bool {
    true
}
