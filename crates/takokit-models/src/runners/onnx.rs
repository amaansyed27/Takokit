use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use takokit_core::{
    ErrorCode, SpeechRequest, SpeechResponse, TakokitError, TakokitResult, TranscriptionRequest,
    TranscriptionResponse,
};
use takokit_package::{ArtifactRole, ExecutionPlan, InstalledArtifactRecord};
use uuid::Uuid;

use super::{onnx_not_implemented, piper_not_implemented, SpeechRunner, TranscriptionRunner};

pub const PIPER_LESSAC_MODEL_ARTIFACT: &str = "en_US-lessac-medium.onnx";
pub const PIPER_LESSAC_CONFIG_ARTIFACT: &str = "en_US-lessac-medium.onnx.json";
pub const KOKORO_MODEL_ARTIFACT: &str = "kokoro-v1.0.int8.onnx";
pub const KOKORO_VOICES_ARTIFACT: &str = "voices-v1.0.bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiperLessacArtifactPaths {
    pub model_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PiperConfig {
    pub audio: PiperAudioConfig,
    #[serde(default)]
    pub espeak: Option<PiperEspeakConfig>,
    #[serde(default)]
    pub inference: Option<PiperInferenceConfig>,
    #[serde(default)]
    pub phoneme_type: Option<String>,
    pub num_symbols: u32,
    pub num_speakers: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PiperAudioConfig {
    pub sample_rate: u32,
    #[serde(default)]
    pub quality: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PiperEspeakConfig {
    pub voice: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PiperInferenceConfig {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PiperLessacInputs {
    pub artifacts: PiperLessacArtifactPaths,
    pub config: PiperConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KokoroArtifactPaths {
    pub model_path: PathBuf,
    pub voices_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct KokoroAdapterRequest<'a> {
    input: &'a str,
    voice: Option<&'a str>,
    model_path: &'a Path,
    voices_path: &'a Path,
    output_path: &'a Path,
}

#[derive(Debug, Deserialize)]
struct KokoroAdapterResponse {
    ok: bool,
    output_path: Option<PathBuf>,
    bytes: Option<u64>,
    sample_rate: Option<u32>,
    voice: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct OnnxRunner;

#[async_trait]
impl SpeechRunner for OnnxRunner {
    async fn speak(
        &self,
        plan: &ExecutionPlan,
        request: SpeechRequest,
        output_dir: &Path,
    ) -> TakokitResult<SpeechResponse> {
        if plan.model.id == "piper-lessac" {
            prepare_piper_lessac(plan)?;
            return Err(piper_not_implemented());
        }

        if plan.model.id == "kokoro" {
            return speak_with_kokoro(plan, request, output_dir);
        }

        Err(onnx_not_implemented())
    }
}

#[async_trait]
impl TranscriptionRunner for OnnxRunner {
    async fn transcribe(
        &self,
        _plan: &ExecutionPlan,
        _request: TranscriptionRequest,
    ) -> TakokitResult<TranscriptionResponse> {
        Err(onnx_not_implemented())
    }
}

pub fn prepare_piper_lessac(plan: &ExecutionPlan) -> TakokitResult<PiperLessacInputs> {
    let artifacts = resolve_piper_lessac_artifacts(plan)?;
    let config = load_piper_config(&artifacts.config_path)?;

    Ok(PiperLessacInputs { artifacts, config })
}

pub fn speak_with_kokoro(
    plan: &ExecutionPlan,
    request: SpeechRequest,
    output_dir: &Path,
) -> TakokitResult<SpeechResponse> {
    if request.input.trim().is_empty() {
        return Err(TakokitError::InvalidRequest(
            "speech input cannot be empty".to_string(),
        ));
    }
    let artifacts = resolve_kokoro_artifacts(plan)?;
    let takokit_root = output_dir.parent().ok_or_else(|| {
        inference_missing(format!(
            "could not infer Takokit root from output directory {}",
            output_dir.display()
        ))
    })?;
    let runner_root = takokit_root.join("runners").join("onnx");
    let python = runner_python_path(&runner_root).ok_or_else(|| {
        inference_missing(format!(
            "Kokoro ONNX runtime is not ready at {}; run `takokit runner install takokit-onnx`",
            runner_root.display()
        ))
    })?;
    let adapter = runner_root.join("adapters").join("kokoro.py");
    if !adapter.is_file() {
        return Err(inference_missing(format!(
            "Kokoro adapter is missing at {}; run `takokit runner install takokit-onnx`",
            adapter.display()
        )));
    }

    std::fs::create_dir_all(output_dir)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
    let id = Uuid::new_v4();
    let output_path = output_dir.join(format!("speech-{id}.wav"));
    let payload = serde_json::to_vec(&KokoroAdapterRequest {
        input: &request.input,
        voice: request.voice.as_deref().filter(|voice| *voice != "default"),
        model_path: &artifacts.model_path,
        voices_path: &artifacts.voices_path,
        output_path: &output_path,
    })
    .map_err(|error| {
        TakokitError::Audio(format!("could not encode Kokoro adapter request: {error}"))
    })?;

    let mut child = Command::new(&python)
        .arg(&adapter)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| TakokitError::Audio(format!("could not start Kokoro adapter: {error}")))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| TakokitError::Audio("Kokoro adapter stdin was unavailable".to_string()))?
        .write_all(&payload)
        .map_err(|error| {
            TakokitError::Audio(format!("could not send Kokoro adapter request: {error}"))
        })?;
    let output = child.wait_with_output().map_err(|error| {
        TakokitError::Audio(format!("could not wait for Kokoro adapter: {error}"))
    })?;
    let adapter_response: KokoroAdapterResponse =
        serde_json::from_slice(&output.stdout).map_err(|error| {
            TakokitError::Audio(format!(
                "Kokoro adapter returned invalid JSON ({error}): {}",
                String::from_utf8_lossy(&output.stdout).trim()
            ))
        })?;
    if !output.status.success() || !adapter_response.ok {
        return Err(TakokitError::Audio(format!(
            "Kokoro adapter failed: {}{}",
            adapter_response
                .error
                .unwrap_or_else(|| "unknown adapter failure".to_string()),
            stderr_suffix(&output.stderr)
        )));
    }
    let reported_path = adapter_response.output_path.ok_or_else(|| {
        TakokitError::Audio("Kokoro adapter did not return an output path".to_string())
    })?;
    if reported_path != output_path || !output_path.is_file() {
        return Err(TakokitError::Audio(format!(
            "Kokoro adapter did not create the requested WAV output at {}",
            output_path.display()
        )));
    }
    let actual_bytes = std::fs::metadata(&output_path)
        .map_err(|error| TakokitError::Storage(error.to_string()))?
        .len();
    if adapter_response
        .bytes
        .is_some_and(|reported| reported != actual_bytes)
    {
        return Err(TakokitError::Audio(
            "Kokoro adapter reported a byte count that does not match the WAV output".to_string(),
        ));
    }

    Ok(SpeechResponse {
        id,
        model: plan.model.id.clone(),
        voice: adapter_response.voice.or(request.voice),
        engine: "kokoro-onnx".to_string(),
        output_path,
        content_type: "audio/wav".to_string(),
        bytes: actual_bytes,
        sample_rate: adapter_response.sample_rate,
    })
}

pub fn resolve_kokoro_artifacts(plan: &ExecutionPlan) -> TakokitResult<KokoroArtifactPaths> {
    let record = plan.installed_model.as_ref().ok_or_else(|| {
        artifact_error(
            ErrorCode::ArtifactMissing,
            format!(
                "installed model record for {} is missing; pull the model before running it",
                plan.model.id
            ),
        )
    })?;
    Ok(KokoroArtifactPaths {
        model_path: resolve_artifact_path(
            &record.artifacts,
            KOKORO_MODEL_ARTIFACT,
            ArtifactRole::Model,
        )?,
        voices_path: resolve_artifact_path(
            &record.artifacts,
            KOKORO_VOICES_ARTIFACT,
            ArtifactRole::Voice,
        )?,
    })
}

pub fn resolve_piper_lessac_artifacts(
    plan: &ExecutionPlan,
) -> TakokitResult<PiperLessacArtifactPaths> {
    let record = plan.installed_model.as_ref().ok_or_else(|| {
        artifact_error(
            ErrorCode::ArtifactMissing,
            format!(
                "installed model record for {} is missing; pull the model before running it",
                plan.model.id
            ),
        )
    })?;

    Ok(PiperLessacArtifactPaths {
        model_path: resolve_artifact_path(
            &record.artifacts,
            PIPER_LESSAC_MODEL_ARTIFACT,
            ArtifactRole::Model,
        )?,
        config_path: resolve_artifact_path(
            &record.artifacts,
            PIPER_LESSAC_CONFIG_ARTIFACT,
            ArtifactRole::Config,
        )?,
    })
}

pub fn load_piper_config(path: &Path) -> TakokitResult<PiperConfig> {
    let source = std::fs::read_to_string(path).map_err(|error| {
        artifact_error(
            ErrorCode::ArtifactMissing,
            format!(
                "Piper config artifact {} could not be read: {error}",
                path.display()
            ),
        )
    })?;

    serde_json::from_str(&source).map_err(|error| {
        artifact_error(
            ErrorCode::ArtifactConfigInvalid,
            format!(
                "Piper config artifact {} is invalid JSON: {error}",
                path.display()
            ),
        )
    })
}

fn resolve_artifact_path(
    artifacts: &[InstalledArtifactRecord],
    name: &str,
    role: ArtifactRole,
) -> TakokitResult<PathBuf> {
    let artifact = artifacts
        .iter()
        .find(|artifact| artifact.name == name && artifact.role == role)
        .ok_or_else(|| {
            artifact_error(
                ErrorCode::ArtifactMissing,
                format!("required Piper artifact {name} is missing from the installed record"),
            )
        })?;

    if !artifact.downloaded {
        return Err(artifact_error(
            ErrorCode::ArtifactNotDownloaded,
            format!("required Piper artifact {name} is recorded but not downloaded"),
        ));
    }

    let path = artifact.local_path.clone().ok_or_else(|| {
        artifact_error(
            ErrorCode::ArtifactMissing,
            format!("required Piper artifact {name} has no local path in the installed record"),
        )
    })?;

    if !path.is_file() {
        return Err(artifact_error(
            ErrorCode::ArtifactMissing,
            format!(
                "required Piper artifact {name} is missing at {}",
                path.display()
            ),
        ));
    }

    Ok(path)
}

fn artifact_error(code: ErrorCode, message: impl Into<String>) -> TakokitError {
    TakokitError::Resolution {
        code,
        message: message.into(),
    }
}

fn runner_python_path(runner_root: &Path) -> Option<PathBuf> {
    let venv = runner_root.join("runtime").join("venv");
    let candidates = if cfg!(windows) {
        vec![venv.join("Scripts").join("python.exe")]
    } else {
        vec![
            venv.join("bin").join("python3"),
            venv.join("bin").join("python"),
        ]
    };
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn inference_missing(message: impl Into<String>) -> TakokitError {
    TakokitError::Resolution {
        code: ErrorCode::InferenceNotImplemented,
        message: message.into(),
    }
}

fn stderr_suffix(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        String::new()
    } else {
        format!("; {stderr}")
    }
}

#[cfg(test)]
mod tests {
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

    fn artifact_record(
        name: &str,
        role: ArtifactRole,
        local_path: &Path,
    ) -> InstalledArtifactRecord {
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
}
