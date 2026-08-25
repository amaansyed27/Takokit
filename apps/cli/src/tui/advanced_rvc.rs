use std::path::Path;

use takokit_core::RvcVoiceProject;
use takokit_models::RvcVoiceService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedRvcField {
    Project,
    Name,
    Path,
    Index,
    Preset,
    Consent,
    Action,
}

impl AdvancedRvcField {
    pub fn next(self) -> Self {
        match self {
            Self::Project => Self::Name,
            Self::Name => Self::Path,
            Self::Path => Self::Index,
            Self::Index => Self::Preset,
            Self::Preset => Self::Consent,
            Self::Consent => Self::Action,
            Self::Action => Self::Project,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Project => Self::Action,
            Self::Name => Self::Project,
            Self::Path => Self::Name,
            Self::Index => Self::Path,
            Self::Preset => Self::Index,
            Self::Consent => Self::Preset,
            Self::Action => Self::Consent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedRvcAction {
    NewVoice,
    ImportExisting,
    AddSample,
    Inspect,
    Prepare,
    Preflight,
    Train,
    Status,
    Logs,
    Cancel,
    Recover,
    Checkpoints,
    Indexes,
    ActivateCheckpoint,
    TestVoice,
    UseInConvert,
}

pub const ADVANCED_RVC_ACTIONS: [AdvancedRvcAction; 16] = [
    AdvancedRvcAction::NewVoice,
    AdvancedRvcAction::ImportExisting,
    AdvancedRvcAction::AddSample,
    AdvancedRvcAction::Inspect,
    AdvancedRvcAction::Prepare,
    AdvancedRvcAction::Preflight,
    AdvancedRvcAction::Train,
    AdvancedRvcAction::Status,
    AdvancedRvcAction::Logs,
    AdvancedRvcAction::Cancel,
    AdvancedRvcAction::Recover,
    AdvancedRvcAction::Checkpoints,
    AdvancedRvcAction::Indexes,
    AdvancedRvcAction::ActivateCheckpoint,
    AdvancedRvcAction::TestVoice,
    AdvancedRvcAction::UseInConvert,
];

impl AdvancedRvcAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::NewVoice => "New Voice",
            Self::ImportExisting => "Import Existing",
            Self::AddSample => "Add Sample",
            Self::Inspect => "Inspect Dataset",
            Self::Prepare => "Prepare Dataset",
            Self::Preflight => "Hardware Preflight",
            Self::Train => "Start Training",
            Self::Status => "Training Status",
            Self::Logs => "Training Logs",
            Self::Cancel => "Cancel Training",
            Self::Recover => "Recover Training",
            Self::Checkpoints => "Checkpoints",
            Self::Indexes => "Indexes",
            Self::ActivateCheckpoint => "Activate Checkpoint",
            Self::TestVoice => "Test Voice",
            Self::UseInConvert => "Use in Convert",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::NewVoice => "Name + consent → create a persistent managed project",
            Self::ImportExisting => "Path=.pth or .takovoice; optional Index=.index; Name + consent",
            Self::AddSample => "Path=audio file; Takokit copies it into managed voice storage",
            Self::Inspect => "Inspect durations, format facts, duplicates, and objective warnings",
            Self::Prepare => "Run preprocessing, RMVPE F0, and feature extraction",
            Self::Preflight => "Check selected preset against CPU/GPU/RAM/disk",
            Self::Train => "Start the persistent managed RVC training worker",
            Self::Status => "Read current persistent job state",
            Self::Logs => "Open the persisted preparation/training log",
            Self::Cancel => "Request clean cancellation of the active managed job",
            Self::Recover => "Recover a genuinely recoverable failed/cancelled/stale job",
            Self::Checkpoints => "List validated checkpoints and IDs",
            Self::Indexes => "List built/imported index artifacts and IDs",
            Self::ActivateCheckpoint => "Path=checkpoint UUID; Index=optional index UUID",
            Self::TestVoice => "Path=source speech audio; run the normal RVC converter",
            Self::UseInConvert => "Open Convert with this managed voice already selected",
        }
    }

    pub fn uses_audio_picker(self) -> bool {
        matches!(self, Self::AddSample | Self::TestVoice)
    }
}

pub const RVC_PRESETS: [&str; 4] = ["quick", "balanced", "high-quality", "custom"];

#[derive(Debug, Clone)]
pub struct AdvancedRvcState {
    pub projects: Vec<RvcVoiceProject>,
    pub project_index: usize,
    pub field: AdvancedRvcField,
    pub name: String,
    pub name_cursor: usize,
    pub path: String,
    pub path_cursor: usize,
    pub index: String,
    pub index_cursor: usize,
    pub preset_index: usize,
    pub consent: bool,
    pub action_index: usize,
}

impl AdvancedRvcState {
    pub fn new(storage_root: &Path) -> anyhow::Result<Self> {
        let projects = RvcVoiceService::new(storage_root).list()?;
        Ok(Self {
            projects,
            project_index: 0,
            field: AdvancedRvcField::Action,
            name: String::new(),
            name_cursor: 0,
            path: String::new(),
            path_cursor: 0,
            index: String::new(),
            index_cursor: 0,
            preset_index: 1,
            consent: false,
            action_index: 0,
        })
    }

    pub fn reload(&mut self, storage_root: &Path) -> anyhow::Result<()> {
        let selected = self.selected_project().map(|project| project.id);
        self.projects = RvcVoiceService::new(storage_root).list()?;
        self.project_index = selected
            .and_then(|id| self.projects.iter().position(|project| project.id == id))
            .unwrap_or(0);
        Ok(())
    }

    pub fn selected_project(&self) -> Option<&RvcVoiceProject> {
        self.projects.get(self.project_index)
    }

    pub fn selected_action(&self) -> AdvancedRvcAction {
        ADVANCED_RVC_ACTIONS[self.action_index.min(ADVANCED_RVC_ACTIONS.len() - 1)]
    }

    pub fn preset(&self) -> &'static str {
        RVC_PRESETS[self.preset_index.min(RVC_PRESETS.len() - 1)]
    }

    pub fn clear_paths(&mut self) {
        self.path.clear();
        self.path_cursor = 0;
        self.index.clear();
        self.index_cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_workflow_exposes_required_lifecycle_actions() {
        let labels = ADVANCED_RVC_ACTIONS
            .iter()
            .map(|action| action.label())
            .collect::<Vec<_>>();
        for required in [
            "New Voice",
            "Import Existing",
            "Add Sample",
            "Inspect Dataset",
            "Prepare Dataset",
            "Hardware Preflight",
            "Start Training",
            "Training Logs",
            "Cancel Training",
            "Checkpoints",
            "Test Voice",
            "Use in Convert",
        ] {
            assert!(labels.contains(&required));
        }
    }

    #[test]
    fn preset_order_keeps_backend_product_names() {
        assert_eq!(RVC_PRESETS, ["quick", "balanced", "high-quality", "custom"]);
    }
}
