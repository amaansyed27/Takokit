use std::path::PathBuf;

use takokit_core::{RuntimeConfig, SessionSummary};
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::{
    persist_workspace, resolve_workspace, LocalStore, WorkspaceStore, WorkspaceSurface,
};
use uuid::Uuid;

use crate::workspace::{CliWorkspace, SESSION_ENV, WORKSPACE_ENV};

use super::{
    catalog::{
        capability_indexes, find_capability_index, find_model_index, find_runner_index,
        load_runtime_rows, system_rows, ModelRow, RunnerRow, SystemAction, SystemRow,
    },
    convert::ConvertState,
};

pub const HOME_ACTIONS: [(&str, &str); 8] = [
    ("Speak", "Generate speech with an installed TTS model"),
    ("Transcribe", "Turn a local audio file into text"),
    ("Clone voice", "Create a consented local voice profile"),
    (
        "Convert voice",
        "Run conversion and review execution separately from quality",
    ),
    ("Manage", "Inspect models, runners, and the local service"),
    ("Sessions", "Open prior work or start a clean session"),
    ("Workspace", "View or change the project-specific .tako location"),
    ("Activity", "Read the complete result from the latest task"),
];

pub const MANAGE_ACTIONS: [(&str, &str); 3] = [
    ("Installed models", "Use, repair, or remove local models"),
    ("Runners", "Inspect and repair shared execution runtimes"),
    ("System", "Daemon status, diagnostics, logs, and GUI"),
];

