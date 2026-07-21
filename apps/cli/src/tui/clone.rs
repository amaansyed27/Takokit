use super::catalog::ModelRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneField {
    Model,
    Name,
    Sample,
    Consent,
    Submit,
}

impl CloneField {
    pub fn next(self) -> Self {
        match self {
            Self::Model => Self::Name,
            Self::Name => Self::Sample,
            Self::Sample => Self::Consent,
            Self::Consent => Self::Submit,
            Self::Submit => Self::Model,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Model => Self::Submit,
            Self::Name => Self::Model,
            Self::Sample => Self::Name,
            Self::Consent => Self::Sample,
            Self::Submit => Self::Consent,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloneState {
    pub model_indexes: Vec<usize>,
    pub model_index: usize,
    pub field: CloneField,
    pub name: String,
    pub name_cursor: usize,
    pub sample: String,
    pub sample_cursor: usize,
    pub consent: bool,
}

impl CloneState {
    pub fn new(models: &[ModelRow]) -> Self {
        let model_indexes = clone_model_indexes(models);
        let model_index = model_indexes
            .iter()
            .position(|index| models[*index].id == "chatterbox")
            .unwrap_or(0);
        Self {
            model_indexes,
            model_index,
            field: CloneField::Name,
            name: String::new(),
            name_cursor: 0,
            sample: String::new(),
            sample_cursor: 0,
            consent: false,
        }
    }

    pub fn reload_models(&mut self, models: &[ModelRow]) {
        let selected = self
            .model_indexes
            .get(self.model_index)
            .and_then(|index| models.get(*index))
            .map(|model| model.id.clone());
        self.model_indexes = clone_model_indexes(models);
        self.model_index = selected
            .and_then(|id| {
                self.model_indexes
                    .iter()
                    .position(|index| models[*index].id == id)
            })
            .unwrap_or(0);
    }
}

fn clone_model_indexes(models: &[ModelRow]) -> Vec<usize> {
    models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| model.voice_cloning.then_some(index))
        .collect()
}
