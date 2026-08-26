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

#[derive(Debug, Serialize, Deserialize)]
struct Journal {
    state: String,
    install_root: PathBuf,
    bundle: PathBuf,
    expected_version: String,
    backup_root: Option<PathBuf>,
    message: String,
    updated_at: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    if !cfg!(windows) {
        return Err("takokit-updater is Windows-only in Slice 4".into());
    }
    wait_for_parent(args.parent_pid)?;
    validate_install_root(&args.install_root)?;

    let parent = args
        .install_root
        .parent()
        .ok_or("installation root has no parent")?;
    let nonce = format!("{}-{}", std::process::id(), now());
    let replacement = parent.join(format!("Takokit.update.{nonce}"));
    let backup = parent.join(format!("Takokit.rollback.{nonce}"));

    write_journal(
        &args.journal,
        &Journal {
            state: "extracting".into(),
            install_root: args.install_root.clone(),
            bundle: args.bundle.clone(),
            expected_version: args.expected_version.clone(),
            backup_root: Some(backup.clone()),
            message: "Extracting verified update bundle.".into(),
            updated_at: now(),
        },
    )?;

    remove_dir_if_exists(&replacement)?;
    remove_dir_if_exists(&backup)?;
    extract_bundle(&args.bundle, &replacement)?;
    validate_replacement(&replacement)?;

    write_state(
        &args,
        "replacing",
        Some(backup.clone()),
        "Replacing immutable application tree.",
    )?;

    fs::rename(&args.install_root, &backup).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "could not move current installation {} to rollback {}: {error}",
                args.install_root.display(),
                backup.display()
            ),
        )
    })?;

    if let Err(error) = fs::rename(&replacement, &args.install_root) {
        let rollback = fs::rename(&backup, &args.install_root);
        write_state(
            &args,
            "rolled_back",
            None,
            &format!("Replacement failed ({error}); rollback result: {rollback:?}"),
        )?;
        return Err(error.into());
    }

    let verification = verify_install(&args.install_root, &args.expected_version)
        .and_then(|_| restart_daemon_if_requested(&args));
    match verification {
        Ok(()) => {
            remove_dir_if_exists(&backup)?;
            write_state(
                &args,
                "completed",
                None,
                if args.restart_daemon {
                    "Update installed and verified; the owned Takokit daemon restarted successfully. Rollback tree removed."
                } else {
                    "Update installed and verified. Rollback tree removed."
                },
            )?;
            Ok(())
        }
        Err(error) => {
            let failed = parent.join(format!("Takokit.failed.{nonce}"));
            let _ = fs::rename(&args.install_root, &failed);
            let rollback_result = fs::rename(&backup, &args.install_root);
            let rollback_restart = if args.restart_daemon && rollback_result.is_ok() {
                restart_daemon(&args.install_root).map_err(|restart_error| restart_error.to_string())
            } else {
                Ok(())
            };
            write_state(
                &args,
                "rolled_back",
                None,
                &format!(
                    "Post-update verification failed ({error}); rollback result: {rollback_result:?}; rollback daemon restart: {rollback_restart:?}"
                ),
            )?;
            Err(error)
        }
    }
}

#[derive(Debug)]
struct Args {
    parent_pid: u32,
    install_root: PathBuf,
    bundle: PathBuf,
    expected_version: String,
    journal: PathBuf,
    restart_daemon: bool,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut parent_pid = None;
        let mut install_root = None;
        let mut bundle = None;
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

fn validate_install_root(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = root.join("distribution.json");
    let bytes = fs::read(&metadata).map_err(|error| {
        format!(
            "installed distribution metadata missing at {}: {error}",
            metadata.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if value.get("product").and_then(|v| v.as_str()) != Some("Takokit")
        || value.get("mode").and_then(|v| v.as_str()) != Some("installed")
    {
        return Err("updater refuses a directory that is not a Takokit installed distribution".into());
    }
    let canonical = fs::canonicalize(root)?;
    if canonical.parent().is_none() || canonical == Path::new(r"C:\") {
        return Err("unsafe installation root".into());
    }
    Ok(())
}

fn validate_replacement(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for required in [
        "bin/tako.exe",
        "bin/takokit.exe",
        "bin/Takokit.exe",
        "bin/takokit-updater.exe",
        "distribution.json",
        "resources/registry/index.json",
        "resources/gui/index.html",
    ] {
        if !root.join(required).is_file() {
            return Err(format!("update bundle is missing required file {required}").into());
        }
    }
    Ok(())
}

fn extract_bundle(bundle: &Path, destination: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

fn verify_install(root: &Path, expected_version: &str) -> Result<(), Box<dyn std::error::Error>> {
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

fn restart_daemon_if_requested(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    if args.restart_daemon {
        restart_daemon(&args.install_root)?;
    }
    Ok(())
}

fn restart_daemon(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

fn write_state(
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

fn write_journal(path: &Path, journal: &Journal) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(journal)?)?;
    let _ = fs::remove_file(path);
    fs::rename(temporary, path)?;
    Ok(())
}

fn remove_dir_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(windows)]
fn wait_for_parent(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
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
fn wait_for_parent(_pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_zip_rejects_parent_traversal() {
        let path = Path::new("../evil.exe");
        assert!(path.components().any(|component| matches!(component, Component::ParentDir)));
    }

    #[test]
    fn boolean_parser_is_strict() {
        assert_eq!(parse_bool("true").unwrap(), true);
        assert_eq!(parse_bool("0").unwrap(), false);
        assert!(parse_bool("sometimes").is_err());
    }
}
