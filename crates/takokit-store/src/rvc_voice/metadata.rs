use super::RvcVoiceLayout;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::{TakokitError, TakokitResult};
use uuid::Uuid;

pub(super) const RECOVERY_NAME_FILE: &str = "project-name.txt";

pub(super) fn validate_name(name: &str) -> TakokitResult<String> {
    let value = name.trim();
    if value.is_empty() {
        return Err(TakokitError::InvalidRequest(
            "voice name cannot be empty".into(),
        ));
    }
    if value.chars().count() > 120 {
        return Err(TakokitError::InvalidRequest(
            "voice name cannot exceed 120 characters".into(),
        ));
    }
    Ok(value.to_string())
}

pub(super) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn metadata_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_error() -> TakokitError {
    TakokitError::Storage("RVC metadata lock is poisoned".to_string())
}

fn recovery_name_path(layout: &RvcVoiceLayout) -> PathBuf {
    layout.root.join(RECOVERY_NAME_FILE)
}

pub(super) fn write_recovery_name(layout: &RvcVoiceLayout, name: &str) -> TakokitResult<()> {
    let _guard = metadata_lock().lock().map_err(|_| lock_error())?;
    let path = recovery_name_path(layout);
    fs::write(&path, name).map_err(|error| {
        TakokitError::Storage(format!(
            "could not preserve RVC project name at {}: {error}",
            path.display()
        ))
    })
}

pub(super) fn read_recovery_name(layout: &RvcVoiceLayout) -> Option<String> {
    let _guard = metadata_lock().lock().ok()?;
    fs::read_to_string(recovery_name_path(layout))
        .ok()
        .and_then(|value| validate_name(&value).ok())
}

pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> TakokitResult<T> {
    let _guard = metadata_lock().lock().map_err(|_| lock_error())?;
    match read_json_file(path) {
        Ok(value) => Ok(value),
        Err(primary) => {
            let previous = previous_json_path(path);
            let value = read_json_file(&previous).map_err(|_| primary)?;
            if path.exists() {
                fs::remove_file(path).map_err(storage_error)?;
            }
            fs::rename(previous, path).map_err(storage_error)?;
            Ok(value)
        }
    }
}

fn read_json_file<T: DeserializeOwned>(path: &Path) -> TakokitResult<T> {
    let bytes = fs::read(path).map_err(|error| {
        TakokitError::Storage(format!(
            "could not read metadata {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        TakokitError::Storage(format!("invalid metadata {}: {error}", path.display()))
    })
}

pub(super) fn atomic_json<T: Serialize>(path: &Path, value: &T) -> TakokitResult<()> {
    let _guard = metadata_lock().lock().map_err(|_| lock_error())?;
    let parent = path.parent().ok_or_else(|| {
        TakokitError::Storage(format!("metadata path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(storage_error)?;
    let temporary = path.with_extension(format!("json.tmp-{}", Uuid::new_v4()));
    let previous = previous_json_path(path);
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
    let mut file = File::create(&temporary).map_err(|error| {
        TakokitError::Storage(format!(
            "could not create temporary metadata {}: {error}",
            temporary.display()
        ))
    })?;
    file.write_all(&bytes).map_err(|error| {
        TakokitError::Storage(format!(
            "could not write temporary metadata {}: {error}",
            temporary.display()
        ))
    })?;
    file.sync_all().map_err(storage_error)?;

    if previous.exists() {
        fs::remove_file(&previous).map_err(storage_error)?;
    }
    let replaced_existing = path.exists();
    if replaced_existing {
        fs::rename(path, &previous).map_err(|error| {
            TakokitError::Storage(format!(
                "could not rotate metadata {} to {}: {error}",
                path.display(),
                previous.display()
            ))
        })?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        if replaced_existing {
            let _ = fs::rename(&previous, path);
        }
        return Err(TakokitError::Storage(format!(
            "could not publish metadata {}: {error}",
            path.display()
        )));
    }
    Ok(())
}

pub(super) fn previous_json_path(path: &Path) -> PathBuf {
    path.with_extension("json.previous")
}

pub(super) fn read_metadata_dir<T: DeserializeOwned>(dir: &Path) -> TakokitResult<Vec<T>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut values = Vec::new();
    for entry in fs::read_dir(dir).map_err(storage_error)? {
        let path = entry.map_err(storage_error)?.path();
        if is_uuid_json_record(&path) {
            values.push(read_json(&path)?);
        }
    }
    Ok(values)
}

pub(super) fn is_uuid_json_record(path: &Path) -> bool {
    path.extension().and_then(|value| value.to_str()) == Some("json")
        && path
            .file_stem()
            .and_then(|value| value.to_str())
            .is_some_and(|value| Uuid::parse_str(value).is_ok())
}

pub(super) fn storage_error(error: std::io::Error) -> TakokitError {
    TakokitError::Storage(error.to_string())
}
