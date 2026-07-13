use takokit_core::RuntimeConfig;
use takokit_package::{plan_model, InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiTab {
    Models,
    Speak,
    Transcribe,
    Runners,
    System,
}

impl TuiTab {
    pub const ALL: [Self; 5] = [
        Self::Models,
        Self::Speak,
        Self::Transcribe,
        Self::Runners,
        Self::System,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Models => "Models",
            Self::Speak => "Speak",
            Self::Transcribe => "Transcribe",
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

#[derive(Debug, Clone)]
pub struct ModelRow {
    pub id: String,
    pub title: String,
    pub state: String,
    pub detail: String,
    pub tts: bool,
    pub stt: bool,
    pub executable: bool,
}

#[derive(Debug, Clone)]
pub struct RunnerRow {
    pub id: String,
    pub title: String,
    pub state: String,
    pub detail: String,
    pub installed: bool,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAction {
    Status,
    Doctor,
    StartDaemon,
    StopDaemon,
    RestartDaemon,
    Logs,
    OpenGui,
}

#[derive(Debug, Clone)]
pub struct SystemRow {
    pub title: &'static str,
    pub state: &'static str,
    pub detail: &'static str,
    pub action: SystemAction,
}

pub struct App {
    pub tab: TuiTab,
    pub models: Vec<ModelRow>,
    pub runners: Vec<RunnerRow>,
    pub system: Vec<SystemRow>,
    pub model_index: usize,
    pub runner_index: usize,
    pub system_index: usize,
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
}

impl App {
    pub fn new(
        config: &RuntimeConfig,
        store: &LocalStore,
        package_registry: &PackageRegistry,
        installed_registry: &InstalledRegistry,
    ) -> anyhow::Result<Self> {
        let (models, runners) = load_runtime_rows(package_registry, installed_registry)?;
        let (tts_models, stt_models) = capability_indexes(&models);
        let speak_model_index =
            find_capability_index(&models, &tts_models, None, "kokoro");
        let transcribe_model_index =
            find_capability_index(&models, &stt_models, None, "whisper-tiny");
        Ok(Self {
            tab: TuiTab::Models,
            models,
            runners,
            system: system_rows(),
            model_index: 0,
            runner_index: 0,
            system_index: 0,
            speak_model_index,
            transcribe_model_index,
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
            status: "Ready. Choose a task above; no commands are required.".to_string(),
            running_label: None,
            last_label: None,
            output_scroll: 0,
            tick: 0,
            show_help: false,
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
        Ok(())
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
        self.speak_model_index = find_capability_index(&self.models, &self.tts_models, Some(id), id);
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

fn load_runtime_rows(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<(Vec<ModelRow>, Vec<RunnerRow>)> {
    let models = package_registry
        .models()?
        .into_iter()
        .map(|model| {
            let plan = plan_model(package_registry, installed_registry, &model.id)?;
            let capabilities = [
                model.capabilities.tts.then_some("text to speech"),
                model.capabilities.stt.then_some("speech to text"),
                model.capabilities.voice_cloning.then_some("voice cloning"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
            Ok(ModelRow {
                id: model.id,
                title: model.name,
                state: if plan.executable {
                    "ready".to_string()
                } else {
                    plan.lifecycle_state.to_string()
                },
                detail: format!(
                    "{}\n\nCapability: {}\nFamily: {}\nRunner: {}\nHardware: {}\n\n{}",
                    model.description,
                    if capabilities.is_empty() { "specialized" } else { &capabilities },
                    model.family,
                    plan.required_runner,
                    model.hardware.min_ram.as_deref().unwrap_or("no minimum listed"),
                    if plan.executable {
                        "Ready to use. Press Enter to open the matching task screen.".to_string()
                    } else {
                        format!(
                            "Not ready yet. Press Enter to let Takokit install what is missing.\nMissing: {}",
                            if plan.missing.is_empty() { "setup".to_string() } else { plan.missing.join("; ") }
                        )
                    }
                ),
                tts: model.capabilities.tts,
                stt: model.capabilities.stt,
                executable: plan.executable,
            })
        })
        .collect::<Result<Vec<_>, takokit_package::PackageError>>()?;

    let runners = package_registry
        .runners()?
        .into_iter()
        .map(|runner| {
            let record = installed_registry.installed_runner_record(&runner.id).ok();
            let state = record
                .as_ref()
                .map(|record| record.status.to_string())
                .unwrap_or_else(|| "available".to_string());
            let ready = state == "ready";
            RunnerRow {
                id: runner.id,
                title: runner.name,
                state: state.clone(),
                detail: format!(
                    "{}\n\nVersion: {}\nPlatforms: {}\nModel families: {}\nState: {}\n\n{}",
                    runner.description,
                    runner.version,
                    runner.platforms.join(", "),
                    runner.supported_model_families.join(", "),
                    state,
                    if ready {
                        "Ready. Press Enter to run its diagnostic check."
                    } else if record.is_some() {
                        "The runner contract exists. Press Enter to install its runtime."
                    } else {
                        "Press Enter to add this runner."
                    }
                ),
                installed: record.is_some(),
                ready,
            }
        })
        .collect();
    Ok((models, runners))
}

fn capability_indexes(models: &[ModelRow]) -> (Vec<usize>, Vec<usize>) {
    let tts = models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| model.tts.then_some(index))
        .collect();
    let stt = models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| model.stt.then_some(index))
        .collect();
    (tts, stt)
}

fn find_model_index(models: &[ModelRow], id: Option<&str>) -> usize {
    id.and_then(|id| models.iter().position(|model| model.id == id))
        .unwrap_or(0)
}

fn find_runner_index(runners: &[RunnerRow], id: Option<&str>) -> usize {
    id.and_then(|id| runners.iter().position(|runner| runner.id == id))
        .unwrap_or(0)
}

fn find_capability_index(
    models: &[ModelRow],
    indexes: &[usize],
    selected: Option<&str>,
    preferred: &str,
) -> usize {
    selected
        .and_then(|id| indexes.iter().position(|index| models[*index].id == id))
        .or_else(|| indexes.iter().position(|index| models[*index].id == preferred))
        .unwrap_or(0)
}

fn system_rows() -> Vec<SystemRow> {
    vec![
        SystemRow {
            title: "Runtime status",
            state: "read",
            detail: "Check the daemon, storage, and currently available runtime state.",
            action: SystemAction::Status,
        },
        SystemRow {
            title: "Doctor",
            state: "diagnostics",
            detail: "Run the complete local setup and model readiness check.",
            action: SystemAction::Doctor,
        },
        SystemRow {
            title: "Start daemon",
            state: "service",
            detail: "Start Takokit's managed local API service.",
            action: SystemAction::StartDaemon,
        },
        SystemRow {
            title: "Stop daemon",
            state: "service",
            detail: "Stop the managed local API service.",
            action: SystemAction::StopDaemon,
        },
        SystemRow {
            title: "Restart daemon",
            state: "service",
            detail: "Restart the managed local API service.",
            action: SystemAction::RestartDaemon,
        },
        SystemRow {
            title: "View logs",
            state: "diagnostics",
            detail: "Show the latest daemon log location and output.",
            action: SystemAction::Logs,
        },
        SystemRow {
            title: "Open GUI",
            state: "interface",
            detail: "Open the local browser interface using the same daemon and model state.",
            action: SystemAction::OpenGui,
        },
    ]
}
