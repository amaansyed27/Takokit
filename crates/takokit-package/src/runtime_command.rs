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
    use std::io::Write as _;

    let program = program.as_ref();
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    write!(log, "$ {}", program.display())?;
    for arg in args {
        write!(log, " {}", arg.as_os_str().to_string_lossy())?;
    }
    writeln!(log)?;
    log.flush()?;

    let stdout = log.try_clone()?;
    let stderr = log.try_clone()?;
    let mut command = Command::new(program);
    for arg in args {
        command.arg(arg.as_os_str());
    }
    command.stdout(stdout).stderr(stderr);
    configure_managed_command(&mut command);
    let status = command
        .status()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "managed runtime command".to_string(),
            reason: format!(
                "could not start {}: {error}; see {}",
                program.display(),
                log_path.display()
            ),
        })?;
    writeln!(log, "\n[exit status: {status}]")?;

    if status.success() {
        Ok(())
    } else {
        Err(PackageError::ArtifactInstallFailed {
            artifact: "managed runtime command".to_string(),
            reason: format!(
                "{} exited with {}; see {}",
                program.display(),
                status,
                log_path.display()
            ),
        })
    }
}

pub(crate) fn configure_managed_command(command: &mut Command) {
    // Managed installs often download multi-gigabyte model/runtime files. The
    // upstream defaults are too short for slower links, so use conservative
    // retry/timeout values unless the user explicitly configured their own.
    for (name, value) in [
        ("UV_HTTP_TIMEOUT", "120"),
        ("UV_HTTP_CONNECT_TIMEOUT", "30"),
        ("UV_HTTP_RETRIES", "8"),
        ("HF_HUB_DOWNLOAD_TIMEOUT", "120"),
        ("HF_HUB_ETAG_TIMEOUT", "30"),
    ] {
        if std::env::var_os(name).is_none() {
            command.env(name, value);
        }
    }

    if let Some(root) = std::env::var_os("TAKOKIT_HOME").map(PathBuf::from) {
        for (name, path) in [
            ("UV_CACHE_DIR", root.join("cache").join("uv")),
            ("UV_PYTHON_INSTALL_DIR", root.join("tools").join("python")),
            ("UV_TOOL_DIR", root.join("tools").join("uv-tools")),
            ("UV_TOOL_BIN_DIR", root.join("tools").join("bin")),
        ] {
            if std::env::var_os(name).is_none() {
                command.env(name, path);
            }
        }
    }

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
    pub(crate) fn as_os_str(&self) -> &std::ffi::OsStr {
        match self {
            Self::Arg(value) => value.as_ref(),
            Self::Path(value) => value.as_os_str(),
        }
    }
}
