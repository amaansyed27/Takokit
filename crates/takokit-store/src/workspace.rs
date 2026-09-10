use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::{
    NewSessionEvent, SessionEvent, SessionRecord, SessionSummary, TakokitError, TakokitResult,
};
use uuid::Uuid;

mod recovery;
use recovery::*;

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
        read_summary_recovering(id, &self.summary_path(id))
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
mod tests;
