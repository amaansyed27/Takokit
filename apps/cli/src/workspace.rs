use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::path::PathBuf;
use takokit_core::{
    NewSessionEvent, SessionEventState, SessionRecord, SessionTask, SpeechRequest, SpeechResponse,
    TranscriptionRequest, TranscriptionResponse,
};
use takokit_store::{
    load_persisted_workspace, persist_workspace, resolve_workspace, LocalStore, WorkspaceStore,
    WorkspaceSurface,
};
use uuid::Uuid;

pub(crate) const WORKSPACE_ENV: &str = "TAKOKIT_WORKSPACE";
pub(crate) const SESSION_ENV: &str = "TAKOKIT_SESSION_ID";

#[derive(Debug, Clone)]
pub(crate) struct CliWorkspace {
    pub(crate) store: WorkspaceStore,
    pub(crate) session: Option<SessionRecord>,
}

impl CliWorkspace {
    pub(crate) fn resolve(
        workspace: Option<PathBuf>,
        session_id: Option<Uuid>,
        interactive_launch: bool,
        title: &str,
    ) -> anyhow::Result<Self> {
        let surface = match title {
            "Takokit GUI" => WorkspaceSurface::Gui,
            "Takokit TUI" => WorkspaceSurface::Tui,
            _ => WorkspaceSurface::Cli,
        };
        let global_root = LocalStore::default_root();
        let environment = std::env::var_os(WORKSPACE_ENV).map(PathBuf::from);
        let explicit = workspace.or(environment);
        let persisted = if matches!(surface, WorkspaceSurface::Gui | WorkspaceSurface::Tui) {
            load_persisted_workspace(&global_root)?
        } else {
            None
        };
        let current = std::env::current_dir().ok();
        let resolved = resolve_workspace(explicit.clone(), persisted, current, surface)?;
        if matches!(surface, WorkspaceSurface::Gui | WorkspaceSurface::Tui)
            && (explicit.is_some() || resolved.source != takokit_store::WorkspaceSource::CurrentDirectory)
        {
            persist_workspace(&global_root, &resolved.root)?;
        }
        let store = WorkspaceStore::new(resolved.root);

        let session = if let Some(id) = session_id {
            Some(store.open_session(Some(id), Some(title))?)
        } else if interactive_launch {
            None
        } else {
            let active = store.active_session()?;
            match active {
                Some(id) if store.session_dir(id).join("session.json").is_file() => {
                    Some(store.open_session(Some(id), Some(title))?)
                }
                _ => Some(store.create_session(Some(title))?),
            }
        };
        let workspace = Self { store, session };
        workspace.export_environment();
        Ok(workspace)
    }

    pub(crate) fn session_id(&self) -> Uuid {
        self.session
            .as_ref()
            .expect("workspace operation requires an active session")
            .summary
            .id
    }

    pub(crate) fn active_session_id(&self) -> Option<Uuid> {
        self.session.as_ref().map(|session| session.summary.id)
    }

    pub(crate) fn outputs_dir(&self) -> Option<PathBuf> {
        self.active_session_id()
            .map(|id| self.store.session_outputs_dir(id))
    }

    pub(crate) fn export_environment(&self) {
        std::env::set_var(WORKSPACE_ENV, self.store.workspace_root());
        if let Some(id) = self.active_session_id() {
            std::env::set_var(SESSION_ENV, id.to_string());
        } else {
            std::env::remove_var(SESSION_ENV);
        }
    }

    pub(crate) fn gui_query(&self) -> String {
        let mut query = format!(
            "workspace={}",
            utf8_percent_encode(
                &self.store.workspace_root().to_string_lossy(),
                NON_ALPHANUMERIC
            )
        );
        if let Some(id) = self.active_session_id() {
            query.push_str("&session=");
            query.push_str(&id.to_string());
        }
        query
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
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let _ = self.store.append_event(
            session_id,
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
    let explicit = workspace.or_else(|| std::env::var_os(WORKSPACE_ENV).map(PathBuf::from));
    let resolved = resolve_workspace(
        explicit,
        None,
        std::env::current_dir().ok(),
        WorkspaceSurface::Cli,
    )?;
    Ok(WorkspaceStore::new(resolved.root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_query_contains_workspace_without_forcing_a_session() {
        let root = std::env::temp_dir().join(format!("takokit-cli-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let context = CliWorkspace::resolve(Some(root.clone()), None, true, "Takokit GUI").unwrap();
        let query = context.gui_query();
        assert!(query.contains("workspace="));
        assert!(!query.contains("session="));
        assert!(!root.join(".tako").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn stale_implicit_active_session_is_replaced_for_real_operations() {
        let root = std::env::temp_dir().join(format!("takokit-cli-stale-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let store = WorkspaceStore::new(&root);
        let stale = Uuid::new_v4();
        store.set_active_session(stale).unwrap();

        let context = CliWorkspace::resolve(Some(root.clone()), None, false, "recovered").unwrap();

        assert_ne!(context.session_id(), stale);
        assert!(context
            .store
            .session_dir(context.session_id())
            .join("session.json")
            .is_file());
        assert_eq!(
            context.store.active_session().unwrap(),
            Some(context.session_id())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn explicit_missing_session_still_fails() {
        let root = std::env::temp_dir().join(format!("takokit-cli-explicit-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let missing = Uuid::new_v4();

        let result = CliWorkspace::resolve(Some(root.clone()), Some(missing), false, "explicit");

        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
