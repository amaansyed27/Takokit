#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    let background = std::env::args_os().any(|argument| argument == "--background");
    if let Err(error) = takokit_cli::run_resident_application(!background) {
        takokit_cli::show_resident_startup_error(&error.to_string());
    }
}

#[cfg(not(windows))]
#[tokio::main]
async fn main() {
    let mut command = std::process::Command::new(
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("tako")))
            .unwrap_or_else(|| "tako".into()),
    );
    command.arg("gui");
    if let Err(error) = command.spawn() {
        eprintln!("Takokit could not launch the local browser GUI: {error}");
    }
}
