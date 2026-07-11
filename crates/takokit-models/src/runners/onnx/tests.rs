use std::path::{Path, PathBuf};

use takokit_core::{CapabilityKind, ErrorCode, TakokitError};
use takokit_package::{
    ArtifactManifest, ArtifactRole, CapabilityManifest, ExecutionPlan, ExecutionStatus,
    HardwareManifest, InstalledArtifactRecord, InstalledModelRecord, InstalledPackageStatus,
    ModelBackend, ModelKind, ModelManifest, RunnerDependencyStrategy, RunnerKind,
    RunnerLifecycleState, RunnerManifest,
};

use super::{load_piper_config, resolve_piper_lessac_artifacts, OnnxRunner};
use crate::runners::SpeechRunner;

#[test]
fn resolves_piper_lessac_model_and_config_artifact_paths_from_installed_record() {
    let temp = tempfile::tempdir().expect("tempdir");
    let model_path = temp.path().join("en_US-lessac-medium.onnx");
    let config_path = temp.path().join("en_US-lessac-medium.onnx.json");
    std::fs::write(&model_path, b"model").expect("model fixture");
    std::fs::write(&config_path, b"{}").expect("config fixture");
    let plan = piper_plan(vec![
        artifact_record("en_US-lessac-medium.onnx", ArtifactRole::Model, &model_path),
        artifact_record(
            "en_US-lessac-medium.onnx.json",
            ArtifactRole::Config,
            &config_path,
        ),
    ]);

    let artifacts = resolve_piper_lessac_artifacts(&plan).expect("artifact paths");

    assert_eq!(artifacts.model_path, model_path);
    assert_eq!(artifacts.config_path, config_path);
}

#[test]
fn resolving_piper_lessac_artifacts_reports_missing_config_artifact() {
    let temp = tempfile::tempdir().expect("tempdir");
    let model_path = temp.path().join("en_US-lessac-medium.onnx");
    std::fs::write(&model_path, b"model").expect("model fixture");
    let plan = piper_plan(vec![artifact_record(
        "en_US-lessac-medium.onnx",
        ArtifactRole::Model,
        &model_path,
    )]);

    let error = resolve_piper_lessac_artifacts(&plan).expect_err("missing config");

    assert!(matches!(
        error,
        TakokitError::Resolution {
            code: ErrorCode::ArtifactMissing,
            message
        } if message.contains("en_US-lessac-medium.onnx.json")
    ));
}

#[test]
fn parses_piper_json_config_into_typed_struct() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("en_US-lessac-medium.onnx.json");
    std::fs::write(
        &config_path,
        r#"{
  "audio": { "sample_rate": 22050, "quality": "medium" },
  "espeak": { "voice": "en-us" },
  "inference": { "noise_scale": 0.667, "length_scale": 1.0, "noise_w": 0.8 },
  "phoneme_type": "espeak",
  "num_symbols": 256,
  "num_speakers": 1,
  "speaker_id_map": {}
}"#,
    )
    .expect("config fixture");

    let config = load_piper_config(&config_path).expect("piper config");

    assert_eq!(config.audio.sample_rate, 22050);
    assert_eq!(config.audio.quality.as_deref(), Some("medium"));
    assert_eq!(
        config.espeak.as_ref().map(|item| item.voice.as_str()),
        Some("en-us")
    );
    assert_eq!(config.num_speakers, 1);
    assert_eq!(config.phoneme_type.as_deref(), Some("espeak"));
}

#[tokio::test]
async fn piper_speech_reports_typed_text_frontend_blocker_after_artifact_prep() {
    let temp = tempfile::tempdir().expect("tempdir");
    let model_path = temp.path().join("en_US-lessac-medium.onnx");
    let config_path = temp.path().join("en_US-lessac-medium.onnx.json");
    std::fs::write(&model_path, b"model").expect("model fixture");
    std::fs::write(
        &config_path,
        r#"{
  "audio": { "sample_rate": 22050, "quality": "medium" },
  "espeak": { "voice": "en-us" },
  "inference": { "noise_scale": 0.667, "length_scale": 1.0, "noise_w": 0.8 },
  "phoneme_type": "espeak",
  "num_symbols": 256,
  "num_speakers": 1,
  "speaker_id_map": {}
}"#,
    )
    .expect("config fixture");
    let plan = piper_plan(vec![
        artifact_record("en_US-lessac-medium.onnx", ArtifactRole::Model, &model_path),
        artifact_record(
            "en_US-lessac-medium.onnx.json",
            ArtifactRole::Config,
            &config_path,
        ),
    ]);

    let error = OnnxRunner
        .speak(
            &plan,
            takokit_core::SpeechRequest {
                model: "piper-lessac".to_string(),
                input: "Hello from Takokit".to_string(),
                voice: None,
                response_format: Some("wav".to_string()),
            },
            temp.path(),
        )
        .await
        .expect_err("piper frontend is intentionally blocked");

    assert!(matches!(
        error,
        TakokitError::Resolution {
            code: ErrorCode::PiperTextFrontendNotImplemented,
            message
        } if message.contains("phonemizer/token preparation")
    ));
}

fn artifact_record(name: &str, role: ArtifactRole, local_path: &Path) -> InstalledArtifactRecord {
    InstalledArtifactRecord {
        name: name.to_string(),
        sha256: "test-sha256".to_string(),
        bytes: None,
        url: None,
        role,
        local_path: Some(local_path.to_path_buf()),
        downloaded: true,
    }
}

fn piper_plan(artifacts: Vec<InstalledArtifactRecord>) -> ExecutionPlan {
    ExecutionPlan {
        model: ModelManifest {
            id: "piper-lessac".to_string(),
            name: "Piper Lessac".to_string(),
            family: "piper".to_string(),
            version: "0.1.0".to_string(),
            kind: ModelKind::Tts,
            backend: ModelBackend::Onnx,
            runner: "takokit-onnx".to_string(),
            required_adapter: None,
            license: "mit".to_string(),
            description: "Piper Lessac voice.".to_string(),
            capabilities: CapabilityManifest {
                tts: true,
                stt: false,
                voice_cloning: false,
                live_transcription: false,
                live_audio: true,
            },
            hardware: HardwareManifest {
                cpu: true,
                gpu: false,
                min_ram: Some("2gb".to_string()),
            },
            artifacts: ArtifactManifest::default(),
        },
        capability: CapabilityKind::TextToSpeech,
        runner: RunnerManifest {
            id: "takokit-onnx".to_string(),
            name: "Takokit ONNX Runner".to_string(),
            version: "0.1.0".to_string(),
            kind: RunnerKind::Onnx,
            platforms: vec!["any".to_string()],
            supported_model_families: vec!["Piper".to_string()],
            supported_tasks: vec![CapabilityKind::TextToSpeech],
            dependency_strategy: RunnerDependencyStrategy::BundledNative,
            install_state: RunnerLifecycleState::ContractInstalled,
            notes: "test".to_string(),
            description: "ONNX scaffold.".to_string(),
        },
        runner_installed: true,
        status: ExecutionStatus::Planned,
        installed_model: Some(InstalledModelRecord {
            id: "piper-lessac".to_string(),
            version: "0.1.0".to_string(),
            source: "test".to_string(),
            manifest_path: PathBuf::from("piper-lessac.toml"),
            runner: "takokit-onnx".to_string(),
            installed_at: "0".to_string(),
            artifacts,
            status: InstalledPackageStatus::Ready,
            note: "test".to_string(),
        }),
    }
}
