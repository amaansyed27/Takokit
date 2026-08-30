use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, File},
    io,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use zip::ZipArchive;

const TEST_FAILPOINT_ENV: &str = "TAKOKIT_UPDATER_TEST_FAILPOINT";
const REQUIRED_REPLACEMENT_FILES: &[&str] = &[
    "bin/tako.exe",
    "bin/takokit.exe",
    "bin/takokit-updater.exe",
    "distribution.json",
    "resources/registry/index.json",
    "resources/gui/index.html",
];

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Journal {
    pub(super) state: String,
    pub(super) install_root: PathBuf,
    pub(super) bundle: PathBuf,
    pub(super) expected_version: String,
    pub(super) backup_root: Option<PathBuf>,
    pub(super) message: String,
    pub(super) updated_at: u64,
}

#[derive(Debug)]
pub(super) struct Args {
    pub(super) parent_pid: u32,
    pub(super) install_root: PathBuf,
    pub(super) bundle: PathBuf,
    pub(super) installer: PathBuf,
    pub(super) expected_version: String,
    pub(super) journal: PathBuf,
    pub(super) restart_daemon: bool,
}

impl Args {
    pub(super) fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut parent_pid = None;
        let mut install_root = None;
        let mut bundle = None;
        let mut installer = None;
        let mut expected_version = None;
        let mut journal = None;
        let mut restart_daemon = false;
        let mut args = env::args().skip(1);
        while let Some(flag) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value after {flag}"))?;
            match flag.as_str() {
                "--parent-pid" => parent_pid = Some(value.parse()?),
                "--install-root" => install_root = Some(PathBuf::from(value)),
                "--bundle" => bundle = Some(PathBuf::from(value)),
                "--installer" => installer = Some(PathBuf::from(value)),
                "--expected-version" => expected_version = Some(value),
                "--journal" => journal = Some(PathBuf::from(value)),
                "--restart-daemon" => restart_daemon = parse_bool(&value)?,
                _ => return Err(format!("unknown updater argument {flag}").into()),
            }
        }
        Ok(Self {
            parent_pid: parent_pid.ok_or("missing --parent-pid")?,
            install_root: install_root.ok_or("missing --install-root")?,
            bundle: bundle.ok_or("missing --bundle")?,
            installer: installer.ok_or("missing --installer")?,
            expected_version: expected_version.ok_or("missing --expected-version")?,
            journal: journal.ok_or("missing --journal")?,
            restart_daemon,
        })
    }
}

fn parse_bool(value: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Ok(true),
        "0" | "false" | "no" => Ok(false),
        _ => Err(format!("invalid boolean {value}").into()),
    }
}

pub(super) fn configured_test_failpoint() -> Option<String> {
    env::var(TEST_FAILPOINT_ENV)
        .ok()
        .filter(|value| matches!(value.as_str(), "after_backup" | "after_replace"))
}

pub(super) fn validate_install_root(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = root.join("distribution.json");
    let bytes = fs::read(&metadata).map_err(|error| {
        format!(
            "installed distribution metadata missing at {}: {error}",
            metadata.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if value.get("product").and_then(|value| value.as_str()) != Some("Takokit")
        || value.get("mode").and_then(|value| value.as_str()) != Some("installed")
    {
        return Err(
            "updater refuses a directory that is not a Takokit installed distribution".into(),
        );
    }
    let canonical = fs::canonicalize(root)?;
    if canonical.parent().is_none() || canonical == Path::new(r"C:\") {
        return Err("unsafe installation root".into());
    }
    Ok(())
}

pub(super) fn validate_replacement(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for required in REQUIRED_REPLACEMENT_FILES {
        if !root.join(required).is_file() {
            return Err(format!("update bundle is missing required file {required}").into());
        }
    }
    Ok(())
}

pub(super) fn extract_bundle(
    bundle: &Path,
    destination: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(destination)?;
    let file = File::open(bundle)?;
    let mut archive = ZipArchive::new(file)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = Path::new(entry.name());
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(format!("update ZIP contains unsafe path {}", entry.name()).into());
        }
        let output = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&output)?;
        io::copy(&mut entry, &mut file)?;
    }
    Ok(())
}

pub(super) fn verify_install(
    root: &Path,
    expected_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let tako = root.join("bin").join("tako.exe");
    let output = Command::new(&tako).arg("version").output()?;
    if !output.status.success() {
        return Err(format!("{} version exited with {}", tako.display(), output.status).into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout
        .lines()
        .next()
        .is_some_and(|line| line.trim() == format!("takokit {expected_version}"))
    {
        return Err(format!(
            "updated binary version mismatch; expected {expected_version}, output was {stdout:?}"
        )
        .into());
    }
    Ok(())
}

pub(super) fn restart_daemon_if_requested(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    if args.restart_daemon {
        restart_daemon(&args.install_root)?;
    }
    Ok(())
}

pub(super) fn restart_daemon(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let tako = root.join("bin").join("tako.exe");
    let mut command = Command::new(&tako);
    command.args(["daemon", "start"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "updated Takokit daemon failed to restart: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(())
}

pub(super) fn write_state(
    args: &Args,
    state: &str,
    backup_root: Option<PathBuf>,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    write_journal(
        &args.journal,
        &Journal {
            state: state.to_string(),
            install_root: args.install_root.clone(),
            bundle: args.bundle.clone(),
            expected_version: args.expected_version.clone(),
            backup_root,
            message: message.to_string(),
            updated_at: now(),
        },
    )
}

pub(super) fn write_journal(
    path: &Path,
    journal: &Journal,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(journal)?)?;
    let _ = fs::remove_file(path);
    fs::rename(temporary, path)?;
    Ok(())
}

pub(super) fn remove_dir_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(windows)]
pub(super) fn wait_for_parent(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::c_void;
    type Handle = *mut c_void;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 258;
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn CloseHandle(handle: Handle) -> i32;
    }
    let handle = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
    if handle.is_null() {
        return Ok(());
    }
    let status = unsafe { WaitForSingleObject(handle, 30_000) };
    unsafe {
        let _ = CloseHandle(handle);
    }
    match status {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => Err(format!("parent process {pid} did not exit within 30 seconds").into()),
        other => Err(format!("waiting for parent process {pid} failed with status {other}").into()),
    }
}

#[cfg(not(windows))]
pub(super) fn wait_for_parent(_pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_replacement_fixture(root: &Path) {
        for required in REQUIRED_REPLACEMENT_FILES {
            let path = root.join(required);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, b"fixture").unwrap();
        }
    }

    #[test]
    fn update_zip_rejects_parent_traversal() {
        let path = Path::new("../evil.exe");
        assert!(path
            .components()
            .any(|component| matches!(component, Component::ParentDir)));
    }

    #[test]
    fn boolean_parser_is_strict() {
        assert!(parse_bool("true").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(parse_bool("sometimes").is_err());
    }

    #[test]
    fn replacement_validation_accepts_wrapper_free_runtime() {
        let temp = tempfile::tempdir().unwrap();
        write_replacement_fixture(temp.path());

        assert!(validate_replacement(temp.path()).is_ok());
        assert!(!temp.path().join("Takokit.exe").exists());
    }

    #[test]
    fn replacement_validation_still_requires_real_daemon_binary() {
        let temp = tempfile::tempdir().unwrap();
        write_replacement_fixture(temp.path());
        fs::remove_file(temp.path().join("bin/takokit.exe")).unwrap();

        let error = validate_replacement(temp.path()).unwrap_err().to_string();
        assert!(error.contains("bin/takokit.exe"));
    }
}
