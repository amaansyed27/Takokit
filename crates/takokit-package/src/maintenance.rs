//! Cross-process maintenance coordination and safe UV-cache cleanup.

use crate::{PackageError, PackageResult};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{fs::{File, OpenOptions}, io::Write, path::{Path, PathBuf}, time::{Instant, SystemTime, UNIX_EPOCH}};

pub const AUTO_CLEANUP_INTERVAL_SECS: u64 = 24 * 60 * 60;

pub struct MaintenanceGuard {
    file: File,
}

impl Drop for MaintenanceGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageCleanupReport {
    pub root: PathBuf,
    pub target: PathBuf,
    pub dry_run: bool,
    pub removed: bool,
    pub reclaimed_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutomaticCleanupState {
    pub enabled: bool,
    pub status: String,
    pub last_attempt_unix: Option<u64>,
    pub last_success_unix: Option<u64>,
    pub reclaimed_bytes: u64,
    pub elapsed_ms: u64,
    pub skip_reason: Option<String>,
    pub error: Option<String>,
}

pub fn acquire_maintenance_lock(root: &Path) -> PackageResult<MaintenanceGuard> {
    let file = open_lock(root)?;
    file.lock_exclusive()?;
    Ok(MaintenanceGuard { file })
}

pub fn try_acquire_maintenance_lock(root: &Path) -> PackageResult<Option<MaintenanceGuard>> {
    let file = open_lock(root)?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(MaintenanceGuard { file })),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn clean_uv_cache(root: &Path, dry_run: bool) -> PackageResult<StorageCleanupReport> {
    let _guard = acquire_maintenance_lock(root)?;
    clean_uv_cache_unlocked(root, dry_run)
}

pub fn automatic_cleanup_enabled() -> bool {
    std::env::var("TAKOKIT_AUTO_STORAGE_CLEANUP")
        .map(|value| !matches!(value.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off"))
        .unwrap_or(true)
}

pub fn automatic_cleanup_state(root: &Path) -> PackageResult<AutomaticCleanupState> {
    let path = state_path(root);
    if !path.is_file() {
        return Ok(AutomaticCleanupState {
            enabled: automatic_cleanup_enabled(),
            status: "never-run".to_string(),
            ..AutomaticCleanupState::default()
        });
    }
    let mut state: AutomaticCleanupState = serde_json::from_slice(&std::fs::read(path)?)?;
    state.enabled = automatic_cleanup_enabled();
    Ok(state)
}

pub fn run_automatic_uv_cleanup(root: &Path) -> AutomaticCleanupState {
    let started = Instant::now();
    let now = now();
    let enabled = automatic_cleanup_enabled();
    let previous = automatic_cleanup_state(root).unwrap_or_default();
    let mut state = AutomaticCleanupState {
        enabled,
        status: "skipped".to_string(),
        last_attempt_unix: previous.last_attempt_unix,
        last_success_unix: previous.last_success_unix,
        reclaimed_bytes: 0,
        elapsed_ms: 0,
        skip_reason: None,
        error: None,
    };

    if !enabled {
        state.skip_reason = Some("disabled by TAKOKIT_AUTO_STORAGE_CLEANUP".to_string());
        finish_state(root, &mut state, started);
        return state;
    }
    if previous.last_attempt_unix.is_some_and(|attempt| now.saturating_sub(attempt) < AUTO_CLEANUP_INTERVAL_SECS) {
        state.skip_reason = Some("throttled; cleanup ran within the last 24 hours".to_string());
        finish_state(root, &mut state, started);
        return state;
    }
    state.last_attempt_unix = Some(now);

    let guard = match try_acquire_maintenance_lock(root) {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            state.skip_reason = Some("maintenance lock is held by a pull or runtime install".to_string());
            finish_state(root, &mut state, started);
            return state;
        }
        Err(error) => {
            state.status = "failed".to_string();
            state.error = Some(error.to_string());
            finish_state(root, &mut state, started);
            return state;
        }
    };

    match clean_uv_cache_unlocked(root, false) {
        Ok(report) => {
            state.status = "completed".to_string();
            state.last_success_unix = Some(now);
            state.reclaimed_bytes = report.reclaimed_bytes;
        }
        Err(error) => {
            state.status = "failed".to_string();
            state.error = Some(error.to_string());
        }
    }
    drop(guard);
    finish_state(root, &mut state, started);
    state
}

fn clean_uv_cache_unlocked(root: &Path, dry_run: bool) -> PackageResult<StorageCleanupReport> {
    let target = root.join("cache").join("uv");
    let reclaimed_bytes = path_size(&target);
    let mut removed = false;
    if !dry_run && target.exists() {
        std::fs::remove_dir_all(&target)?;
        std::fs::create_dir_all(&target)?;
        removed = true;
    }
    Ok(StorageCleanupReport {
        root: root.to_path_buf(),
        target,
        dry_run,
        removed,
        reclaimed_bytes,
    })
}

fn open_lock(root: &Path) -> PackageResult<File> {
    let path = root.join("runtime").join("maintenance.lock");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(OpenOptions::new().read(true).write(true).create(true).open(path)?)
}

fn state_path(root: &Path) -> PathBuf {
    root.join("runtime").join("storage-cleanup.json")
}

fn finish_state(root: &Path, state: &mut AutomaticCleanupState, started: Instant) {
    state.elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let path = state_path(root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    if let Ok(source) = serde_json::to_vec_pretty(state) {
        if std::fs::write(&temporary, source).is_ok() {
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::rename(&temporary, &path);
        }
    }
    let log_path = root.join("logs").join("storage-cleanup.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut log) = OpenOptions::new().create(true).append(true).open(log_path) {
        if let Ok(line) = serde_json::to_string(state) {
            let _ = writeln!(log, "{line}");
        }
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else { return 0; };
    if metadata.file_type().is_symlink() { return 0; }
    if metadata.is_file() { return metadata.len(); }
    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| path_size(&entry.path()))
        .fold(0_u64, u64::saturating_add)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_only_removes_isolated_uv_cache() {
        let root = tempfile::tempdir().expect("tempdir");
        let uv = root.path().join("cache/uv");
        let model = root.path().join("models/fixture");
        std::fs::create_dir_all(&uv).unwrap();
        std::fs::create_dir_all(&model).unwrap();
        std::fs::write(uv.join("package"), b"cache").unwrap();
        std::fs::write(model.join("weights"), b"model").unwrap();
        let report = clean_uv_cache(root.path(), false).expect("clean");
        assert!(report.removed);
        assert!(!uv.join("package").exists());
        assert!(model.join("weights").exists());
    }

    #[test]
    fn automatic_cleanup_is_throttled_after_success() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(root.path().join("cache/uv")).unwrap();
        let first = run_automatic_uv_cleanup(root.path());
        let second = run_automatic_uv_cleanup(root.path());
        assert_eq!(first.status, "completed");
        assert_eq!(second.status, "skipped");
        assert!(second.skip_reason.unwrap().contains("throttled"));
    }
}
