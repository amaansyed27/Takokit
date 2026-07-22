use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use takokit_core::{
    ErrorCode, SpeechRequest, SpeechResponse, TakokitError, TakokitResult, TrainVoiceRequest,
    TrainVoiceResponse, TranscriptionRequest, TranscriptionResponse, VoiceConversionRequest,
    VoiceConversionResponse,
};
use takokit_package::{adapter_for_model, ExecutionPlan};
use takokit_store::VoiceProfileStore;
use uuid::Uuid;

use super::{
    configure_runner_command, SpeechRunner, TranscriptionRunner, VoiceConversionRunner,
    VoiceTrainingRunner,
};

#[derive(Debug, Default, Clone)]
pub struct PythonManagedRunner;

#[derive(Debug, Serialize)]
struct ManagedAdapterRequest<'a> {
    operation: &'a str,
    model_id: &'a str,
    model_dir: &'a Path,
    cache_dir: &'a Path,
    input: Option<&'a str>,
    voice: Option<&'a str>,
    language: Option<&'a str>,
    instruction: Option<&'a str>,
    reference_text: Option<&'a str>,
    output_path: Option<&'a Path>,
    output_dir: Option<&'a Path>,
    audio_path: Option<&'a Path>,
    target_voice: Option<&'a str>,
    dataset_path: Option<&'a Path>,
    name: Option<&'a str>,
    pitch_shift: Option<i32>,
    epochs: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ManagedAdapterResponse {
    ok: bool,
    output_path: Option<PathBuf>,
    bytes: Option<u64>,
    sample_rate: Option<u32>,
    voice: Option<String>,
    text: Option<String>,
    status: Option<String>,
    log_path: Option<PathBuf>,
    error: Option<String>,
}

#[async_trait]
impl SpeechRunner for PythonManagedRunner {
    async fn speak(
        &self,
        plan: &ExecutionPlan,
        request: SpeechRequest,
        output_dir: &Path,
    ) -> TakokitResult<SpeechResponse> {
        speak_with_adapter(plan, request, output_dir)
    }
}

#[async_trait]
impl TranscriptionRunner for PythonManagedRunner {
    async fn transcribe(
        &self,
        plan: &ExecutionPlan,
        request: TranscriptionRequest,
    ) -> TakokitResult<TranscriptionResponse> {
        transcribe_with_adapter(plan, request)
    }
}

#[async_trait]
impl VoiceConversionRunner for PythonManagedRunner {
    async fn convert(
        &self,
        plan: &ExecutionPlan,
        request: VoiceConversionRequest,
        output_dir: &Path,
    ) -> TakokitResult<VoiceConversionResponse> {
        convert_with_adapter(plan, request, output_dir)
    }
}

#[async_trait]
impl VoiceTrainingRunner for PythonManagedRunner {
    async fn train(
        &self,
        plan: &ExecutionPlan,
        request: TrainVoiceRequest,
    ) -> TakokitResult<TrainVoiceResponse> {
        train_with_adapter(plan, request)
    }
}

