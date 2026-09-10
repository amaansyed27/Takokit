use super::*;
use std::io::{Read, Write};

pub(super) fn active_session_unlocked(root: &Path) -> TakokitResult<Option<Uuid>> {
    let path = root.join("active-session");
    let backup = backup_path(&path);

    let id = match read_uuid_file(&path) {
        Ok(Some(id)) => {
            if backup.exists() {
                let _ = std::fs::remove_file(&backup);
            }
            Some(id)
        }
        Ok(None) => recover_uuid_backup(&path, &backup, None)?,
        Err(primary_error) => recover_uuid_backup(&path, &backup, Some(primary_error))?,
    };

    let Some(id) = id else {
        return Ok(None);
    };
    if !root.join("sessions").join(id.to_string()).is_dir() {
        clear_active_session_state(root)?;
        return Ok(None);
    }
    Ok(Some(id))
}

pub(super) fn clear_active_session_state(root: &Path) -> TakokitResult<()> {
    let path = root.join("active-session");
    let backup = backup_path(&path);
    for candidate in [&path, &backup] {
        match std::fs::remove_file(candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(storage_error(error)),
        }
    }
    sync_parent(&path);
    Ok(())
}

pub(super) fn read_summary_recovering(id: Uuid, path: &Path) -> TakokitResult<SessionSummary> {
    let backup = backup_path(path);
    match read_summary_file(path) {
        Ok(Some(summary)) if summary.id == id => {
            if backup.exists() {
                let _ = std::fs::remove_file(&backup);
            }
            Ok(summary)
        }
        Ok(Some(summary)) => recover_summary_backup(
            id,
            path,
            &backup,
            Some(TakokitError::Storage(format!(
                "session {id} metadata identifies session {}",
                summary.id
            ))),
        ),
        Ok(None) => recover_summary_backup(id, path, &backup, None),
        Err(primary_error) => recover_summary_backup(id, path, &backup, Some(primary_error)),
    }
}

pub(super) fn reconcile_summary(
    summary: &SessionSummary,
    events: &[SessionEvent],
) -> SessionSummary {
    let mut reconciled = summary.clone();
    reconciled.event_count = events.len();
    reconciled.output_count = events
        .iter()
        .filter(|event| event.output_path.is_some())
        .count();
    if let Some(last) = events.last() {
        reconciled.updated_at = reconciled.updated_at.max(last.timestamp);
        reconciled.last_task = Some(last.task);
        reconciled.last_model = events.iter().rev().find_map(|event| event.model.clone());
        if reconciled.title.starts_with("Takokit session ") {
            reconciled.title = generated_session_title(&events[0]);
        }
    } else {
        reconciled.last_task = None;
        reconciled.last_model = None;
        reconciled.updated_at = reconciled.updated_at.max(reconciled.created_at);
    }
    reconciled
}

pub(super) fn generated_session_title(event: &SessionEvent) -> String {
    match &event.model {
        Some(model) => format!("{} · {model}", event.task.label()),
        None => event.task.label().to_string(),
    }
}

pub(super) fn read_events_recovering_torn_tail(
    path: &Path,
    expected_session: Uuid,
) -> TakokitResult<Vec<SessionEvent>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let mut source = Vec::new();
    File::open(path)
        .map_err(storage_error)?
        .read_to_end(&mut source)
        .map_err(storage_error)?;
    if source.is_empty() {
        return Ok(Vec::new());
    }

    let ends_with_newline = source.ends_with(b"\n");
    let mut events = Vec::new();
    let mut valid_bytes = 0_usize;
    let lines = source
        .split_inclusive(|byte| *byte == b'\n')
        .collect::<Vec<_>>();
    for (index, raw_line) in lines.iter().enumerate() {
        let is_last = index + 1 == lines.len();
        let line = raw_line.strip_suffix(b"\n").unwrap_or(raw_line);
        if line.iter().all(|byte| byte.is_ascii_whitespace()) {
            valid_bytes += raw_line.len();
            continue;
        }
        match serde_json::from_slice::<SessionEvent>(line) {
            Ok(event) => {
                if event.session_id != expected_session {
                    return Err(TakokitError::Storage(format!(
                        "session event log {} contains event {} for session {} instead of {}",
                        path.display(),
                        event.id,
                        event.session_id,
                        expected_session
                    )));
                }
                events.push(event);
                valid_bytes += raw_line.len();
            }
            Err(_) if is_last && !ends_with_newline => {
                let file = OpenOptions::new()
                    .write(true)
                    .open(path)
                    .map_err(storage_error)?;
                file.set_len(valid_bytes as u64).map_err(storage_error)?;
                file.sync_all().map_err(storage_error)?;
                break;
            }
            Err(error) => {
                return Err(TakokitError::Storage(format!(
                    "session event log {} is corrupt: {error}",
                    path.display()
                )))
            }
        }
    }
    Ok(events)
}

