use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct AutomaticCheckReport {
    available_version: Option<String>,
}

pub(super) fn check_update_async(hwnd: HWND, automatic: bool) {
    {
        let mut state = STATE.get().unwrap().lock().unwrap();
        if state.update_check_running {
            return;
        }
        state.update_check_running = true;
    }
    let hwnd = hwnd as isize;
    thread::spawn(move || {
        let args = if automatic {
            vec!["--output", "json", "update", "auto-check"]
        } else {
            vec!["--output", "json", "update", "check"]
        };
        let output = std::env::current_exe()
            .ok()
            .and_then(|exe| hidden_command(exe).args(args).output().ok());
        let successful = output.as_ref().is_some_and(|value| value.status.success());
        let version = output
            .filter(|value| value.status.success())
            .and_then(|value| parse_update_version(&value.stdout, automatic));
        let mut state = STATE.get().unwrap().lock().unwrap();
        let newly_available = version.is_some() && version != state.update_version;
        state.update_version = version;
        state.update_check_running = false;
        state.last_update_check_failed = !successful;
        drop(state);
        unsafe {
            PostMessageW(
                hwnd as HWND,
                if newly_available {
                    UPDATE_READY
                } else if !automatic {
                    UPDATE_CHECKED
                } else {
                    STATE_CHANGED
                },
                0,
                0,
            )
        };
    });
}

pub(super) unsafe fn show_update_check_result(hwnd: HWND) {
    let state = STATE.get().unwrap().lock().unwrap();
    let message = if state.last_update_check_failed {
        "Takokit could not check for updates. Try again when the network is available."
    } else {
        "Takokit is up to date."
    };
    MessageBoxW(
        hwnd,
        wide(message).as_ptr(),
        wide("Takokit Updates").as_ptr(),
        MB_OK | MB_ICONINFORMATION,
    );
}

pub(super) fn parse_update_version(output: &[u8], automatic: bool) -> Option<String> {
    if automatic {
        return serde_json::from_slice::<AutomaticCheckReport>(output)
            .ok()
            .and_then(|report| report.available_version);
    }
    let value: serde_json::Value = serde_json::from_slice(output).ok()?;
    value
        .get("available")
        .and_then(|available| available.as_bool())
        .unwrap_or(false)
        .then(|| value.get("offered_version")?.as_str().map(str::to_owned))
        .flatten()
}

pub(super) fn apply_update_async() {
    thread::spawn(|| {
        if let Ok(exe) = std::env::current_exe() {
            let _ = hidden_command(exe).args(["update", "apply"]).spawn();
        }
    });
}

pub(super) unsafe fn show_update_notification(hwnd: HWND) {
    let Some(version) = STATE.get().unwrap().lock().unwrap().update_version.clone() else {
        return;
    };
    let mut data = notification_data(hwnd);
    data.uFlags = NIF_INFO;
    copy_wide(&mut data.szInfoTitle, "Takokit update available");
    copy_wide(
        &mut data.szInfo,
        &format!("Takokit v{version} is ready to install. Click to install the signed update."),
    );
    data.dwInfoFlags = 1;
    Shell_NotifyIconW(NIM_MODIFY, &data);
}