fn speak_with_adapter(
    plan: &ExecutionPlan,
    request: SpeechRequest,
    output_dir: &Path,
) -> TakokitResult<SpeechResponse> {
    if request.input.trim().is_empty() {
        return Err(TakokitError::InvalidRequest(
            "speech input cannot be empty".to_string(),
        ));
    }
    let adapter = adapter_id(plan)?;
    let layout = adapter_layout(plan, adapter)?;
    let resolved_voice = resolve_speech_voice(plan, request.voice.as_deref())?;
    std::fs::create_dir_all(output_dir)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
    let id = Uuid::new_v4();
    let output_path = output_dir.join(format!("speech-{id}.wav"));
    let model_dir = plan.storage_root.join("models").join(&plan.model.id);
    let cache_dir = plan.storage_root.join("cache");
    let payload = ManagedAdapterRequest {
        operation: "speech",
        model_id: &plan.model.id,
        model_dir: &model_dir,
        cache_dir: &cache_dir,
        input: Some(&request.input),
        voice: resolved_voice.as_deref(),
        language: request.language.as_deref(),
        instruction: request.instruction.as_deref(),
        reference_text: request.reference_text.as_deref(),
        output_path: Some(&output_path),
        output_dir: None,
        audio_path: None,
        target_voice: None,
        dataset_path: None,
        name: None,
        pitch_shift: None,
        epochs: None,
    };
    let response = run_adapter(adapter, &layout, &payload)?;
    validate_file_output(adapter, &output_path, response.output_path.as_deref())?;
    let bytes = output_bytes(&output_path)?;
    if response.bytes.is_some_and(|reported| reported != bytes) {
        return Err(TakokitError::Audio(format!(
            "{adapter} adapter reported a byte count that does not match the output"
        )));
    }
    Ok(SpeechResponse {
        id,
        model: plan.model.id.clone(),
        voice: request.voice.or(response.voice),
        engine: adapter.replace('_', "-"),
        output_path,
        content_type: "audio/wav".to_string(),
        bytes,
        sample_rate: response.sample_rate,
    })
}

fn transcribe_with_adapter(
    plan: &ExecutionPlan,
    request: TranscriptionRequest,
) -> TakokitResult<TranscriptionResponse> {
    if !request.file_path.is_file() {
        return Err(TakokitError::InvalidRequest(format!(
            "audio file does not exist: {}",
            request.file_path.display()
        )));
    }
    let adapter = adapter_id(plan)?;
    let layout = adapter_layout(plan, adapter)?;
    let model_dir = plan.storage_root.join("models").join(&plan.model.id);
    let cache_dir = plan.storage_root.join("cache");
    let payload = ManagedAdapterRequest {
        operation: "transcribe",
        model_id: &plan.model.id,
        model_dir: &model_dir,
        cache_dir: &cache_dir,
        input: None,
        voice: None,
        language: None,
        instruction: None,
        reference_text: None,
        output_path: None,
        output_dir: None,
        audio_path: Some(&request.file_path),
        target_voice: None,
        dataset_path: None,
        name: None,
        pitch_shift: None,
        epochs: None,
    };
    let response = run_adapter(adapter, &layout, &payload)?;
    let text = response
        .text
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| TakokitError::Audio(format!("{adapter} returned no transcript text")))?;
    Ok(TranscriptionResponse {
        id: Uuid::new_v4(),
        model: plan.model.id.clone(),
        text,
    })
}

fn convert_with_adapter(
    plan: &ExecutionPlan,
    request: VoiceConversionRequest,
    output_dir: &Path,
) -> TakokitResult<VoiceConversionResponse> {
    if !request.consent_affirmed {
        return Err(TakokitError::InvalidRequest(
            "voice conversion requires explicit ownership or permission consent".to_string(),
        ));
    }
    if !request.source_path.is_file() {
        return Err(TakokitError::InvalidRequest(format!(
            "source audio does not exist: {}",
            request.source_path.display()
        )));
    }
    let adapter = adapter_id(plan)?;
    let layout = adapter_layout(plan, adapter)?;
    let target_voice = resolve_target_voice(plan, &request.target_voice)?;
    std::fs::create_dir_all(output_dir)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
    let id = Uuid::new_v4();
    let output_path = output_dir.join(format!("conversion-{id}.wav"));
    let model_dir = plan.storage_root.join("models").join(&plan.model.id);
    let cache_dir = plan.storage_root.join("cache");
    let payload = ManagedAdapterRequest {
        operation: "convert",
        model_id: &plan.model.id,
        model_dir: &model_dir,
        cache_dir: &cache_dir,
        input: None,
        voice: None,
        language: None,
        instruction: None,
        reference_text: None,
        output_path: Some(&output_path),
        output_dir: None,
        audio_path: Some(&request.source_path),
        target_voice: Some(&target_voice),
        dataset_path: None,
        name: None,
        pitch_shift: Some(request.pitch_shift),
        epochs: None,
    };
    let response = run_adapter(adapter, &layout, &payload)?;
    validate_file_output(adapter, &output_path, response.output_path.as_deref())?;
    Ok(VoiceConversionResponse {
        id,
        model: plan.model.id.clone(),
        target_voice: request.target_voice,
        output_path: output_path.clone(),
        content_type: "audio/wav".to_string(),
        bytes: output_bytes(&output_path)?,
        sample_rate: response.sample_rate,
    })
}

