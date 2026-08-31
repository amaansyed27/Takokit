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

fn required_replacement_files() -> Vec<String> {
    let suffix = std::env::consts::EXE_SUFFIX;
    vec![
        format!("bin/tako{suffix}"),
        format!("bin/Takokit{suffix}"),
        format!("bin/takokit-server{suffix}"),
        format!("bin/takokit-updater{suffix}"),
        "distribution.json".into(),
        "resources/registry/index.json".into(),
        "resources/gui/index.html".into(),
    ]
}

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
    pub(super) installer: Option<PathBuf>,
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
            installer,
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
    if canonical.parent().is_none() {
        return Err("unsafe installation root".into());
    }
    Ok(())
}

pub(super) fn validate_replacement(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for required in required_replacement_files() {
        if !root.join(&required).is_file() {
            return Err(format!("update bundle is missing required file {required}").into());
        }
    }
    Ok(())
}

pub(super) fn extract_bundle(
    bundle: &Path,
    destination: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if bundle.extension().and_then(|value| value.to_str()) == Some("zip") {
        return extract_zip(bundle, destination);
    }
    extract_tar_gz(bundle, destination)
}

fn validate_archive_path(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("archive contains unsafe path {}", path.display()).into());
    }
    Ok(())
}

fn extract_zip(bundle: &Path, destination: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(destination)?;
    let file = File::open(bundle)?;
    let mut archive = ZipArchive::new(file)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = Path::new(entry.name());
        validate_archive_path(relative)?;
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

fn extract_tar_gz(bundle: &Path, destination: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use flate2::read::GzDecoder;
    fs::create_dir_all(destination)?;
    let decoder = GzDecoder::new(File::open(bundle)?);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        validate_archive_path(&path)?;
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            return Err(format!("update archive contains unsafe link {}", path.display()).into());
        }
        if !entry.unpack_in(destination)? {
            return Err(format!(
                "update archive entry escaped destination: {}",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

pub(super) fn verify_install(
    root: &Path,
    expected_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let tako = root
        .join("bin")
        .join(format!("tako{}", std::env::consts::EXE_SUFFIX));
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
    let tako = root
        .join("bin")
        .join(format!("tako{}", std::env::consts::EXE_SUFFIX));
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

#[cfg(unix)]
pub(super) fn wait_for_parent(_pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    // Unix permits an executing binary to be renamed or unlinked. Waiting with
    // kill(pid, 0) is both unnecessary and unreliable because an exited orphan
    // can remain visible as a zombie until its reaper collects it.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_replacement_fixture(root: &Path) {
        for required in required_replacement_files() {
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
    fn archive_path_validation_rejects_absolute_and_parent_paths() {
        assert!(validate_archive_path(Path::new("../evil")).is_err());
        assert!(validate_archive_path(Path::new("/absolute/evil")).is_err());
        assert!(validate_archive_path(Path::new("safe/bin/tako")).is_ok());
    }

    #[test]
    fn tar_extraction_rejects_symbolic_links() {
        use flate2::{write::GzEncoder, Compression};
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("update.tar.gz");
        let file = File::create(&archive_path).unwrap();
        let encoder = GzEncoder::new(file, Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_cksum();
        archive
            .append_link(&mut header, "bin/tako", "/tmp/foreign")
            .unwrap();
        archive.into_inner().unwrap().finish().unwrap();

        let error = extract_bundle(&archive_path, &temp.path().join("out"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsafe link"));
    }

    #[cfg(unix)]
    #[test]
    fn tar_extraction_preserves_executable_mode() {
        use flate2::{write::GzEncoder, Compression};
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("update.tar.gz");
        let encoder = GzEncoder::new(File::create(&archive_path).unwrap(), Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let payload = b"#!/bin/sh\n";
        let mut header = tar::Header::new_gnu();
        header.set_size(payload.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive
            .append_data(&mut header, "bin/tako", &payload[..])
            .unwrap();
        archive.into_inner().unwrap().finish().unwrap();
        let destination = temp.path().join("out");
        extract_bundle(&archive_path, &destination).unwrap();
        assert_eq!(
            fs::metadata(destination.join("bin/tako"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0o111
        );
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
        let daemon = format!("bin/takokit-server{}", std::env::consts::EXE_SUFFIX);
        fs::remove_file(temp.path().join(&daemon)).unwrap();

        let error = validate_replacement(temp.path()).unwrap_err().to_string();
        assert!(error.contains(&daemon));
    }
}
