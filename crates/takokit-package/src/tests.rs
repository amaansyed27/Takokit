use super::*;
use crate::artifact_io::sha256_file;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use takokit_core::CapabilityKind;

const MODEL_TOML: &str = r#"
id = "kokoro"
name = "Kokoro"
family = "kokoro"
version = "0.1.0"
kind = "tts"
backend = "onnx"
runner = "takokit-onnx"
license = "apache-2.0"
description = "Fast local text-to-speech model."

[capabilities]
tts = true
stt = false
voice_cloning = false
live_transcription = false
live_audio = true

[hardware]
cpu = true
gpu = false
min_ram = "4gb"

[artifacts]
weights = []
voices = []
"#;

const RUNNER_TOML: &str = r#"
id = "takokit-onnx"
name = "Takokit ONNX Runner"
version = "0.1.0"
kind = "onnx"
platforms = ["windows-x64", "linux-x64", "macos-arm64"]
description = "Native ONNX runner for CPU-friendly models."
"#;

const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

mod artifact_install;
mod catalog;
mod manifest;
mod package_records;
mod resolution;
mod runtime;

#[derive(Debug, Deserialize)]
struct ModelLifecycleFixture {
    state: ModelLifecycleState,
}

#[derive(Debug, Deserialize)]
struct RunnerLifecycleFixture {
    state: RunnerLifecycleState,
}

fn write_test_registry(root: &Path) {
    let models = root.join("models");
    let runners = root.join("runners");
    std::fs::create_dir_all(&models).expect("models dir");
    std::fs::create_dir_all(&runners).expect("runners dir");
    std::fs::write(models.join("kokoro.toml"), MODEL_TOML).expect("model toml");
    std::fs::write(runners.join("takokit-onnx.toml"), RUNNER_TOML).expect("runner toml");
}

fn artifact_test_manifest(source: &Path, sha256: &str) -> ModelManifest {
    let source = source.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"
id = "piper-lessac"
name = "Piper Lessac"
family = "piper"
version = "0.1.0"
kind = "tts"
backend = "onnx"
runner = "takokit-onnx"
license = "mit"
description = "Piper Lessac test manifest."

[capabilities]
tts = true
stt = false
voice_cloning = false
live_transcription = false
live_audio = true

[hardware]
cpu = true
gpu = false
min_ram = "2gb"

[artifacts]

[[artifacts.weights]]
name = "fixture.onnx"
url = "{source}"
sha256 = "{sha256}"
bytes = 5
role = "model"
"#
    );

    toml::from_str(&toml).expect("artifact manifest")
}

fn multi_artifact_test_manifest(
    model_source: &Path,
    model_sha256: &str,
    config_source: &Path,
    config_sha256: &str,
) -> ModelManifest {
    let model_source = model_source.to_string_lossy().replace('\\', "/");
    let config_source = config_source.to_string_lossy().replace('\\', "/");
    let toml = format!(
        r#"
id = "piper-lessac"
name = "Piper Lessac"
family = "piper"
version = "0.1.0"
kind = "tts"
backend = "onnx"
runner = "takokit-onnx"
license = "mit"
description = "Piper Lessac multi-artifact test manifest."

[capabilities]
tts = true
stt = false
voice_cloning = false
live_transcription = false
live_audio = true

[hardware]
cpu = true
gpu = false
min_ram = "2gb"

[artifacts]

[[artifacts.weights]]
name = "fixture.onnx"
url = "{model_source}"
sha256 = "{model_sha256}"
bytes = 5
role = "model"

[[artifacts.configs]]
name = "fixture.onnx.json"
url = "{config_source}"
sha256 = "{config_sha256}"
bytes = 31
role = "config"
"#
    );

    toml::from_str(&toml).expect("multi artifact manifest")
}