fn train_with_adapter(
    plan: &ExecutionPlan,
    request: TrainVoiceRequest,
) -> TakokitResult<TrainVoiceResponse> {
    if !request.consent_affirmed {
        return Err(TakokitError::InvalidRequest(
            "voice training requires explicit ownership or permission consent".to_string(),
        ));
    }
    if !request.samples_path.is_dir() {
        return Err(TakokitError::InvalidRequest(format!(
            "training dataset directory does not exist: {}",
            request.samples_path.display()
        )));
    }
    let adapter = adapter_id(plan)?;
    let layout = adapter_layout(plan, adapter)?;
    let id = Uuid::new_v4();
    let output_dir = plan
        .storage_root
        .join("voices")
        .join("trained")
        .join(id.to_string());
    std::fs::create_dir_all(&output_dir)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
    let model_dir = plan.storage_root.join("models").join(&plan.model.id);
    let cache_dir = plan.storage_root.join("cache");
    let payload = ManagedAdapterRequest {
        operation: "train",
        model_id: &plan.model.id,
        model_dir: &model_dir,
        cache_dir: &cache_dir,
        input: None,
        voice: None,
        language: None,
        instruction: None,
        reference_text: None,
        output_path: None,
        output_dir: Some(&output_dir),
        audio_path: None,
        target_voice: None,
        dataset_path: Some(&request.samples_path),
        name: Some(&request.name),
        pitch_shift: None,
        epochs: request.epochs,
    };
    let response = run_adapter(adapter, &layout, &payload)?;
    let reported = response.output_path.unwrap_or_else(|| output_dir.clone());
    if !reported.exists() {
        return Err(TakokitError::Storage(format!(
            "{adapter} did not create the reported training output: {}",
            reported.display()
        )));
    }
    Ok(TrainVoiceResponse {
        id,
        model: plan.model.id.clone(),
        name: request.name,
        output_path: reported,
        status: response.status.unwrap_or_else(|| "completed".to_string()),
        log_path: response.log_path,
    })
}

#[derive(Debug)]
struct AdapterLayout {
    python: PathBuf,
    script: PathBuf,
}

fn adapter_layout(plan: &ExecutionPlan, adapter: &str) -> TakokitResult<AdapterLayout> {
    let runner_root = plan.storage_root.join("runners").join("python-managed");
    let adapter_dir = runner_root.join("adapters").join(adapter);
    let script = adapter_dir.join(format!("{adapter}.py"));
    if !script.is_file() {
        return Err(inference_missing(format!(
            "{adapter} adapter is missing; run `takokit adapter install {adapter}`"
        )));
    }
    let python = adapter_python_path(&adapter_dir)
        .or_else(|| runner_python_path(&runner_root))
        .ok_or_else(|| {
            inference_missing(format!(
                "{adapter} Python environment is missing; run `takokit adapter install {adapter}`"
            ))
        })?;
    Ok(AdapterLayout { python, script })
}

fn adapter_id(plan: &ExecutionPlan) -> TakokitResult<&str> {
    plan.model
        .required_adapter
        .as_deref()
        .or_else(|| adapter_for_model(&plan.model.id))
        .ok_or_else(|| blocked_adapter(&plan.model.id))
}

fn resolve_speech_voice(
    plan: &ExecutionPlan,
    voice: Option<&str>,
) -> TakokitResult<Option<String>> {
    let Some(voice) = voice.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if voice == "default" {
        return Ok(None);
    }
    if !plan.model.capabilities.voice_cloning {
        return Ok(Some(voice.to_string()));
    }
    resolve_target_voice(plan, voice).map(Some)
}