#[derive(Debug, Clone, PartialEq)]
pub enum TuiAction {
    Quit,
    Refresh,
    PullModel(String),
    RemoveModel(String),
    Speak {
        model: String,
        voice: String,
        text: String,
    },
    Transcribe {
        model: String,
        audio: String,
    },
    CloneVoice {
        model: String,
        name: String,
        sample: String,
    },
    ConvertVoice {
        model: String,
        source: String,
        target: String,
        f0_method: String,
        pitch_shift: i32,
        index_rate: f32,
        rms_mix_rate: f32,
        protect: f32,
        filter_radius: u32,
    },
    PullRunner(String),
    InstallRunner(String),
    RemoveRunner(String),
    DoctorRunner(String),
    RunSystem(SystemAction),
    OpenSession(Uuid),
    NewSession,
    ChangeWorkspace(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiScreen {
    Home,
    Speak,
    Transcribe,
    Clone,
    Convert,
    Manage,
    Models,
    Runners,
    System,
    Sessions,
    Workspace,
    Activity,
}

impl TuiScreen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Speak => "Speak",
            Self::Transcribe => "Transcribe",
            Self::Clone => "Clone voice",
            Self::Convert => "Convert voice",
            Self::Manage => "Manage",
            Self::Models => "Installed models",
            Self::Runners => "Runners",
            Self::System => "System",
            Self::Sessions => "Sessions",
            Self::Workspace => "Workspace",
            Self::Activity => "Activity",
        }
    }

    pub fn parent(self) -> Self {
        match self {
            Self::Models | Self::Runners | Self::System => Self::Manage,
            Self::Home => Self::Home,
            _ => Self::Home,
        }
    }

    pub fn accepts_text(self) -> bool {
        matches!(
            self,
            Self::Speak | Self::Transcribe | Self::Clone | Self::Convert | Self::Workspace
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakField {
    Model,
    Voice,
    Text,
    Submit,
}

impl SpeakField {
    pub fn next(self) -> Self {
        match self {
            Self::Model => Self::Voice,
            Self::Voice => Self::Text,
            Self::Text => Self::Submit,
            Self::Submit => Self::Model,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Model => Self::Submit,
            Self::Voice => Self::Model,
            Self::Text => Self::Voice,
            Self::Submit => Self::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscribeField {
    Model,
    Audio,
    Submit,
}

impl TranscribeField {
    pub fn next(self) -> Self {
        match self {
            Self::Model => Self::Audio,
            Self::Audio => Self::Submit,
            Self::Submit => Self::Model,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Model => Self::Submit,
            Self::Audio => Self::Model,
            Self::Submit => Self::Audio,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceField {
    Path,
    Apply,
}

impl WorkspaceField {
    pub fn next(self) -> Self {
        match self {
            Self::Path => Self::Apply,
            Self::Apply => Self::Path,
        }
    }

    pub fn previous(self) -> Self {
        self.next()
    }
}

pub struct App {
    pub screen: TuiScreen,
    pub home_index: usize,
    pub manage_index: usize,
    pub models: Vec<ModelRow>,
    pub runners: Vec<RunnerRow>,
    pub system: Vec<SystemRow>,
    pub sessions: Vec<SessionSummary>,
    pub model_index: usize,
    pub runner_index: usize,
    pub system_index: usize,
    pub session_index: usize,
    pub tts_models: Vec<usize>,
    pub stt_models: Vec<usize>,
    pub speak_model_index: usize,
    pub transcribe_model_index: usize,
    pub speak_field: SpeakField,
    pub transcribe_field: TranscribeField,
    pub speak_voice: String,
    pub speak_voice_cursor: usize,
    pub speak_text: String,
    pub speak_text_cursor: usize,
    pub transcribe_audio: String,
    pub transcribe_audio_cursor: usize,
    pub clone_state: super::clone::CloneState,
    pub convert_state: ConvertState,
    pub workspace_field: WorkspaceField,
    pub workspace_input: String,
    pub workspace_input_cursor: usize,
    pub storage_root: String,
    pub workspace_root: String,
    pub server: String,
    pub status: String,
    pub running_label: Option<String>,
    pub last_label: Option<String>,
    pub output_scroll: u16,
    pub tick: u64,
    pub show_help: bool,
    pub confirmation_message: Option<String>,
    pub pending_confirmation: Option<TuiAction>,
    workspace_store: WorkspaceStore,
    active_session: Option<Uuid>,
}

impl App {
    pub fn new(
        config: &RuntimeConfig,
        store: &LocalStore,
        package_registry: &PackageRegistry,
        installed_registry: &InstalledRegistry,
        workspace: &CliWorkspace,
    ) -> anyhow::Result<Self> {
        let (models, runners) = load_runtime_rows(package_registry, installed_registry)?;
        let (tts_models, stt_models) = capability_indexes(&models);
        let clone_state = super::clone::CloneState::new(&models);
        let convert_state = ConvertState::new(&models);
        let sessions = workspace.store.list_sessions(None)?;
        let active_session = workspace
            .active_session_id()
            .or(workspace.store.active_session()?);
        let session_index = session_position(&sessions, active_session);
        let workspace_root = workspace.store.workspace_root().display().to_string();
        let status = match active_session {
            Some(id) => format!(
                "Workspace {workspace_root}. Session {id} is active; outputs are written under its .tako history."
            ),
            None => format!(
                "Workspace {workspace_root}. No session exists yet; .tako will be created by the first workflow or New Session."
            ),
        };
        let workspace_input_cursor = workspace_root.chars().count();

        Ok(Self {
            screen: TuiScreen::Home,
            home_index: 0,
            manage_index: 0,
            speak_model_index: find_capability_index(&models, &tts_models, None, "kokoro"),
            transcribe_model_index: find_capability_index(
                &models,
                &stt_models,
                None,
                "whisper-tiny",
            ),
            models,
            runners,
            system: system_rows(),
            sessions,
            model_index: 0,
            runner_index: 0,
            system_index: 0,
            session_index,
            tts_models,
            stt_models,
            speak_field: SpeakField::Text,
            transcribe_field: TranscribeField::Audio,
            speak_voice: "default".to_string(),
            speak_voice_cursor: 7,
            speak_text: String::new(),
            speak_text_cursor: 0,
            transcribe_audio: String::new(),
            transcribe_audio_cursor: 0,
            clone_state,
            convert_state,
            workspace_field: WorkspaceField::Path,
            workspace_input: workspace_root.clone(),
            workspace_input_cursor,
            storage_root: store.root().display().to_string(),
            workspace_root,
            server: config.local_base_url(),
            status,
            running_label: None,
            last_label: None,
            output_scroll: 0,
            tick: 0,
            show_help: false,
            confirmation_message: None,
            pending_confirmation: None,
            workspace_store: workspace.store.clone(),
            active_session,
        })
    }

    pub fn reload(
        &mut self,
        config: &RuntimeConfig,
        store: &LocalStore,
        package_registry: &PackageRegistry,
        installed_registry: &InstalledRegistry,
    ) -> anyhow::Result<()> {
        let selected_model = self.selected_model().map(|model| model.id.clone());
        let selected_runner = self.selected_runner().map(|runner| runner.id.clone());
        let speak_model = self.selected_speak_model().map(|model| model.id.clone());
        let transcribe_model = self
            .selected_transcribe_model()
            .map(|model| model.id.clone());
        let (models, runners) = load_runtime_rows(package_registry, installed_registry)?;
        let (tts_models, stt_models) = capability_indexes(&models);

        self.models = models;
        self.runners = runners;
        self.tts_models = tts_models;
        self.stt_models = stt_models;
        self.model_index = find_model_index(&self.models, selected_model.as_deref());
        self.runner_index = find_runner_index(&self.runners, selected_runner.as_deref());
        self.speak_model_index = find_capability_index(
            &self.models,
            &self.tts_models,
            speak_model.as_deref(),
            "kokoro",
        );
        self.transcribe_model_index = find_capability_index(
            &self.models,
            &self.stt_models,
            transcribe_model.as_deref(),
            "whisper-tiny",
        );
        self.clone_state.reload_models(&self.models);
        self.convert_state.reload_models(&self.models);
        self.storage_root = store.root().display().to_string();
        self.server = config.local_base_url();
        self.reload_sessions()?;
        Ok(())
    }

    pub fn reload_sessions(&mut self) -> anyhow::Result<()> {
        self.sessions = self.workspace_store.list_sessions(None)?;
        if let Some(persisted) = self.workspace_store.active_session()? {
            if self
                .sessions
                .iter()
                .any(|session| session.id == persisted)
            {
                self.active_session = Some(persisted);
            }
        }
        self.session_index = session_position(&self.sessions, self.active_session);
        Ok(())
    }

    pub fn activate_session(&mut self, id: Uuid) -> anyhow::Result<()> {
        let session = self.workspace_store.read_session(id)?;
        self.workspace_store.set_active_session(id)?;
        self.active_session = Some(id);
        std::env::set_var(WORKSPACE_ENV, self.workspace_store.workspace_root());
        std::env::set_var(SESSION_ENV, id.to_string());
        self.reload_sessions()?;
        self.set_status(format!(
            "Opened {}. New outputs will be saved in {}.",
            session.summary.title,
            self.workspace_store.session_outputs_dir(id).display()
        ));
        Ok(())
    }

    pub fn create_session(&mut self) -> anyhow::Result<()> {
        let session = self.workspace_store.create_session(None)?;
        self.activate_session(session.summary.id)
    }

    pub fn switch_workspace(&mut self, path: &str) -> anyhow::Result<()> {
        let value = path.trim();
        if value.is_empty() {
            anyhow::bail!("workspace path cannot be empty");
        }
        let resolved = resolve_workspace(
            Some(PathBuf::from(value)),
            None,
            Some(self.workspace_store.workspace_root().to_path_buf()),
            WorkspaceSurface::Tui,
        )?;
        persist_workspace(&LocalStore::default_root(), &resolved.root)?;
        let store = WorkspaceStore::new(resolved.root);
        let sessions = store.list_sessions(None)?;
        let active_session = store
            .active_session()?
            .filter(|id| store.session_dir(*id).join("session.json").is_file());
        self.workspace_store = store;
        self.sessions = sessions;
        self.active_session = active_session;
        self.session_index = session_position(&self.sessions, self.active_session);
        self.workspace_root = self.workspace_store.workspace_root().display().to_string();
        self.workspace_input = self.workspace_root.clone();
        self.workspace_input_cursor = self.workspace_input.chars().count();
        std::env::set_var(WORKSPACE_ENV, self.workspace_store.workspace_root());
        if let Some(id) = self.active_session {
            std::env::set_var(SESSION_ENV, id.to_string());
        } else {
            std::env::remove_var(SESSION_ENV);
        }
        self.set_status(format!(
            "Workspace changed to {}. Installed models remain global. No .tako data was created by this switch.",
            self.workspace_root
        ));
        Ok(())
    }

    pub fn request_confirmation(&mut self, message: impl Into<String>, action: TuiAction) {
        self.confirmation_message = Some(message.into());
        self.pending_confirmation = Some(action);
    }

    pub fn cancel_confirmation(&mut self) {
        self.confirmation_message = None;
        self.pending_confirmation = None;
    }

    pub fn confirm_pending(&mut self) -> Option<TuiAction> {
        self.confirmation_message = None;
        self.pending_confirmation.take()
    }

    pub fn workspace_args(&self) -> Vec<String> {
        let mut arguments = vec![
            "--workspace".to_string(),
            self.workspace_store.workspace_root().display().to_string(),
        ];
        if let Some(id) = self.active_session {
            arguments.extend(["--session".to_string(), id.to_string()]);
        }
        arguments
    }

    pub fn active_session(&self) -> Option<Uuid> {
        self.active_session
    }

    pub fn selected_model(&self) -> Option<&ModelRow> {
        self.models.get(self.model_index)
    }

    pub fn selected_runner(&self) -> Option<&RunnerRow> {
        self.runners.get(self.runner_index)
    }

    pub fn selected_system(&self) -> Option<&SystemRow> {
        self.system.get(self.system_index)
    }

    pub fn selected_session(&self) -> Option<&SessionSummary> {
        self.sessions.get(self.session_index)
    }

    pub fn selected_speak_model(&self) -> Option<&ModelRow> {
        self.tts_models
            .get(self.speak_model_index)
            .and_then(|index| self.models.get(*index))
    }

    pub fn selected_clone_model(&self) -> Option<&ModelRow> {
        self.clone_state
            .model_indexes
            .get(self.clone_state.model_index)
            .and_then(|index| self.models.get(*index))
    }

    pub fn selected_convert_model(&self) -> Option<&ModelRow> {
        self.convert_state
            .model_indexes
            .get(self.convert_state.model_index)
            .and_then(|index| self.models.get(*index))
    }

    pub fn selected_transcribe_model(&self) -> Option<&ModelRow> {
        self.stt_models
            .get(self.transcribe_model_index)
            .and_then(|index| self.models.get(*index))
    }

    pub fn set_speak_model(&mut self, id: &str) {
        self.speak_model_index =
            find_capability_index(&self.models, &self.tts_models, Some(id), id);
    }

    pub fn set_transcribe_model(&mut self, id: &str) {
        self.transcribe_model_index =
            find_capability_index(&self.models, &self.stt_models, Some(id), id);
    }

    pub fn set_convert_model(&mut self, id: &str) {
        self.convert_state.model_index = self
            .convert_state
            .model_indexes
            .iter()
            .position(|index| self.models[*index].id == id)
            .unwrap_or(0);
    }

    pub fn set_status(&mut self, value: impl Into<String>) {
        self.status = value.into();
        self.output_scroll = 0;
    }
}

fn session_position(sessions: &[SessionSummary], active: Option<Uuid>) -> usize {
    active
        .and_then(|id| sessions.iter().position(|session| session.id == id))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_screens_have_obvious_parents() {
        assert_eq!(TuiScreen::Models.parent(), TuiScreen::Manage);
        assert_eq!(TuiScreen::Convert.parent(), TuiScreen::Home);
        assert_eq!(TuiScreen::Workspace.parent(), TuiScreen::Home);
        assert_eq!(TuiScreen::Home.parent(), TuiScreen::Home);
    }

    #[test]
    fn home_starts_with_primary_tasks_and_has_workspace_access() {
        assert_eq!(HOME_ACTIONS[0].0, "Speak");
        assert_eq!(HOME_ACTIONS[1].0, "Transcribe");
        assert_eq!(HOME_ACTIONS[2].0, "Clone voice");
        assert_eq!(HOME_ACTIONS[3].0, "Convert voice");
        assert!(HOME_ACTIONS.iter().any(|item| item.0 == "Workspace"));
    }

    #[test]
    fn session_position_handles_an_uninitialized_workspace() {
        assert_eq!(session_position(&[], None), 0);
    }

    #[test]
    fn workspace_field_navigation_is_bounded() {
        assert_eq!(WorkspaceField::Path.next(), WorkspaceField::Apply);
        assert_eq!(WorkspaceField::Apply.next(), WorkspaceField::Path);
    }
}
