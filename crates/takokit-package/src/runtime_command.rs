//! Shared managed-runtime command execution and Python path resolution.

use crate::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn runner_python_path(venv_dir: &Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![venv_dir.join("Scripts").join("python.exe")]
    } else {
        vec![
            venv_dir.join("bin").join("python3"),
            venv_dir.join("bin").join("python"),
        ]
    };
    candidates.into_iter().find(|candidate| candidate.is_file())
}

pub(crate) fn run_logged_command(
    log_path: &Path,
    program: impl AsRef<Path>,
    args: &[PathOrArg],
) -> PackageResult<()> {
    let program = program.as_ref();
    let mut command = Command::new(program);
    for arg in args {
        command.arg(arg.as_os_str());
    }
    configure_managed_command(&mut command);
    let output = command
        .output()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "managed runtime command".to_string(),
            reason: format!(
                "could not start {}: {error}; see {}",
                program.display(),
                log_path.display()
            ),
        })?;
    let mut log = String::new();
    log.push_str(&format!("$ {}", program.display()));
    for arg in args {
        log.push(' ');
        log.push_str(&arg.as_os_str().to_string_lossy());
    }
    log.push('\n');
    log.push_str(&String::from_utf8_lossy(&output.stdout));
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    log.push('\n');
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?
        .write_all(log.as_bytes())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(PackageError::ArtifactInstallFailed {
            artifact: "managed runtime command".to_string(),
            reason: format!(
                "{} exited with {}; see {}",
                program.display(),
                output.status,
                log_path.display()
            ),
        })
    }
}

pub(crate) fn configure_managed_command(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

#[derive(Debug, Clone)]
pub(crate) enum PathOrArg {
    Arg(String),
    Path(PathBuf),
}

impl From<&str> for PathOrArg {
    fn from(value: &str) -> Self {
        Self::Arg(value.to_string())
    }
}

impl From<String> for PathOrArg {
    fn from(value: String) -> Self {
        Self::Arg(value)
    }
}

impl From<PathBuf> for PathOrArg {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl PathOrArg {
    fn as_os_str(&self) -> &std::ffi::OsStr {
        match self {
            Self::Arg(value) => value.as_ref(),
            Self::Path(value) => value.as_os_str(),
        }
    }
}
