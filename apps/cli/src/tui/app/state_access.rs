use super::*;

impl App {
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

    pub fn compatible_speak_voice_ids(&self) -> Vec<String> {
        let Some(model) = self.selected_speak_model() else {
            return vec!["default".to_string()];
        };
        let mut voices = vec!["default".to_string()];
        voices.extend(
            self.voice_profiles
                .iter()
                .filter(|profile| profile.model_id == model.id)
                .map(|profile| profile.id.clone()),
        );
        voices
    }

    pub fn compatible_saved_voice_count(&self) -> usize {
        self.compatible_speak_voice_ids().len().saturating_sub(1)
    }

    pub fn cycle_speak_voice(&mut self, delta: isize) {
        let voices = self.compatible_speak_voice_ids();
        if voices.is_empty() {
            return;
        }
        let current = voices
            .iter()
            .position(|voice| voice == &self.speak_voice)
            .unwrap_or(0);
        let len = voices.len() as isize;
        let next = (current as isize + delta).rem_euclid(len) as usize;
        self.speak_voice = voices[next].clone();
        self.speak_voice_cursor = self.speak_voice.chars().count();
    }

    pub fn normalize_speak_voice_for_model(&mut self) {
        let voices = self.compatible_speak_voice_ids();
        if voices.iter().any(|voice| voice == &self.speak_voice) {
            if self.speak_voice == "default"
                && self
                    .selected_speak_model()
                    .is_some_and(|model| model.voice_cloning)
                && voices.len() > 1
            {
                self.speak_voice = voices[1].clone();
                self.speak_voice_cursor = self.speak_voice.chars().count();
            }
            return;
        }
        self.speak_voice = if self
            .selected_speak_model()
            .is_some_and(|model| model.voice_cloning)
            && voices.len() > 1
        {
            voices[1].clone()
        } else {
            "default".to_string()
        };
        self.speak_voice_cursor = self.speak_voice.chars().count();
    }

    pub fn set_speak_model(&mut self, id: &str) {
        self.speak_model_index =
            find_capability_index(&self.models, &self.tts_models, Some(id), id);
        self.normalize_speak_voice_for_model();
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
}