pub(super) fn replace_file(path: &Path, bytes: &[u8]) -> TakokitResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(storage_error)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(storage_error)?;
    file.write_all(bytes).map_err(storage_error)?;
    file.sync_all().map_err(storage_error)?;
    drop(file);

    let backup = backup_path(path);
    if path.is_file() {
        std::fs::copy(path, &backup).map_err(storage_error)?;
        OpenOptions::new()
            .write(true)
            .open(&backup)
            .and_then(|file| file.sync_all())
            .map_err(storage_error)?;
    }
    if let Err(error) = replace_file_platform(path, &temporary) {
        let _ = std::fs::remove_file(&temporary);
        if !path.is_file() && backup.is_file() {
            let _ = restore_backup(path, &backup);
        }
        return Err(error);
    }
    sync_parent(path);
    let _ = std::fs::remove_file(backup);
    Ok(())
}

fn read_uuid_file(path: &Path) -> TakokitResult<Option<Uuid>> {
    if !path.exists() {
        return Ok(None);
    }
    let source = std::fs::read_to_string(path).map_err(storage_error)?;
    let id = Uuid::parse_str(source.trim()).map_err(|error| {
        TakokitError::Storage(format!(
            "workspace active-session state {} is invalid: {error}",
            path.display()
        ))
    })?;
    Ok(Some(id))
}

fn recover_uuid_backup(
    path: &Path,
    backup: &Path,
    primary_error: Option<TakokitError>,
) -> TakokitResult<Option<Uuid>> {
    match read_uuid_file(backup) {
        Ok(Some(id)) => {
            restore_backup(path, backup)?;
            Ok(Some(id))
        }
        Ok(None) => match primary_error {
            Some(error) => Err(error),
            None => Ok(None),
        },
        Err(backup_error) => match primary_error {
            Some(primary_error) => Err(TakokitError::Storage(format!(
                "workspace active-session state and backup are invalid: {primary_error}; backup: {backup_error}"
            ))),
            None => Err(backup_error),
        },
    }
}

fn read_summary_file(path: &Path) -> TakokitResult<Option<SessionSummary>> {
    if !path.exists() {
        return Ok(None);
    }
    let source = std::fs::read(path).map_err(storage_error)?;
    serde_json::from_slice(&source)
        .map(Some)
        .map_err(storage_error)
}

fn recover_summary_backup(
    id: Uuid,
    path: &Path,
    backup: &Path,
    primary_error: Option<TakokitError>,
) -> TakokitResult<SessionSummary> {
    match read_summary_file(backup) {
        Ok(Some(summary)) => {
            if summary.id != id {
                return Err(TakokitError::Storage(format!(
                    "session {id} backup metadata identifies session {}",
                    summary.id
                )));
            }
            restore_backup(path, backup)?;
            Ok(summary)
        }
        Ok(None) => match primary_error {
            Some(error) => Err(error),
            None => Err(TakokitError::Storage(format!(
                "could not read session {id} at {}: file does not exist",
                path.display()
            ))),
        },
        Err(backup_error) => match primary_error {
            Some(primary_error) => Err(TakokitError::Storage(format!(
                "session {id} metadata and backup are invalid: {primary_error}; backup: {backup_error}"
            ))),
            None => Err(backup_error),
        },
    }
}

#[cfg(not(windows))]
fn replace_file_platform(path: &Path, temporary: &Path) -> TakokitResult<()> {
    std::fs::rename(temporary, path).map_err(storage_error)
}

#[cfg(windows)]
fn replace_file_platform(path: &Path, temporary: &Path) -> TakokitResult<()> {
    if path.exists() {
        std::fs::remove_file(path).map_err(storage_error)?;
    }
    std::fs::rename(temporary, path).map_err(storage_error)
}

fn restore_backup(path: &Path, backup: &Path) -> TakokitResult<()> {
    let temporary = path.with_extension(format!("restore-{}", Uuid::new_v4()));
    std::fs::copy(backup, &temporary).map_err(storage_error)?;
    OpenOptions::new()
        .write(true)
        .open(&temporary)
        .and_then(|file| file.sync_all())
        .map_err(storage_error)?;
    replace_file_platform(path, &temporary)?;
    sync_parent(path);
    let _ = std::fs::remove_file(backup);
    Ok(())
}

pub(super) fn backup_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("state");
    path.with_file_name(format!("{name}.bak"))
}

fn sync_parent(_path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = _path.parent() {
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
    }
}
