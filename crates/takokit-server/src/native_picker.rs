use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn pick_audio_file(initial_dir: &Path) -> Result<Option<PathBuf>, String> {
    pick_path(initial_dir, PickerKind::Audio)
}

pub fn pick_folder(initial_dir: &Path) -> Result<Option<PathBuf>, String> {
    pick_path(initial_dir, PickerKind::Folder)
}

pub fn pick_rvc_checkpoint(initial_dir: &Path) -> Result<Option<PathBuf>, String> {
    pick_path(initial_dir, PickerKind::RvcCheckpoint)
}

pub fn pick_rvc_index(initial_dir: &Path) -> Result<Option<PathBuf>, String> {
    pick_path(initial_dir, PickerKind::RvcIndex)
}

pub fn pick_rvc_package(initial_dir: &Path) -> Result<Option<PathBuf>, String> {
    pick_path(initial_dir, PickerKind::RvcPackage)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerKind {
    Audio,
    Folder,
    RvcCheckpoint,
    RvcIndex,
    RvcPackage,
}

impl PickerKind {
    #[cfg(any(windows, test))]
    fn windows_filter(self) -> &'static str {
        match self {
            Self::Audio => "Audio files|*.wav;*.mp3;*.flac;*.ogg;*.m4a;*.aac;*.wma|All files|*.*",
            Self::RvcCheckpoint => "RVC checkpoints|*.pth|All files|*.*",
            Self::RvcIndex => "RVC indexes|*.index|All files|*.*",
            Self::RvcPackage => "Takokit voice packages|*.takovoice|All files|*.*",
            Self::Folder => "All files|*.*",
        }
    }

    fn is_folder(self) -> bool {
        self == Self::Folder
    }
}

#[cfg(windows)]
fn pick_path(initial_dir: &Path, kind: PickerKind) -> Result<Option<PathBuf>, String> {
    let initial = escape_powershell_single_quoted(&initial_dir.display().to_string());
    let script = if kind.is_folder() {
        format!(
            "$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Windows.Forms; \
             [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); \
             $d=New-Object System.Windows.Forms.FolderBrowserDialog; \
             $d.SelectedPath='{initial}'; \
             if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ [Console]::Write($d.SelectedPath) }}"
        )
    } else {
        let filter = escape_powershell_single_quoted(kind.windows_filter());
        format!(
            "$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Windows.Forms; \
             [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); \
             $d=New-Object System.Windows.Forms.OpenFileDialog; \
             $d.InitialDirectory='{initial}'; \
             $d.Filter='{filter}'; \
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
fn pick_path(initial_dir: &Path, kind: PickerKind) -> Result<Option<PathBuf>, String> {
    let initial = applescript_string(&initial_dir.display().to_string());
    let chooser = if kind.is_folder() {
        "choose folder"
    } else {
        "choose file"
    };
    let prompt = if kind.is_folder() {
        "Choose a Takokit workspace"
    } else {
        "Choose a local file for Takokit"
    };
    // `osascript` is the narrow local bridge used by the loopback-only Takokit API.
    // Activating System Events first makes the native chooser foreground reliably
    // when the request originates from Safari/Chrome rather than a terminal.
    let script = format!(
        "tell application \"System Events\" to activate\n\
         try\n\
           set initialLocation to POSIX file \"{initial}\"\n\
           set selectedItem to {chooser} with prompt \"{prompt}\" default location initialLocation\n\
           return POSIX path of selectedItem\n\
         on error number -128\n\
           return \"\"\n\
         end try"
    );
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .map_err(|error| format!("could not open the macOS path picker: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "the macOS path picker failed".to_string()
        } else {
            format!("the macOS path picker failed: {stderr}")
        });
    }
    selected_path_from_stdout(&output.stdout)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn pick_path(initial_dir: &Path, kind: PickerKind) -> Result<Option<PathBuf>, String> {
    let mut zenity = Command::new("zenity");
    zenity.arg("--file-selection");
    if kind.is_folder() {
        zenity.arg("--directory");
    }
    zenity.arg(format!("--filename={}/", initial_dir.display()));
    match zenity.output() {
        Ok(output) => {
            if !output.status.success() {
                return Ok(None);
            }
            selected_path_from_stdout(&output.stdout)
        }
        Err(zenity_error) => {
            let mut kdialog = Command::new("kdialog");
            if kind.is_folder() {
                kdialog.arg("--getexistingdirectory").arg(initial_dir);
            } else {
                kdialog.arg("--getopenfilename").arg(initial_dir);
            }
            match kdialog.output() {
                Ok(output) if output.status.success() => selected_path_from_stdout(&output.stdout),
                Ok(_) => Ok(None),
                Err(kdialog_error) => Err(format!(
                    "no graphical path picker is available (zenity: {zenity_error}; kdialog: {kdialog_error}); enter an absolute path instead"
                )),
            }
        }
    }
}

fn selected_path_from_stdout(stdout: &[u8]) -> Result<Option<PathBuf>, String> {
    let value = std::str::from_utf8(stdout)
        .map_err(|error| format!("path picker returned non-UTF-8 output: {error}"))?
        .trim();
    if value.is_empty() {
        Ok(None)
    } else {
        let path = PathBuf::from(value);
        if !path.is_absolute() {
            return Err(format!(
                "path picker returned a non-absolute path: {}",
                path.display()
            ));
        }
        Ok(Some(path))
    }
}

#[cfg(any(target_os = "macos", test))]
fn applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
    fn picker_output_preserves_unicode_absolute_paths() {
        let value = if cfg!(windows) {
            "C:\\Voice Project ü\\sample.wav\r\n".to_string()
        } else {
            "/tmp/Voice Project ü/sample.wav\n".to_string()
        };
        let selected = selected_path_from_stdout(value.as_bytes())
            .unwrap()
            .unwrap();
        assert!(selected.is_absolute());
        assert!(selected.to_string_lossy().contains("Voice Project ü"));
    }

    #[test]
    fn relative_picker_output_is_rejected() {
        assert!(selected_path_from_stdout(b"relative/folder\n").is_err());
    }

    #[test]
    fn applescript_paths_escape_quotes_and_backslashes() {
        assert_eq!(applescript_string("/tmp/a\\b\"c"), "/tmp/a\\\\b\\\"c");
    }

    #[test]
    fn rvc_artifact_filters_are_specific() {
        assert!(PickerKind::RvcCheckpoint.windows_filter().contains("*.pth"));
        assert!(PickerKind::RvcIndex.windows_filter().contains("*.index"));
        assert!(PickerKind::RvcPackage
            .windows_filter()
            .contains("*.takovoice"));
    }
}
