use super::*;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "Takokit";
const LEGACY_RUN_VALUE: &str = "TakokitTray";

pub(super) fn startup_enabled() -> bool {
    hidden_command("reg.exe")
        .args(["query", RUN_KEY, "/v", RUN_VALUE])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub(super) fn set_startup(enabled: bool) {
    if enabled {
        if let Ok(exe) = std::env::current_exe() {
            let value = format!("\"{}\" --background", exe.display());
            let _ = hidden_command("reg.exe")
                .args([
                    "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &value, "/f",
                ])
                .status();
            let _ = hidden_command("reg.exe")
                .args(["delete", RUN_KEY, "/v", LEGACY_RUN_VALUE, "/f"])
                .status();
        }
    } else {
        let _ = hidden_command("reg.exe")
            .args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
            .status();
        let _ = hidden_command("reg.exe")
            .args(["delete", RUN_KEY, "/v", LEGACY_RUN_VALUE, "/f"])
            .status();
    }
}
