use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn pick_audio_file(initial_dir: &Path) -> Result<Option<PathBuf>, String> {
    pick_path(initial_dir, false)
}

pub(super) fn pick_folder(initial_dir: &Path) -> Result<Option<PathBuf>, String> {
    pick_path(initial_dir, true)
}

#[cfg(windows)]
fn pick_path(initial_dir: &Path, folder: bool) -> Result<Option<PathBuf>, String> {
    let initial = escape_powershell_single_quoted(&initial_dir.display().to_string());
    let script = if folder {
        format!(
            "$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Windows.Forms; \
             [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); \
             $d=New-Object System.Windows.Forms.FolderBrowserDialog; \
             $d.SelectedPath='{initial}'; \
             if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ [Console]::Write($d.SelectedPath) }}"
        )
    } else {
        format!(
            "$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Windows.Forms; \
             [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); \
             $d=New-Object System.Windows.Forms.OpenFileDialog; \
             $d.InitialDirectory='{initial}'; \
             $d.Filter='Audio files|*.wav;*.mp3;*.flac;*.ogg;*.m4a;*.aac;*.wma|All files|*.*'; \
             if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ [Console]::Write($d.FileName) }}"
        )
    };

    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-Command", &script])
        .output()
        .map_err(|error| format!("could not open the Windows path picker: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "the Windows path picker failed".to_string()
        } else {
            format!("the Windows path picker failed: {stderr}")
        });
    }
    selected_path_from_stdout(&output.stdout)
}

#[cfg(target_os = "macos")]
fn pick_path(initial_dir: &Path, folder: bool) -> Result<Option<PathBuf>, String> {
    let initial = initial_dir.display().to_string().replace('"', "\\\"");
    let kind = if folder { "folder" } else { "file" };
    let script = format!("POSIX path of (choose {kind} default location POSIX file \"{initial}\")");
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|error| format!("could not open the macOS path picker: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    selected_path_from_stdout(&output.stdout)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn pick_path(initial_dir: &Path, folder: bool) -> Result<Option<PathBuf>, String> {
    let mut command = Command::new("zenity");
    command.arg("--file-selection");
    if folder {
        command.arg("--directory");
    }
    command.arg(format!("--filename={}/", initial_dir.display()));
    let output = command.output().map_err(|error| {
        format!(
            "no graphical path picker is available ({error}); paste, drag, or enter a path instead"
        )
    })?;
    if !output.status.success() {
        return Ok(None);
    }
    selected_path_from_stdout(&output.stdout)
}

fn selected_path_from_stdout(stdout: &[u8]) -> Result<Option<PathBuf>, String> {
    let value = std::str::from_utf8(stdout)
        .map_err(|error| format!("path picker returned non-UTF-8 output: {error}"))?
        .trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(value)))
    }
}

#[cfg(windows)]
fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_picker_output_is_cancelled() {
        assert_eq!(selected_path_from_stdout(b"\r\n").unwrap(), None);
    }

    #[test]
    fn picker_output_preserves_unicode() {
        assert_eq!(
            selected_path_from_stdout("C:\\Voice Project ü\\sample.wav\r\n".as_bytes()).unwrap(),
            Some(PathBuf::from("C:\\Voice Project ü\\sample.wav"))
        );
    }
}
