#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, process::Command};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use url::Url;

fn main() {
    if let Err(error) = run() {
        show_startup_error(&error.to_string());
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let bin_dir = std::env::current_exe()?
        .parent()
        .map(PathBuf::from)
        .ok_or("desktop executable has no parent directory")?;
    let tako = bin_dir.join("tako.exe");
    if !tako.is_file() {
        return Err(format!("Takokit CLI is missing from the installed application: {}", tako.display()).into());
    }

    let mut daemon = Command::new(&tako);
    daemon.args(["daemon", "start"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        daemon.creation_flags(0x0800_0000);
    }
    let output = daemon.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Takokit could not start its managed daemon.\n\n{}{}",
            stderr.trim(),
            if stdout.trim().is_empty() { String::new() } else { format!("\n{}", stdout.trim()) }
        )
        .into());
    }

    let url = Url::parse("http://127.0.0.1:5050/gui?workspace_source=safe_default")?;
    tauri::Builder::default()
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.clone()))
                .title("Takokit 0.0.1")
                .inner_size(1280.0, 820.0)
                .min_inner_size(980.0, 640.0)
                .center()
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())?;
    Ok(())
}

#[cfg(windows)]
fn show_startup_error(message: &str) {
    use std::os::windows::ffi::OsStrExt;
    use std::{ffi::OsStr, ptr};
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }
    let text: Vec<u16> = OsStr::new(message).encode_wide().chain(Some(0)).collect();
    let caption: Vec<u16> = OsStr::new("Takokit startup error").encode_wide().chain(Some(0)).collect();
    unsafe {
        let _ = MessageBoxW(ptr::null_mut(), text.as_ptr(), caption.as_ptr(), 0x10);
    }
}

#[cfg(not(windows))]
fn show_startup_error(message: &str) {
    eprintln!("Takokit startup error: {message}");
}
