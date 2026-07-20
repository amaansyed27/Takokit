use super::*;
use std::collections::HashSet;

#[test]
fn bundled_catalog_contains_exactly_thirty_one_unique_models() {
    let registry = PackageRegistry::bundled();
    let models = registry.models().expect("bundled models");
    assert_eq!(
        models.len(),
        31,
        "release catalog must contain exactly 31 models"
    );
    let unique = models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(unique.len(), models.len(), "model IDs must be unique");
}

#[test]
fn every_model_has_a_known_runner_and_capability() {
    let registry = PackageRegistry::bundled();
    for model in registry.models().expect("bundled models") {
        registry
            .runner(&model.runner)
            .unwrap_or_else(|error| panic!("model {} has unknown runner: {error}", model.id));
        assert!(
            model.capabilities.tts
                || model.capabilities.stt
                || model.capabilities.voice_cloning
                || model.capabilities.voice_training
                || model.capabilities.voice_conversion
                || model.capabilities.live_transcription
                || model.capabilities.live_audio,
            "model {} declares no capability",
            model.id
        );
    }
}

#[test]
fn executable_python_models_map_to_their_declared_adapter() {
    let registry = PackageRegistry::bundled();
    for model in registry.models().expect("bundled models") {
        let Some(required) = model.required_adapter.as_deref() else {
            continue;
        };
        let mapped = adapter_for_model(&model.id);
        assert_eq!(
            mapped,
            Some(required),
            "model {} declares adapter {required}, but model-to-adapter mapping is {mapped:?}",
            model.id
        );
    }
}
