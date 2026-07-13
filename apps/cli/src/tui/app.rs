use takokit_core::{RuntimeConfig, SessionSummary};
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::{LocalStore, WorkspaceStore};
use uuid::Uuid;

use crate::workspace::{CliWorkspace, SESSION_ENV, WORKSPACE_ENV};

use super::catalog::{
    capability_indexes, find_capability_index, find_model_index, find_runner_index,
    load_runtime_rows, system_rows, ModelRow, RunnerRow, SystemAction, SystemRow,
};

#[derive(Debug, Clone, PartialEq, Eq)]
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
    PullRunner(String),
    InstallRunner(String),
    RemoveRunner(String),
    DoctorRunner(String),
    RunSystem(SystemAction),
    OpenSession(Uuid),
    NewSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiTab {
    Models,
    Speak,
    Transcribe,
    Sessions,
    Runners,
    System,
}

impl TuiTab {
    pub const ALL: [Self; 6] = [
        Self::Models,
        Self::Speak,
        Self::Transcribe,
        Self::Sessions,
        Self::Runners,
        Self::System,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Models => "Models",
            Self::Speak => "Speak",
            Self::Transcribe => "Transcribe",
            Self::Sessions => "Sessions",
            Self::Runners => "Runners",
            Self::System => "System",
        }
    }

    pub(super) fn next(self) -> Self {
        let index = Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub(super) fn previous(self) -> Self {
        let index = Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakField {
    Model,
    Voice,
    Text,
    Primary,
}

impl SpeakField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Model => Self::Voice,
            Self::Voice => Self::Text,
            Self::Text => Self::Primary,
            Self::Primary => Self::Model,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Model => Self::Primary,
            Self::Voice => Self::Model,
            Self::Text => Self::Voice,
            Self::Primary => Self::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscribeField {
    Model,
    Audio,
    Primary,
}

impl TranscribeField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Model => Self::Audio,
            Self::Audio => Self::Primary,
            Self::Primary => Self::Model,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Model => Self::Primary,
            Self::Audio => Self::Model,
            Self::Primary => Self::Audio,
        }
    }
}

pub struct App {
    pub tab: TuiTab,
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
    pub storage_root: String,
    pub server: String,
    pub status: String,
    pub running_label: Option<String>,
    pub last_label: Option<String>,
    pub output_scroll: u16,
    pub tick: u64,
    pub show_help: bool,
    pub slash_open: bool,
    pub slash_input: String,
    pub slash_cursor: usize,
    workspace_store: WorkspaceStore,
    active_session: Uuid,
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
        let sessions = workspace.store.list_sessions(None)?;
        let active_session = workspace.session_id();
        let session_index = session_position(&sessions, active_session);
        Ok(Self {
            tab: TuiTab::Models,
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
            storage_root: store.root().display().to_string(),
            server: config.local_base_url(),
            status: format!(
                "Session {} is active. Outputs are saved under {}.",
                active_session,
                workspace.outputs_dir().display()
            ),
            running_label: None,
            last_label: None,
            output_scroll: 0,
            tick: 0,
            show_help: false,
            slash_open: false,
            slash_input: String::new(),
            slash_cursor: 0,
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
        self.storage_root = store.root().display().to_string();
        self.server = config.local_base_url();
        self.reload_sessions()?;
        Ok(())
    }

    pub fn reload_sessions(&mut self) -> anyhow::Result<()> {
        self.sessions = self.workspace_store.list_sessions(None)?;
        self.session_index = session_position(&self.sessions, self.active_session);
        Ok(())
    }

    pub fn activate_session(&mut self, id: Uuid) -> anyhow::Result<()> {
        let session = self.workspace_store.read_session(id)?;
        self.workspace_store.set_active_session(id)?;
        self.active_session = id;
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

    pub fn workspace_args(&self) -> Vec<String> {
        vec![
            "--workspace".to_string(),
            self.workspace_store
                .workspace_root()
                .display()
                .to_string(),
            "--session".to_string(),
            self.active_session.to_string(),
        ]
    }

    pub fn active_session(&self) -> Uuid {
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

    pub fn set_status(&mut self, value: impl Into<String>) {
        self.status = value.into();
        self.output_scroll = 0;
    }
}

fn session_position(sessions: &[SessionSummary], active: Uuid) -> usize {
    sessions
        .iter()
        .position(|session| session.id == active)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_cycle_includes_sessions() {
        assert_eq!(TuiTab::Transcribe.next(), TuiTab::Sessions);
        assert_eq!(TuiTab::Sessions.previous(), TuiTab::Transcribe);
    }
}
