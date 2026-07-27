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
    run_logged_command_with_env(log_path, program, args, &[])
}

pub(crate) fn run_logged_command_with_env(
    log_path: &Path,
    program: impl AsRef<Path>,
    args: &[PathOrArg],
    environment: &[(&str, &str)],
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
    for (name, value) in environment {
        command.env(name, value);
    }
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

    // Python-backed package and model tools may emit Unicode even when their
    // output is redirected to a log. Windows otherwise defaults to a legacy
    // code page and can report a successful download as failed while printing
    // characters such as the Hugging Face completion checkmark.
    command.env("PYTHONUTF8", "1");
    command.env("PYTHONIOENCODING", "utf-8");

    #[cfg(windows)]
    if std::env::var_os("UV_LINK_MODE").is_none() {
        // All managed environments and the uv cache live on the same volume.
        // Hard links keep adapter isolation without storing identical package
        // files (notably Torch and CUDA libraries) more than once.
        command.env("UV_LINK_MODE", "hardlink");
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn command_env<'a>(command: &'a Command, name: &str) -> Option<&'a OsStr> {
        command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new(name))
            .and_then(|(_, value)| value)
    }

    #[test]
    fn managed_python_commands_force_utf8_output() {
        let mut command = Command::new("python");
        configure_managed_command(&mut command);

        assert_eq!(
            command_env(&command, "PYTHONUTF8"),
            Some(OsStr::new("1"))
        );
        assert_eq!(
            command_env(&command, "PYTHONIOENCODING"),
            Some(OsStr::new("utf-8"))
        );
    }
}
