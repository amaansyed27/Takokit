use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::path::{Path, PathBuf};
use takokit_core::{
    NewSessionEvent, SessionEventState, SessionRecord, SessionTask, SpeechRequest, SpeechResponse,
    TranscriptionRequest, TranscriptionResponse,
};
use takokit_store::WorkspaceStore;
use uuid::Uuid;

pub(crate) const WORKSPACE_ENV: &str = "TAKOKIT_WORKSPACE";
pub(crate) const SESSION_ENV: &str = "TAKOKIT_SESSION_ID";

#[derive(Debug, Clone)]
pub(crate) struct CliWorkspace {
    pub(crate) store: WorkspaceStore,
    pub(crate) session: SessionRecord,
}

impl CliWorkspace {
    pub(crate) fn resolve(
        workspace: Option<PathBuf>,
        session_id: Option<Uuid>,
        create_new: bool,
        title: &str,
    ) -> anyhow::Result<Self> {
        let store = resolve_store(workspace)?;
        let selected = if session_id.is_some() {
            session_id
        } else if create_new {
            None
        } else {
            store.active_session()?
        };
        let session = store.open_session(selected, Some(title))?;
        let workspace = Self { store, session };
        workspace.export_environment();
        Ok(workspace)
    }

    pub(crate) fn session_id(&self) -> Uuid {
        self.session.summary.id
    }

    pub(crate) fn outputs_dir(&self) -> PathBuf {
        self.store.session_outputs_dir(self.session_id())
    }

    pub(crate) fn export_environment(&self) {
        std::env::set_var(WORKSPACE_ENV, self.store.workspace_root());
        std::env::set_var(SESSION_ENV, self.session_id().to_string());
    }

    pub(crate) fn gui_query(&self) -> String {
        format!(
            "workspace={}&session={}",
            utf8_percent_encode(
                &self.store.workspace_root().to_string_lossy(),
                NON_ALPHANUMERIC
            ),
            self.session_id()
        )
    }

    pub(crate) fn record_speech(
        &self,
        request: &SpeechRequest,
        response: &SpeechResponse,
    ) -> anyhow::Result<()> {
        self.store.append_event(
            self.session_id(),
            NewSessionEvent {
                task: SessionTask::TextToSpeech,
                state: SessionEventState::Completed,
                model: Some(request.model.clone()),
                input: Some(request.input.clone()),
                source_path: None,
                output_path: Some(response.output_path.clone()),
                text: None,
                message: Some(format!(
                    "Generated {} bytes using {}",
                    response.bytes, response.engine
                )),
            },
        )?;
        Ok(())
    }

    pub(crate) fn record_transcription(
        &self,
        request: &TranscriptionRequest,
        response: &TranscriptionResponse,
    ) -> anyhow::Result<PathBuf> {
        let output = self.store.write_text_output(
            self.session_id(),
            &format!("transcript-{}.txt", response.id),
            &response.text,
        )?;
        self.store.append_event(
            self.session_id(),
            NewSessionEvent {
                task: SessionTask::SpeechToText,
                state: SessionEventState::Completed,
                model: Some(response.model.clone()),
                input: None,
                source_path: Some(request.file_path.clone()),
                output_path: Some(output.clone()),
                text: Some(response.text.clone()),
                message: Some("Transcript saved in the project session.".to_string()),
            },
        )?;
        Ok(output)
    }

    pub(crate) fn record_failure(
        &self,
        task: SessionTask,
        model: Option<String>,
        source_path: Option<PathBuf>,
        input: Option<String>,
        error: &dyn std::fmt::Display,
    ) {
        let _ = self.store.append_event(
            self.session_id(),
            NewSessionEvent {
                task,
                state: SessionEventState::Failed,
                model,
                input,
                source_path,
                output_path: None,
                text: None,
                message: Some(error.to_string()),
            },
        );
    }
}

pub(crate) fn resolve_store(workspace: Option<PathBuf>) -> anyhow::Result<WorkspaceStore> {
    let store = WorkspaceStore::new(absolute_workspace(workspace)?);
    store.ensure_layout()?;
    Ok(store)
}

fn absolute_workspace(workspace: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = workspace
        .or_else(|| std::env::var_os(WORKSPACE_ENV).map(PathBuf::from))
        .unwrap_or(std::env::current_dir()?);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

pub(crate) fn filename(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("output")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_query_contains_workspace_and_session() {
        let root = std::env::temp_dir().join(format!("takokit-cli-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let context = CliWorkspace::resolve(Some(root.clone()), None, true, "test").unwrap();
        let query = context.gui_query();
        assert!(query.contains("workspace="));
        assert!(query.contains(&context.session_id().to_string()));
        let _ = std::fs::remove_dir_all(root);
    }
}
