use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::{
    NewSessionEvent, SessionEvent, SessionRecord, SessionSummary, TakokitError, TakokitResult,
};
use uuid::Uuid;

const WORKSPACE_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceStore {
    workspace_root: PathBuf,
    root: PathBuf,
}

impl WorkspaceStore {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let root = workspace_root.join(".tako");
        Self {
            workspace_root,
            root,
        }
    }

    pub fn from_current_dir() -> TakokitResult<Self> {
        std::env::current_dir()
            .map(Self::new)
            .map_err(storage_error)
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    pub fn session_dir(&self, id: Uuid) -> PathBuf {
        self.sessions_dir().join(id.to_string())
    }

    pub fn session_outputs_dir(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join("outputs")
    }

    pub fn ensure_layout(&self) -> TakokitResult<()> {
        std::fs::create_dir_all(self.sessions_dir()).map_err(storage_error)?;
        let version = self.root.join("version");
        if !version.is_file() {
            replace_file(&version, WORKSPACE_VERSION.as_bytes())?;
        }
        Ok(())
    }

    pub fn create_session(&self, title: Option<&str>) -> TakokitResult<SessionRecord> {
        self.ensure_layout()?;
        let _lock = self.lock()?;
        let id = Uuid::new_v4();
        let timestamp = now();
        let session_dir = self.session_dir(id);
        std::fs::create_dir_all(session_dir.join("outputs")).map_err(storage_error)?;
        let summary = SessionSummary {
            id,
            title: normalized_title(title, timestamp),
            workspace_root: self.workspace_root.clone(),
            created_at: timestamp,
            updated_at: timestamp,
            event_count: 0,
            output_count: 0,
            last_task: None,
            last_model: None,
        };
        self.write_summary(&summary)?;
        replace_file(&session_dir.join("events.jsonl"), b"")?;
        self.set_active_session_unlocked(id)?;
        Ok(SessionRecord {
            summary,
            events: Vec::new(),
        })
    }

    pub fn open_session(
        &self,
        session_id: Option<Uuid>,
        title: Option<&str>,
    ) -> TakokitResult<SessionRecord> {
        match session_id {
            Some(id) => {
                let record = self.read_session(id)?;
                self.set_active_session(id)?;
                Ok(record)
            }
            None => self.create_session(title),
        }
    }

    pub fn active_session(&self) -> TakokitResult<Option<Uuid>> {
        if !self.root.is_dir() {
            return Ok(None);
        }
        let _lock = self.lock()?;
        active_session_unlocked(&self.root)
    }

    pub fn set_active_session(&self, id: Uuid) -> TakokitResult<()> {
        self.ensure_layout()?;
        let _lock = self.lock()?;
        if !self.session_dir(id).is_dir() {
            return Err(TakokitError::Storage(format!(
                "cannot activate missing workspace session {id}"
            )));
        }
        self.set_active_session_unlocked(id)
    }

    fn set_active_session_unlocked(&self, id: Uuid) -> TakokitResult<()> {
        replace_file(&self.root.join("active-session"), id.to_string().as_bytes())
    }

    pub fn read_session(&self, id: Uuid) -> TakokitResult<SessionRecord> {
        if !self.root.is_dir() {
            return Err(TakokitError::Storage(format!(
                "workspace session {id} does not exist"
            )));
        }
        let _lock = self.lock()?;
        let mut summary = self.read_summary(id)?;
        let events_path = self.session_dir(id).join("events.jsonl");
        let events = read_events_recovering_torn_tail(&events_path, id)?;
        let reconciled = reconcile_summary(&summary, &events);
        if reconciled != summary {
            self.write_summary(&reconciled)?;
            summary = reconciled;
        }
        Ok(SessionRecord { summary, events })
    }

    pub fn list_sessions(&self, query: Option<&str>) -> TakokitResult<Vec<SessionSummary>> {
        if !self.sessions_dir().is_dir() {
            return Ok(Vec::new());
        }
        let query = query.map(str::trim).filter(|value| !value.is_empty());
        let query_lower = query.map(str::to_lowercase);
        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(self.sessions_dir()).map_err(storage_error)? {
            let entry = entry.map_err(storage_error)?;
            if !entry.path().is_dir() {
                continue;
            }
            let Ok(id) = Uuid::parse_str(&entry.file_name().to_string_lossy()) else {
                continue;
            };
            let Ok(record) = self.read_session(id) else {
                continue;
            };
            if let Some(query) = query_lower.as_deref() {
                let summary_text = serde_json::to_string(&record.summary)
                    .map_err(storage_error)?
                    .to_lowercase();
                let events_text = serde_json::to_string(&record.events)
                    .map_err(storage_error)?
                    .to_lowercase();
                if !summary_text.contains(query) && !events_text.contains(query) {
                    continue;
                }
            }
            sessions.push(record.summary);
        }
        sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(sessions)
    }

    pub fn append_event(
        &self,
        session_id: Uuid,
        event: NewSessionEvent,
    ) -> TakokitResult<SessionEvent> {
        self.ensure_layout()?;
        let _lock = self.lock()?;
        let mut summary = self.read_summary(session_id)?;
        let event = SessionEvent {
            id: Uuid::new_v4(),
            session_id,
            timestamp: now(),
            task: event.task,
            state: event.state,
            model: event.model,
            input: event.input,
            source_path: event.source_path,
            output_path: event.output_path,
            text: event.text,
            message: event.message,
        };
        let mut encoded = serde_json::to_vec(&event).map_err(storage_error)?;
        encoded.push(b'\n');
        let events_path = self.session_dir(session_id).join("events.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(events_path)
            .map_err(storage_error)?;
        file.write_all(&encoded).map_err(storage_error)?;
        file.sync_data().map_err(storage_error)?;

        summary.updated_at = event.timestamp;
        summary.event_count += 1;
        if event.output_path.is_some() {
            summary.output_count += 1;
        }
        summary.last_task = Some(event.task);
        if let Some(model) = &event.model {
            summary.last_model = Some(model.clone());
        }
        if summary.title.starts_with("Takokit session ") {
            summary.title = generated_session_title(&event);
        }
        self.write_summary(&summary)?;
        self.set_active_session_unlocked(session_id)?;
        Ok(event)
    }

    pub fn write_text_output(
        &self,
        session_id: Uuid,
        filename: &str,
        content: &str,
    ) -> TakokitResult<PathBuf> {
        self.ensure_layout()?;
        let filename = safe_filename(filename)?;
        let _lock = self.lock()?;
        let session_dir = self.session_dir(session_id);
        if !session_dir.is_dir() {
            return Err(TakokitError::Storage(format!(
                "cannot write output for missing workspace session {session_id}"
            )));
        }
        let directory = session_dir.join("outputs");
        std::fs::create_dir_all(&directory).map_err(storage_error)?;
        let path = directory.join(filename);
        replace_file(&path, content.as_bytes())?;
        Ok(path)
    }

    pub fn remove_session(&self, id: Uuid) -> TakokitResult<bool> {
        if !self.root.is_dir() {
            return Ok(false);
        }
        let _lock = self.lock()?;
        let directory = self.session_dir(id);
        if !directory.exists() {
            return Ok(false);
        }
        std::fs::remove_dir_all(directory).map_err(storage_error)?;
        if active_session_unlocked(&self.root)? == Some(id) {
            clear_active_session_state(&self.root)?;
        }
        Ok(true)
    }

    fn summary_path(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join("session.json")
    }

    fn read_summary(&self, id: Uuid) -> TakokitResult<SessionSummary> {
        let path = self.summary_path(id);
        let backup = backup_path(&path);

        match read_summary_file(&path) {
            Ok(Some(summary)) => {
                if backup.exists() {
                    let _ = std::fs::remove_file(&backup);
                }
                Ok(summary)
            }
            Ok(None) => recover_summary_backup(id, &path, &backup, None),
            Err(primary_error) => recover_summary_backup(id, &path, &backup, Some(primary_error)),
        }
    }

    fn write_summary(&self, summary: &SessionSummary) -> TakokitResult<()> {
        let source = serde_json::to_vec_pretty(summary).map_err(storage_error)?;
        replace_file(&self.summary_path(summary.id), &source)
    }

    fn lock(&self) -> TakokitResult<WorkspaceLock> {
        let path = self.root.join("workspace.lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)
            .map_err(storage_error)?;
        file.lock_exclusive().map_err(storage_error)?;
        Ok(WorkspaceLock(file))
    }
}

struct WorkspaceLock(File);

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

fn active_session_unlocked(root: &Path) -> TakokitResult<Option<Uuid>> {
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

fn clear_active_session_state(root: &Path) -> TakokitResult<()> {
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

fn reconcile_summary(summary: &SessionSummary, events: &[SessionEvent]) -> SessionSummary {
    let mut reconciled = summary.clone();
    reconciled.event_count = events.len();
    reconciled.output_count = events
        .iter()
        .filter(|event| event.output_path.is_some())
        .count();
    if let Some(last) = events.last() {
        reconciled.updated_at = reconciled.updated_at.max(last.timestamp);
        reconciled.last_task = Some(last.task);
        reconciled.last_model = events
            .iter()
            .rev()
            .find_map(|event| event.model.clone());
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

fn generated_session_title(event: &SessionEvent) -> String {
    match &event.model {
        Some(model) => format!("{} · {model}", event.task.label()),
        None => event.task.label().to_string(),
    }
}

fn read_events_recovering_torn_tail(
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
    let lines = source.split_inclusive(|byte| *byte == b'\n').collect::<Vec<_>>();
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

fn normalized_title(title: Option<&str>, timestamp: u64) -> String {
    title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("Takokit session {timestamp}"))
}

fn safe_filename(filename: &str) -> TakokitResult<&str> {
    let value = filename.trim();
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
    {
        return Err(TakokitError::Storage(
            "output filename must be a single safe path component".to_string(),
        ));
    }
    Ok(value)
}

fn replace_file(path: &Path, bytes: &[u8]) -> TakokitResult<()> {
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
        File::open(&backup)
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
    File::open(&temporary)
        .and_then(|file| file.sync_all())
        .map_err(storage_error)?;
    replace_file_platform(path, &temporary)?;
    sync_parent(path);
    let _ = std::fs::remove_file(backup);
    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("state");
    path.with_file_name(format!("{name}.bak"))
}

fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn storage_error(error: impl std::fmt::Display) -> TakokitError {
    TakokitError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use takokit_core::{SessionEventState, SessionTask};

    #[test]
    fn listing_empty_workspace_does_not_create_tako() {
        let root = std::env::temp_dir().join(format!("takokit-empty-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let store = WorkspaceStore::new(&root);
        assert!(store.list_sessions(None).unwrap().is_empty());
        assert!(!root.join(".tako").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_sessions_persist_events_outputs_and_search() {
        let root = std::env::temp_dir().join(format!("takokit-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("workspace");
        let store = WorkspaceStore::new(&root);
        let session = store.create_session(None).expect("session");
        let output = store
            .write_text_output(session.summary.id, "transcript.txt", "hello world")
            .expect("output");
        store
            .append_event(
                session.summary.id,
                NewSessionEvent {
                    task: SessionTask::SpeechToText,
                    state: SessionEventState::Completed,
                    model: Some("whisper-tiny".into()),
                    input: None,
                    source_path: Some(root.join("audio.wav")),
                    output_path: Some(output),
                    text: Some("hello world".into()),
                    message: None,
                },
            )
            .expect("event");
        let record = store.read_session(session.summary.id).expect("record");
        assert_eq!(record.events.len(), 1);
        assert_eq!(record.summary.output_count, 1);
        assert_eq!(store.list_sessions(Some("hello world")).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn summary_replacement_supports_multiple_updates() {
        let root = std::env::temp_dir().join(format!("takokit-replace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let store = WorkspaceStore::new(&root);
        let session = store.create_session(Some("one")).unwrap();
        store.set_active_session(session.summary.id).unwrap();
        store.set_active_session(session.summary.id).unwrap();
        assert_eq!(store.active_session().unwrap(), Some(session.summary.id));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn stale_summary_is_reconciled_from_durable_events() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkspaceStore::new(root.path());
        let session = store.create_session(Some("keep title")).unwrap();
        let event = SessionEvent {
            id: Uuid::new_v4(),
            session_id: session.summary.id,
            timestamp: session.summary.created_at + 10,
            task: SessionTask::SpeechToText,
            state: SessionEventState::Completed,
            model: Some("whisper-tiny".into()),
            input: None,
            source_path: None,
            output_path: Some(PathBuf::from("out.txt")),
            text: Some("hello".into()),
            message: None,
        };
        let mut encoded = serde_json::to_vec(&event).unwrap();
        encoded.push(b'\n');
        std::fs::write(store.session_dir(session.summary.id).join("events.jsonl"), encoded).unwrap();

        let recovered = store.read_session(session.summary.id).unwrap();
        assert_eq!(recovered.summary.event_count, 1);
        assert_eq!(recovered.summary.output_count, 1);
        assert_eq!(recovered.summary.last_task, Some(SessionTask::SpeechToText));
        assert_eq!(recovered.summary.last_model.as_deref(), Some("whisper-tiny"));
        assert_eq!(recovered.summary.title, "keep title");
    }

    #[test]
    fn default_title_is_recovered_from_first_durable_event() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkspaceStore::new(root.path());
        let session = store.create_session(None).unwrap();
        let event = SessionEvent {
            id: Uuid::new_v4(),
            session_id: session.summary.id,
            timestamp: session.summary.created_at + 1,
            task: SessionTask::SpeechToText,
            state: SessionEventState::Completed,
            model: Some("whisper-tiny".into()),
            input: None,
            source_path: None,
            output_path: None,
            text: None,
            message: None,
        };
        let mut encoded = serde_json::to_vec(&event).unwrap();
        encoded.push(b'\n');
        std::fs::write(store.session_dir(session.summary.id).join("events.jsonl"), encoded).unwrap();

        let recovered = store.read_session(session.summary.id).unwrap();
        assert_eq!(recovered.summary.title, "Speech to text · whisper-tiny");
    }

    #[test]
    fn torn_final_event_is_truncated_without_losing_valid_events() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkspaceStore::new(root.path());
        let session = store.create_session(None).unwrap();
        store
            .append_event(
                session.summary.id,
                NewSessionEvent {
                    task: SessionTask::Diagnostics,
                    state: SessionEventState::Completed,
                    model: None,
                    input: None,
                    source_path: None,
                    output_path: None,
                    text: None,
                    message: Some("ok".into()),
                },
            )
            .unwrap();
        let events_path = store.session_dir(session.summary.id).join("events.jsonl");
        let mut file = OpenOptions::new().append(true).open(&events_path).unwrap();
        file.write_all(br#"{"id":"torn""#).unwrap();
        file.sync_all().unwrap();

        let recovered = store.read_session(session.summary.id).unwrap();
        assert_eq!(recovered.events.len(), 1);
        assert!(std::fs::read(&events_path).unwrap().ends_with(b"\n"));
    }

    #[test]
    fn missing_primary_summary_recovers_from_backup() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkspaceStore::new(root.path());
        let session = store.create_session(Some("recover me")).unwrap();
        let path = store.summary_path(session.summary.id);
        let backup = backup_path(&path);
        std::fs::copy(&path, &backup).unwrap();
        std::fs::remove_file(&path).unwrap();

        let recovered = store.read_session(session.summary.id).unwrap();
        assert_eq!(recovered.summary.title, "recover me");
        assert!(path.is_file());
        assert!(!backup.exists());
    }

    #[test]
    fn corrupt_primary_summary_recovers_from_valid_backup() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkspaceStore::new(root.path());
        let session = store.create_session(Some("recover me")).unwrap();
        let path = store.summary_path(session.summary.id);
        let backup = backup_path(&path);
        std::fs::copy(&path, &backup).unwrap();
        std::fs::write(&path, b"{torn").unwrap();

        let recovered = store.read_session(session.summary.id).unwrap();
        assert_eq!(recovered.summary.title, "recover me");
        assert!(serde_json::from_slice::<SessionSummary>(&std::fs::read(&path).unwrap()).is_ok());
        assert!(!backup.exists());
    }

    #[test]
    fn corrupt_active_session_recovers_from_valid_backup() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkspaceStore::new(root.path());
        let session = store.create_session(Some("active")).unwrap();
        let path = store.root().join("active-session");
        let backup = backup_path(&path);
        std::fs::copy(&path, &backup).unwrap();
        std::fs::write(&path, b"torn").unwrap();

        assert_eq!(store.active_session().unwrap(), Some(session.summary.id));
        assert_eq!(std::fs::read_to_string(&path).unwrap().trim(), session.summary.id.to_string());
        assert!(!backup.exists());
    }

    #[test]
    fn removed_active_session_does_not_resurrect_from_backup() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkspaceStore::new(root.path());
        let session = store.create_session(Some("remove")).unwrap();
        let path = store.root().join("active-session");
        let backup = backup_path(&path);
        std::fs::copy(&path, &backup).unwrap();

        assert!(store.remove_session(session.summary.id).unwrap());
        assert_eq!(store.active_session().unwrap(), None);
        assert!(!path.exists());
        assert!(!backup.exists());
    }

    #[test]
    fn foreign_session_event_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let store = WorkspaceStore::new(root.path());
        let session = store.create_session(Some("one")).unwrap();
        let other = Uuid::new_v4();
        let event = SessionEvent {
            id: Uuid::new_v4(),
            session_id: other,
            timestamp: session.summary.created_at + 1,
            task: SessionTask::Diagnostics,
            state: SessionEventState::Completed,
            model: None,
            input: None,
            source_path: None,
            output_path: None,
            text: None,
            message: None,
        };
        let mut encoded = serde_json::to_vec(&event).unwrap();
        encoded.push(b'\n');
        std::fs::write(store.session_dir(session.summary.id).join("events.jsonl"), encoded).unwrap();

        assert!(store.read_session(session.summary.id).is_err());
    }
}