const PYTHON_RUNNER: &str =
    include_str!("../../takokit-models/src/runners/python_managed.rs");
const PREFETCH: &str = include_str!("../src/runtime_python/prefetch.rs");
const RUNNER_DISPATCH: &str = include_str!("../../takokit-models/src/runners/mod.rs");

#[test]
fn custom_python_models_keep_their_checkpoint_directory_but_use_the_base_adapter_contract() {
    assert!(PYTHON_RUNNER.contains("runtime_model_id(&plan.model)"));
    assert!(PYTHON_RUNNER.contains("model_id: runtime_model"));
    assert!(PYTHON_RUNNER.contains("model_dir: &model_dir"));
    assert!(PREFETCH.contains("runtime_model_id(model)"));
    assert!(PREFETCH.contains("model_dir: &model_dir"));
}

#[test]
fn every_speech_surface_uses_the_same_voice_contract_validation() {
    assert!(RUNNER_DISPATCH.contains("validate_speech_request(&plan.model, &request)?"));
}
