use std::process::Command;

#[test]
fn both_cli_binaries_expose_the_same_nonempty_build_identifier() {
    let takokit = build_id(env!("CARGO_BIN_EXE_takokit"));
    let tako = build_id(env!("CARGO_BIN_EXE_tako"));
    assert!(!takokit.is_empty(), "takokit build identifier is empty");
    assert_eq!(
        takokit, tako,
        "tako and takokit must identify the same build"
    );
}

fn build_id(executable: &str) -> String {
    let output = Command::new(executable)
        .arg("--build-id")
        .output()
        .expect("run CLI build identifier command");
    assert!(
        output.status.success(),
        "{} --build-id failed: {}",
        executable,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("build ID is UTF-8")
        .trim()
        .to_string()
}
