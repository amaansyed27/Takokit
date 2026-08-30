use super::*;

pub(crate) fn ensure_running(open_gui: bool) -> anyhow::Result<()> {
    let class_name = wide("TakokitResidentWindow");
    let hwnd = unsafe { FindWindowW(class_name.as_ptr(), null()) };
    if !hwnd.is_null() {
        if open_gui {
            unsafe { PostMessageW(hwnd, WM_COMMAND, ID_OPEN_GUI, 0) };
        }
        return Ok(());
    }
    let executable = resident_executable()?;
    let mut command = Command::new(&executable);
    if !open_gui {
        command.arg("--background");
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("start {}: {error}", executable.display()))?;
    Ok(())
}

fn resident_executable() -> anyhow::Result<PathBuf> {
    let current = std::env::current_exe()?;
    if current
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("Takokit"))
    {
        return Ok(current);
    }
    let sibling = current.with_file_name("Takokit.exe");
    if sibling.is_file() {
        Ok(sibling)
    } else {
        anyhow::bail!(
            "Takokit resident application is missing: {}",
            sibling.display()
        )
    }
}

pub(super) fn tako_executable() -> anyhow::Result<PathBuf> {
    let sibling = std::env::current_exe()?.with_file_name("tako.exe");
    if sibling.is_file() {
        Ok(sibling)
    } else {
        anyhow::bail!("Takokit CLI is missing: {}", sibling.display())
    }
}

pub(crate) fn show_startup_error(message: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(message).as_ptr(),
            wide("Takokit could not start").as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
