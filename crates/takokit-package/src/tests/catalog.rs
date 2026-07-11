use super::*;

#[test]

fn bundled_library_model_manifests_parse_with_allowed_enums() {
    let registry = PackageRegistry::bundled();

    let models = registry.library_models().expect("library models");

    assert!(models.iter().any(|model| model.id == "piper-lessac"));

    assert!(models.iter().any(|model| model.id == "whisper"));

    assert!(models.iter().any(|model| model.id == "qwen3-tts"));

    assert!(models.iter().any(|model| model.id == "voxtral"));

    assert!(models
        .iter()
        .all(|model| !model.tasks.is_empty() && !model.languages.is_empty()));

    assert!(models
        .iter()
        .filter(|model| model.runtime_status == LibraryRuntimeStatus::Supported)
        .all(|model| model.id == "piper-lessac"));
}

#[test]

fn bundled_library_runner_manifests_parse_with_allowed_enums() {
    let registry = PackageRegistry::bundled();

    let runners = registry.library_runners().expect("library runners");

    assert!(runners.iter().any(|runner| runner.id == "takokit-onnx"));

    assert!(runners
        .iter()
        .any(|runner| runner.id == "takokit-transformers-audio"));

    assert!(runners
        .iter()
        .any(|runner| runner.id == "takokit-python-managed"));

    assert!(runners
        .iter()
        .all(|runner| !runner.notes.is_empty() && !runner.supported_platforms.is_empty()));
}

#[test]

fn bundled_runtime_runner_manifests_cover_shared_runner_families() {
    let registry = PackageRegistry::bundled();

    let runners = registry.runners().expect("runtime runners");

    let ids: Vec<_> = runners.iter().map(|runner| runner.id.as_str()).collect();

    for required in [
        "takokit-onnx",
        "takokit-whispercpp",
        "takokit-python-managed",
        "takokit-transformers-audio",
        "takokit-nemo",
    ] {
        assert!(ids.contains(&required), "missing runtime runner {required}");
    }

    let python = registry
        .runner("takokit-python-managed")
        .expect("python-managed runner");

    assert!(python
        .supported_model_families
        .iter()
        .any(|family| family == "Qwen3-TTS"));

    assert!(python
        .supported_tasks
        .contains(&CapabilityKind::TextToSpeech));

    assert_eq!(
        python.dependency_strategy,
        RunnerDependencyStrategy::Managed
    );

    assert!(python.notes.contains("Python"));
}

#[test]

fn bundled_runtime_model_manifests_cover_launch_families() {
    let registry = PackageRegistry::bundled();

    let models = registry.models().expect("runtime models");

    let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();

    for required in [
        "piper-lessac",
        "kokoro",
        "whisper-base",
        "qwen3-tts",
        "cosyvoice2",
        "f5-tts",
        "fish-speech",
        "dia",
        "chatterbox",
        "gpt-sovits",
        "openvoice",
        "rvc",
        "qwen3-omni",
        "qwen2-5-omni",
        "voxtral",
        "sensevoice",
        "parakeet",
        "canary",
    ] {
        assert!(ids.contains(&required), "missing runtime model {required}");
    }

    let runners: std::collections::HashSet<_> = registry
        .runners()
        .expect("runtime runners")
        .into_iter()
        .map(|runner| runner.id)
        .collect();

    for model in models {
        assert!(
            runners.contains(&model.runner),
            "{} references unknown runner {}",
            model.id,
            model.runner
        );

        assert!(
            !model.capabilities.to_model_capabilities().is_empty(),
            "{} has no capabilities",
            model.id
        );
    }
}
