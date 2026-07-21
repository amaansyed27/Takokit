use std::{net::TcpListener, process::Command};

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn command(home: &std::path::Path, port: u16, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_takokit"))
        .args(args)
        .env("TAKOKIT_HOME", home)
        .env("TAKOKIT_PORT", port.to_string())
        .env("TAKOKIT_OUTPUT", "json")
        .output()
        .unwrap()
}

fn start(home: &std::path::Path, port: u16) -> std::process::ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_takokit"))
        .args(["daemon", "start"])
        .env("TAKOKIT_HOME", home)
        .env("TAKOKIT_PORT", port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
}

fn info(home: &std::path::Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(home.join("runtime").join("daemon.json")).unwrap())
        .unwrap()
}

struct Cleanup {
    home: std::path::PathBuf,
    port: u16,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = command(&self.home, self.port, &["daemon", "stop"]);
    }
}

#[test]
fn managed_daemon_lifecycle_is_idempotent() {
    let home = tempfile::tempdir().unwrap();
    let port = free_port();
    let _cleanup = Cleanup {
        home: home.path().to_path_buf(),
        port,
    };
    assert!(start(home.path(), port).success());
    let first = info(home.path());
    assert!(start(home.path(), port).success());
    let second = info(home.path());
    assert_eq!(first["instance_id"], second["instance_id"]);
    let status = command(home.path(), port, &["daemon", "status"]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(first["instance_id"], status["instance_id"]);
    assert!(command(home.path(), port, &["daemon", "stop"])
        .status
        .success());
}

#[test]
fn concurrent_starts_return_one_managed_identity() {
    let home = tempfile::tempdir().unwrap();
    let port = free_port();
    let _cleanup = Cleanup {
        home: home.path().to_path_buf(),
        port,
    };
    let first_home = home.path().to_path_buf();
    let second_home = first_home.clone();
    let first = std::thread::spawn(move || start(&first_home, port));
    let second = std::thread::spawn(move || start(&second_home, port));
    assert!(first.join().unwrap().success());
    assert!(second.join().unwrap().success());
    let first = info(home.path());
    let second = info(home.path());
    assert_eq!(first["instance_id"], second["instance_id"]);
}
