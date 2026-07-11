use super::*;

#[test]

fn parses_model_manifest() {
    let manifest: ModelManifest = toml::from_str(MODEL_TOML).expect("model manifest");

    assert_eq!(manifest.id, "kokoro");

    assert_eq!(manifest.family, "kokoro");

    assert_eq!(manifest.kind, ModelKind::Tts);

    assert_eq!(manifest.backend, ModelBackend::Onnx);

    assert_eq!(manifest.runner, "takokit-onnx");

    assert!(manifest.capabilities.tts);

    assert!(!manifest.capabilities.stt);

    assert!(manifest.capabilities.live_audio);

    assert_eq!(manifest.hardware.min_ram.as_deref(), Some("4gb"));
}

#[test]

fn parses_first_class_capabilities_from_manifest() {
    let manifest: ModelManifest = toml::from_str(MODEL_TOML).expect("model manifest");

    assert!(manifest.supports(CapabilityKind::TextToSpeech));

    assert!(manifest.supports(CapabilityKind::LiveAudio));

    assert!(!manifest.supports(CapabilityKind::SpeechToText));

    assert_eq!(
        manifest.capabilities.to_model_capabilities(),
        vec![CapabilityKind::TextToSpeech, CapabilityKind::LiveAudio]
    );
}

#[test]

fn parses_runner_manifest() {
    let manifest: RunnerManifest = toml::from_str(RUNNER_TOML).expect("runner manifest");

    assert_eq!(manifest.id, "takokit-onnx");

    assert_eq!(manifest.kind, RunnerKind::Onnx);

    assert_eq!(
        manifest.platforms,
        vec!["windows-x64", "linux-x64", "macos-arm64"]
    );
}

#[test]

fn parses_artifact_manifest_with_model_and_config_roles() {
    let source = format!(

        r#"

{MODEL_TOML}



[[artifacts.weights]]

name = "en_US-lessac-medium.onnx"

url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"

sha256 = "{HELLO_SHA256}"

bytes = 63200000

role = "model"



[[artifacts.configs]]

name = "en_US-lessac-medium.onnx.json"

url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"

sha256 = "{HELLO_SHA256}"

bytes = 4890

role = "config"

"#

    )

    .replace("[artifacts]\nweights = []\nvoices = []", "[artifacts]");

    let manifest: ModelManifest = toml::from_str(&source).expect("model manifest");

    assert_eq!(manifest.artifacts.weights[0].role, ArtifactRole::Model);

    assert_eq!(manifest.artifacts.configs[0].role, ArtifactRole::Config);

    assert_eq!(manifest.artifacts.all().count(), 2);
}