fn resolve_target_voice(plan: &ExecutionPlan, voice: &str) -> TakokitResult<String> {
    let path = PathBuf::from(voice);
    if path.is_file() || path.is_dir() {
        return Ok(path.display().to_string());
    }
    let reference =
        VoiceProfileStore::new(plan.storage_root.join("voices")).resolve_reference(voice)?;
    Ok(reference.display().to_string())
}

fn run_adapter(
    adapter: &str,
    layout: &AdapterLayout,
    payload: &ManagedAdapterRequest<'_>,
) -> TakokitResult<ManagedAdapterResponse> {
    let encoded = serde_json::to_vec(payload).map_err(|error| {
        TakokitError::Audio(format!("could not encode {adapter} request: {error}"))
    })?;
    let hf_cache = payload.cache_dir.join("huggingface");
    let torch_cache = payload.cache_dir.join("torch");
    let tts_cache = payload.cache_dir.join("coqui");
    let modelscope_cache = payload.cache_dir.join("modelscope");
    for path in [&hf_cache, &torch_cache, &tts_cache, &modelscope_cache] {
        std::fs::create_dir_all(path)
            .map_err(|error| TakokitError::Storage(error.to_string()))?;
    }

    let mut command = Command::new(&layout.python);
    command
        .arg(&layout.script)
        .env("HF_HOME", &hf_cache)
        .env("TORCH_HOME", &torch_cache)
        .env("TTS_HOME", &tts_cache)
        .env("MODELSCOPE_CACHE", &modelscope_cache)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_runner_command(&mut command);
    let mut child = command.spawn().map_err(|error| {
        TakokitError::Audio(format!("could not start {adapter} adapter: {error}"))
    })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| TakokitError::Audio(format!("{adapter} adapter stdin was unavailable")))?
        .write_all(&encoded)
        .map_err(|error| {
            TakokitError::Audio(format!("could not send {adapter} request: {error}"))
        })?;
    let output = child.wait_with_output().map_err(|error| {
        TakokitError::Audio(format!("could not wait for {adapter} adapter: {error}"))
    })?;
    let response: ManagedAdapterResponse =
        serde_json::from_slice(&output.stdout).map_err(|error| {
            TakokitError::Audio(format!(
                "{adapter} returned invalid JSON ({error}): {}{}",
                String::from_utf8_lossy(&output.stdout).trim(),
                stderr_suffix(&output.stderr)
            ))
        })?;
    if !output.status.success() || !response.ok {
        return Err(TakokitError::Audio(format!(
            "{adapter} failed: {}{}",
            response
                .error
                .unwrap_or_else(|| "unknown adapter failure".to_string()),
            stderr_suffix(&output.stderr)
        )));
    }
    Ok(response)
}

fn validate_file_output(
    adapter: &str,
    expected: &Path,
    reported: Option<&Path>,
) -> TakokitResult<()> {
    let reported = reported
        .ok_or_else(|| TakokitError::Audio(format!("{adapter} did not return an output path")))?;
    if reported != expected || !expected.is_file() {
        return Err(TakokitError::Audio(format!(
            "{adapter} did not create the requested WAV at {}",
            expected.display()
        )));
    }
    Ok(())
}

fn output_bytes(path: &Path) -> TakokitResult<u64> {
    std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| TakokitError::Storage(error.to_string()))
}

fn adapter_python_path(adapter_dir: &Path) -> Option<PathBuf> {
    python_candidates(&adapter_dir.join("venv"))
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn runner_python_path(runner_root: &Path) -> Option<PathBuf> {
    python_candidates(&runner_root.join("env").join("venv"))
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn python_candidates(venv: &Path) -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![venv.join("Scripts").join("python.exe")]
    } else {
        vec![
            venv.join("bin").join("python3"),
            venv.join("bin").join("python"),
        ]
    }
}

fn blocked_adapter(model: &str) -> TakokitError {
    inference_missing(format!(
        "{model} has no managed adapter. Takokit will not return a fake result."
    ))
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
