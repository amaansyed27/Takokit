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

#[test]
fn alias_prefers_sibling_canonical_daemon_binary() {
    let temp = tempfile::tempdir().unwrap();
    let alias = temp.path().join(if cfg!(windows) { "tako.exe" } else { "tako" });
    let canonical = temp
        .path()
        .join(if cfg!(windows) { "takokit.exe" } else { "takokit" });
    fs::write(&alias, b"").unwrap();
    fs::write(&canonical, b"").unwrap();

    assert_eq!(preferred_daemon_executable(&alias), canonical);
}

#[test]
fn alias_falls_back_to_itself_when_canonical_binary_is_missing() {
    let temp = tempfile::tempdir().unwrap();
    let alias = temp.path().join(if cfg!(windows) { "tako.exe" } else { "tako" });
    fs::write(&alias, b"").unwrap();

    assert_eq!(preferred_daemon_executable(&alias), alias);
}

#[test]
fn canonical_binary_keeps_itself_as_daemon_executable() {
    let temp = tempfile::tempdir().unwrap();
    let canonical = temp
        .path()
        .join(if cfg!(windows) { "takokit.exe" } else { "takokit" });
    fs::write(&canonical, b"").unwrap();

    assert_eq!(preferred_daemon_executable(&canonical), canonical);
}

#[test]
fn atomic_runtime_record_replaces_previous_value() {
    let temp = tempfile::tempdir().unwrap();
    let store = LocalStore::new(temp.path().to_path_buf());
    store.ensure_layout().unwrap();
    let mut first = test_info(temp.path());
    first.pid = 1;
    let mut second = first.clone();
    second.pid = 2;
    write_atomic(&store.daemon_info_path(), &first).unwrap();
    write_atomic(&store.daemon_info_path(), &second).unwrap();
    assert_eq!(read_info(&store).unwrap().unwrap().pid, 2);
}

#[test]
fn malformed_stale_record_is_removed_when_lock_is_free() {
    let temp = tempfile::tempdir().unwrap();
    let store = LocalStore::new(temp.path().to_path_buf());
    store.ensure_layout().unwrap();
    fs::write(store.daemon_info_path(), b"not json").unwrap();
    cleanup_proven_stale(
        &store,
        &RuntimeConfig {
            host: "127.0.0.1".into(),
            port: unused_port(),
            storage_root: temp.path().to_path_buf(),
        },
    )
    .unwrap();
    assert!(!store.daemon_info_path().exists());
}

#[test]
fn stale_record_is_preserved_while_ownership_lock_is_held() {
    let temp = tempfile::tempdir().unwrap();
    let store = LocalStore::new(temp.path().to_path_buf());
    store.ensure_layout().unwrap();
    write_atomic(&store.daemon_info_path(), &test_info(temp.path())).unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(store.daemon_lock_path())
        .unwrap();
    lock.lock_exclusive().unwrap();
    cleanup_proven_stale(
        &store,
        &RuntimeConfig {
            host: "127.0.0.1".into(),
            port: unused_port(),
            storage_root: temp.path().to_path_buf(),
        },
    )
    .unwrap();
    assert!(store.daemon_info_path().exists());
    lock.unlock().unwrap();
}

#[test]
fn malformed_record_is_preserved_while_ownership_lock_is_held() {
    let temp = tempfile::tempdir().unwrap();
    let store = LocalStore::new(temp.path().to_path_buf());
    store.ensure_layout().unwrap();
    fs::write(store.daemon_info_path(), b"broken legacy record").unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(store.daemon_lock_path())
        .unwrap();
    lock.lock_exclusive().unwrap();
    cleanup_proven_stale(&store, &test_config(temp.path())).unwrap();
    assert!(store.daemon_info_path().exists());
    lock.unlock().unwrap();
}

#[test]
fn held_ownership_lock_does_not_spawn_a_competing_child() {
    let temp = tempfile::tempdir().unwrap();
    let store = LocalStore::new(temp.path().to_path_buf());
    store.ensure_layout().unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(store.daemon_lock_path())
        .unwrap();
    lock.lock_exclusive().unwrap();
    let error = start(&store, &test_config(temp.path()))
        .unwrap_err()
        .to_string();
    assert!(error.contains("owns the runtime lock"));
    assert!(error.contains("daemon.log"));
    assert!(!store.daemon_info_path().exists());
    lock.unlock().unwrap();
}

#[test]
fn direct_server_on_port_is_rejected_without_adoption() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let store = LocalStore::new(temp.path().to_path_buf());
    store.ensure_layout().unwrap();
    let error = start(
        &store,
        &RuntimeConfig {
            host: "127.0.0.1".into(),
            port,
            storage_root: temp.path().to_path_buf(),
        },
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("occupied by a direct Takokit server or another process"));
    assert!(!store.daemon_info_path().exists());
    thread.join().unwrap();
}

#[test]
fn http_404_port_is_occupied_even_when_takokit_health_fails() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let config = RuntimeConfig {
        host: "127.0.0.1".into(),
        port,
        storage_root: temp.path().to_path_buf(),
    };
    let store = LocalStore::new(temp.path().to_path_buf());
    store.ensure_layout().unwrap();
    assert!(port_is_occupied(&config));
    assert!(!takokit_health_responds(&config));
    thread.join().unwrap();
}

#[test]
fn tcp_listener_that_closes_is_still_treated_as_occupied() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let thread = std::thread::spawn(move || {
        let _ = listener.accept().unwrap();
    });
    let config = RuntimeConfig {
        host: "127.0.0.1".into(),
        port,
        storage_root: std::env::temp_dir(),
    };
    assert!(port_is_occupied(&config));
    thread.join().unwrap();
}

#[test]
fn identity_validation_reports_ownership_field_mismatches() {
    let temp = tempfile::tempdir().unwrap();
    let info = test_info(temp.path());
    let mut identity = info.identity();
    identity.executable = temp.path().join("other.exe");
    fs::write(&identity.executable, b"").unwrap();
    assert!(verify_identity(&info, &identity)
        .unwrap_err()
        .to_string()
        .contains("executable"));
    let mut identity = info.identity();
    let other = tempfile::tempdir().unwrap();
    identity.storage_root = other.path().to_path_buf();
    assert!(verify_identity(&info, &identity)
        .unwrap_err()
        .to_string()
        .contains("storage_root"));
    let mut identity = info.identity();
    identity.host = "127.0.0.2".into();
    assert!(verify_identity(&info, &identity)
        .unwrap_err()
        .to_string()
        .contains("host"));
}

fn test_info(root: &std::path::Path) -> DaemonInfo {
    let executable = root.join("takokit.exe");
    fs::write(&executable, b"").unwrap();
    DaemonInfo {
        instance_id: Uuid::new_v4(),
        pid: 42,
        executable,
        storage_root: root.to_path_buf(),
        host: "127.0.0.1".into(),
        port: 5050,
        started_at: 1,
        mode: DaemonMode::Managed,
        log_path: root.join("daemon.log"),
    }
}
fn unused_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
fn test_config(root: &std::path::Path) -> RuntimeConfig {
    RuntimeConfig {
        host: "127.0.0.1".into(),
        port: unused_port(),
        storage_root: root.to_path_buf(),
    }
}