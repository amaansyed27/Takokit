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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedRvcAction {
    NewVoice,
    ImportExisting,
    AddSample,
    Inspect,
    Train,
    Status,
    Cancel,
    Recover,
    TestVoice,
    UseInConvert,
}

pub const ADVANCED_RVC_ACTIONS: [AdvancedRvcAction; 10] = [
    AdvancedRvcAction::NewVoice,
    AdvancedRvcAction::ImportExisting,
    AdvancedRvcAction::AddSample,
    AdvancedRvcAction::Inspect,
    AdvancedRvcAction::Train,
    AdvancedRvcAction::Status,
    AdvancedRvcAction::Cancel,
    AdvancedRvcAction::Recover,
    AdvancedRvcAction::TestVoice,
    AdvancedRvcAction::UseInConvert,
];

impl AdvancedRvcAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::NewVoice => "Create trained voice",
            Self::ImportExisting => "Import existing voice",
            Self::AddSample => "Add recording",
            Self::Inspect => "Check recordings",
            Self::Train => "Train voice",
            Self::Status => "Training progress",
            Self::Cancel => "Cancel training",
            Self::Recover => "Continue / recover training",
            Self::TestVoice => "Test voice",
            Self::UseInConvert => "Use in Clone audio",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::NewVoice => "Give the voice a name and confirm you have permission to use it.",
            Self::ImportExisting => {
                "Import a Takokit .takovoice package or, for legacy RVC models, a .pth file."
            }
            Self::AddSample => "Add a clean recording of this speaker. You can add more than one.",
            Self::Inspect => "Check duration, duplicates and recording quality before training.",
            Self::Train => {
                "Takokit checks hardware, prepares the recordings, trains, builds the index and activates the finished voice automatically."
            }
            Self::Status => "Show the real training stage and epoch progress from the managed job.",
            Self::Cancel => "Stop the current managed training job cleanly.",
            Self::Recover => "Resume a cancelled, failed or interrupted training run when possible.",
            Self::TestVoice => "Choose speech from another speaker and convert it to this trained voice.",
            Self::UseInConvert => "Open Clone audio with this trained voice already selected.",
        }
    }

    pub fn uses_audio_picker(self) -> bool {
        matches!(self, Self::AddSample | Self::TestVoice)
    }

    pub fn requires_name(self) -> bool {
        matches!(self, Self::NewVoice | Self::ImportExisting)
    }

    pub fn requires_path(self) -> bool {
        matches!(
            self,
            Self::ImportExisting | Self::AddSample | Self::TestVoice
        )
    }

    pub fn shows_index_input(self) -> bool {
        matches!(self, Self::ImportExisting)
    }

    pub fn shows_training_quality(self) -> bool {
        matches!(self, Self::Train)
    }

    pub fn requires_consent(self) -> bool {
        matches!(self, Self::NewVoice | Self::ImportExisting)
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

    pub fn cycle_field(&self, backward: bool) -> AdvancedRvcField {
        let fields = self.relevant_fields();
        let index = fields
            .iter()
            .position(|candidate| *candidate == self.field)
            .unwrap_or(0);
        let next = if backward {
            if index == 0 {
                fields.len() - 1
            } else {
                index - 1
            }
        } else {
            (index + 1) % fields.len()
        };
        fields[next]
    }

    fn relevant_fields(&self) -> Vec<AdvancedRvcField> {
        let action = self.selected_action();
        let mut fields = vec![AdvancedRvcField::Project, AdvancedRvcField::Action];
        if action.requires_name() {
            fields.push(AdvancedRvcField::Name);
        }
        if action.requires_path() {
            fields.push(AdvancedRvcField::Path);
        }
        if action.shows_index_input() {
            fields.push(AdvancedRvcField::Index);
        }
        if action.shows_training_quality() {
            fields.push(AdvancedRvcField::Preset);
        }
        if action.requires_consent() {
            fields.push(AdvancedRvcField::Consent);
        }
        fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_tui_exposes_user_level_trained_voice_workflow() {
        let labels = ADVANCED_RVC_ACTIONS
            .iter()
            .map(|action| action.label())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                "Create trained voice",
                "Import existing voice",
                "Add recording",
                "Check recordings",
                "Train voice",
                "Training progress",
                "Cancel training",
                "Continue / recover training",
                "Test voice",
                "Use in Clone audio",
            ]
        );
    }

    #[test]
    fn preset_order_keeps_backend_product_names() {
        assert_eq!(RVC_PRESETS, ["quick", "balanced", "high-quality", "custom"]);
    }
}
