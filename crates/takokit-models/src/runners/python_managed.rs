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
use takokit_package::{adapter_for_model, ExecutionPlan};
use uuid::Uuid;

use super::{SpeechRunner, TranscriptionRunner};

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
    output_path: Option<&'a Path>,
    audio_path: Option<&'a Path>,
}

#[derive(Debug, Deserialize)]
struct ManagedAdapterResponse {
    ok: bool,
    output_path: Option<PathBuf>,
    bytes: Option<u64>,
    sample_rate: Option<u32>,
    voice: Option<String>,
    text: Option<String>,
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
    std::fs::create_dir_all(output_dir)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
    let id = Uuid::new_v4();
    let output_path = output_dir.join(format!("speech-{id}.wav"));
    let payload = ManagedAdapterRequest {
        operation: "speech",
        model_id: &plan.model.id,
        model_dir: &plan.storage_root.join("models").join(&plan.model.id),
        cache_dir: &plan.storage_root.join("cache"),
        input: Some(&request.input),
        voice: request.voice.as_deref().filter(|voice| *voice != "default"),
        output_path: Some(&output_path),
        audio_path: None,
    };
    let response = run_adapter(adapter, &layout, &payload)?;
    let reported_path = response.output_path.ok_or_else(|| {
        TakokitError::Audio(format!("{adapter} adapter did not return an output path"))
    })?;
    if reported_path != output_path || !output_path.is_file() {
        return Err(TakokitError::Audio(format!(
            "{adapter} adapter did not create the requested WAV output at {}",
            output_path.display()
        )));
    }
    let bytes = std::fs::metadata(&output_path)
        .map_err(|error| TakokitError::Storage(error.to_string()))?
        .len();
    if response.bytes.is_some_and(|reported| reported != bytes) {
        return Err(TakokitError::Audio(format!(
            "{adapter} adapter reported a byte count that does not match the output"
        )));
    }
    Ok(SpeechResponse {
        id,
        model: plan.model.id.clone(),
        voice: response.voice.or(request.voice),
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
    let payload = ManagedAdapterRequest {
        operation: "transcribe",
        model_id: &plan.model.id,
        model_dir: &plan.storage_root.join("models").join(&plan.model.id),
        cache_dir: &plan.storage_root.join("cache"),
        input: None,
        voice: None,
        output_path: None,
        audio_path: Some(&request.file_path),
    };
    let response = run_adapter(adapter, &layout, &payload)?;
    let text = response.text.filter(|text| !text.trim().is_empty()).ok_or_else(|| {
        TakokitError::Audio(format!("{adapter} adapter returned no transcript text"))
    })?;
    Ok(TranscriptionResponse {
        id: Uuid::new_v4(),
        model: plan.model.id.clone(),
        text,
    })
}

#[derive(Debug)]
struct AdapterLayout {
    python: PathBuf,
    script: PathBuf,
}

fn adapter_layout(plan: &ExecutionPlan, adapter: &str) -> TakokitResult<AdapterLayout> {
    let runner_root = plan
        .storage_root
        .join("runners")
        .join("python-managed");
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

fn run_adapter(
    adapter: &str,
    layout: &AdapterLayout,
    payload: &ManagedAdapterRequest<'_>,
) -> TakokitResult<ManagedAdapterResponse> {
    let encoded = serde_json::to_vec(payload).map_err(|error| {
        TakokitError::Audio(format!("could not encode {adapter} request: {error}"))
    })?;
    let mut child = Command::new(&layout.python)
        .arg(&layout.script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
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
    let response: ManagedAdapterResponse = serde_json::from_slice(&output.stdout).map_err(|error| {
        TakokitError::Audio(format!(
            "{adapter} adapter returned invalid JSON ({error}): {}{}",
            String::from_utf8_lossy(&output.stdout).trim(),
            stderr_suffix(&output.stderr)
        ))
    })?;
    if !output.status.success() || !response.ok {
        return Err(TakokitError::Audio(format!(
            "{adapter} adapter failed: {}{}",
            response
                .error
                .unwrap_or_else(|| "unknown adapter failure".to_string()),
            stderr_suffix(&output.stderr)
        )));
    }
    Ok(response)
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
        vec![venv.join("bin").join("python3"), venv.join("bin").join("python")]
    }
}

fn blocked_adapter(model: &str) -> TakokitError {
    inference_missing(format!(
        "{model} has no verified managed adapter yet. Takokit will not return a fake result."
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
