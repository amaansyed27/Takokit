#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    let background = std::env::args_os().any(|argument| argument == "--background");
    if let Err(error) = takokit_cli::run_resident_application(!background) {
        takokit_cli::show_resident_startup_error(&error.to_string());
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The Takokit resident application is available on Windows only.");
}
