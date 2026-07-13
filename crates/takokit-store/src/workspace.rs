use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
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
        self.set_active_session(id)?;
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
        let path = self.root.join("active-session");
        if !path.is_file() {
            return Ok(None);
        }
        let value = std::fs::read_to_string(path).map_err(storage_error)?;
        Ok(Uuid::parse_str(value.trim()).ok())
    }

    pub fn set_active_session(&self, id: Uuid) -> TakokitResult<()> {
        self.ensure_layout()?;
        replace_file(&self.root.join("active-session"), id.to_string().as_bytes())
    }

    pub fn read_session(&self, id: Uuid) -> TakokitResult<SessionRecord> {
        let summary = self.read_summary(id)?;
        let events_path = self.session_dir(id).join("events.jsonl");
        let mut events = Vec::new();
        if events_path.is_file() {
            let reader = BufReader::new(File::open(events_path).map_err(storage_error)?);
            for line in reader.lines() {
                let line = line.map_err(storage_error)?;
                if line.trim().is_empty() {
                    continue;
                }
                events.push(serde_json::from_str(&line).map_err(storage_error)?);
            }
        }
        Ok(SessionRecord { summary, events })
    }

    pub fn list_sessions(&self, query: Option<&str>) -> TakokitResult<Vec<SessionSummary>> {
        self.ensure_layout()?;
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
            let Ok(summary) = self.read_summary(id) else {
                continue;
            };
            if let Some(query) = query_lower.as_deref() {
                let summary_text = serde_json::to_string(&summary)
                    .map_err(storage_error)?
                    .to_lowercase();
                let events_text = std::fs::read_to_string(entry.path().join("events.jsonl"))
                    .unwrap_or_default()
                    .to_lowercase();
                if !summary_text.contains(query) && !events_text.contains(query) {
                    continue;
                }
            }
            sessions.push(summary);
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
        let line = serde_json::to_string(&event).map_err(storage_error)?;
        let events_path = self.session_dir(session_id).join("events.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(events_path)
            .map_err(storage_error)?;
        writeln!(file, "{line}").map_err(storage_error)?;
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
            summary.title = match &event.model {
                Some(model) => format!("{} · {model}", event.task.label()),
                None => event.task.label().to_string(),
            };
        }
        self.write_summary(&summary)?;
        self.set_active_session(session_id)?;
        Ok(event)
    }

    pub fn write_text_output(
        &self,
        session_id: Uuid,
        filename: &str,
        content: &str,
    ) -> TakokitResult<PathBuf> {
        let filename = safe_filename(filename)?;
        let directory = self.session_outputs_dir(session_id);
        std::fs::create_dir_all(&directory).map_err(storage_error)?;
        let path = directory.join(filename);
        replace_file(&path, content.as_bytes())?;
        Ok(path)
    }

    pub fn remove_session(&self, id: Uuid) -> TakokitResult<bool> {
        let directory = self.session_dir(id);
        if !directory.exists() {
            return Ok(false);
        }
        std::fs::remove_dir_all(directory).map_err(storage_error)?;
        if self.active_session()? == Some(id) {
            let _ = std::fs::remove_file(self.root.join("active-session"));
        }
        Ok(true)
    }

    fn summary_path(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join("session.json")
    }

    fn read_summary(&self, id: Uuid) -> TakokitResult<SessionSummary> {
        let path = self.summary_path(id);
        let source = std::fs::read_to_string(&path).map_err(|error| {
            TakokitError::Storage(format!(
                "could not read session {id} at {}: {error}",
                path.display()
            ))
        })?;
        serde_json::from_str(&source).map_err(storage_error)
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
    std::fs::write(&temporary, bytes).map_err(storage_error)?;
    replace_file_platform(path, &temporary)
}

#[cfg(not(windows))]
fn replace_file_platform(path: &Path, temporary: &Path) -> TakokitResult<()> {
    std::fs::rename(temporary, path).map_err(storage_error)
}

#[cfg(windows)]
fn replace_file_platform(path: &Path, temporary: &Path) -> TakokitResult<()> {
    if !path.exists() {
        return std::fs::rename(temporary, path).map_err(storage_error);
    }
    let backup = path.with_extension(format!("bak-{}", Uuid::new_v4()));
    std::fs::rename(path, &backup).map_err(storage_error)?;
    match std::fs::rename(temporary, path) {
        Ok(()) => {
            let _ = std::fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::rename(backup, path);
            let _ = std::fs::remove_file(temporary);
            Err(storage_error(error))
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
}
