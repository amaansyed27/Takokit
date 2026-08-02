use super::catalog::ModelRow;

pub const F0_METHODS: [&str; 4] = ["rmvpe", "harvest", "crepe", "pm"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertField {
    Model,
    Source,
    Target,
    F0Method,
    PitchShift,
    IndexRate,
    RmsMixRate,
    Protect,
    FilterRadius,
    Consent,
    Submit,
}

impl ConvertField {
    pub fn next(self) -> Self {
        match self {
            Self::Model => Self::Source,
            Self::Source => Self::Target,
            Self::Target => Self::F0Method,
            Self::F0Method => Self::PitchShift,
            Self::PitchShift => Self::IndexRate,
            Self::IndexRate => Self::RmsMixRate,
            Self::RmsMixRate => Self::Protect,
            Self::Protect => Self::FilterRadius,
            Self::FilterRadius => Self::Consent,
            Self::Consent => Self::Submit,
            Self::Submit => Self::Model,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Model => Self::Submit,
            Self::Source => Self::Model,
            Self::Target => Self::Source,
            Self::F0Method => Self::Target,
            Self::PitchShift => Self::F0Method,
            Self::IndexRate => Self::PitchShift,
            Self::RmsMixRate => Self::IndexRate,
            Self::Protect => Self::RmsMixRate,
            Self::FilterRadius => Self::Protect,
            Self::Consent => Self::FilterRadius,
            Self::Submit => Self::Consent,
        }
    }
}

pub struct ConvertState {
    pub model_indexes: Vec<usize>,
    pub model_index: usize,
    pub field: ConvertField,
    pub source: String,
    pub source_cursor: usize,
    pub target: String,
    pub target_cursor: usize,
    pub f0_method_index: usize,
    pub pitch_shift: String,
    pub pitch_shift_cursor: usize,
    pub index_rate: String,
    pub index_rate_cursor: usize,
    pub rms_mix_rate: String,
    pub rms_mix_rate_cursor: usize,
    pub protect: String,
    pub protect_cursor: usize,
    pub filter_radius: String,
    pub filter_radius_cursor: usize,
    pub consent: bool,
}

impl ConvertState {
    pub fn new(models: &[ModelRow]) -> Self {
        Self {
            model_indexes: conversion_indexes(models),
            model_index: 0,
            field: ConvertField::Source,
            source: String::new(),
            source_cursor: 0,
            target: String::new(),
            target_cursor: 0,
            f0_method_index: 0,
            pitch_shift: "0".to_string(),
            pitch_shift_cursor: 1,
            index_rate: "0.75".to_string(),
            index_rate_cursor: 4,
            rms_mix_rate: "0.25".to_string(),
            rms_mix_rate_cursor: 4,
            protect: "0.33".to_string(),
            protect_cursor: 4,
            filter_radius: "3".to_string(),
            filter_radius_cursor: 1,
            consent: false,
        }
    }

    pub fn reload_models(&mut self, models: &[ModelRow]) {
        let selected = self
            .model_indexes
            .get(self.model_index)
            .and_then(|index| models.get(*index))
            .map(|model| model.id.clone());
        self.model_indexes = conversion_indexes(models);
        self.model_index = selected
            .and_then(|id| {
                self.model_indexes
                    .iter()
                    .position(|index| models[*index].id == id)
            })
            .unwrap_or(0);
    }

    pub fn f0_method(&self) -> &'static str {
        F0_METHODS[self.f0_method_index.min(F0_METHODS.len() - 1)]
    }
}

fn conversion_indexes(models: &[ModelRow]) -> Vec<usize> {
    models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| model.voice_conversion.then_some(index))
        .collect()
}
