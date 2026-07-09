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
use takokit_package::ExecutionPlan;
use uuid::Uuid;

use super::{SpeechRunner, TranscriptionRunner};

#[derive(Debug, Default, Clone)]
pub struct PythonManagedRunner;

#[derive(Debug, Serialize)]
struct QwenAdapterRequest<'a> {
    input: &'a str,
    voice: Option<&'a str>,
    model_dir: &'a Path,
    output_path: &'a Path,
}

#[derive(Debug, Deserialize)]
struct QwenAdapterResponse {
    ok: bool,
    output_path: Option<PathBuf>,
    bytes: Option<u64>,
    sample_rate: Option<u32>,
    voice: Option<String>,
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
        if plan.model.id == "qwen3-tts" {
            return speak_with_qwen3_tts(plan, request, output_dir);
        }
        Err(blocked_adapter(&plan.model.id))
    }
}

#[async_trait]
impl TranscriptionRunner for PythonManagedRunner {
    async fn transcribe(
        &self,
        plan: &ExecutionPlan,
        _request: TranscriptionRequest,
    ) -> TakokitResult<TranscriptionResponse> {
        Err(blocked_adapter(&plan.model.id))
    }
}

pub fn speak_with_qwen3_tts(
    plan: &ExecutionPlan,
    request: SpeechRequest,
    output_dir: &Path,
) -> TakokitResult<SpeechResponse> {
    if request.input.trim().is_empty() {
        return Err(TakokitError::InvalidRequest(
            "speech input cannot be empty".to_string(),
        ));
    }
    let takokit_root = output_dir.parent().ok_or_else(|| {
        inference_missing(format!(
            "could not infer Takokit root from output directory {}",
            output_dir.display()
        ))
    })?;
    let model_dir = takokit_root.join("models").join("qwen3-tts");
    for required in [
        model_dir.join("model.safetensors"),
        model_dir.join("speech_tokenizer").join("model.safetensors"),
        model_dir.join("config.json"),
    ] {
        if !required.is_file() {
            return Err(inference_missing(format!(
                "Qwen3-TTS artifact is missing at {}; run `takokit pull qwen3-tts`",
                required.display()
            )));
        }
    }
    let runner_root = takokit_root.join("runners").join("python-managed");
    let python = runner_python_path(&runner_root).ok_or_else(|| {
        inference_missing(
            "managed Python runtime is missing; run `takokit runner install takokit-python-managed`",
        )
    })?;
    let adapter = runner_root
        .join("adapters")
        .join("qwen3_tts")
        .join("qwen3_tts.py");
    if !adapter.is_file() {
        return Err(inference_missing(
            "Qwen3-TTS adapter is missing; run `takokit adapter install qwen3_tts`",
        ));
    }

    std::fs::create_dir_all(output_dir)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
    let id = Uuid::new_v4();
    let output_path = output_dir.join(format!("speech-{id}.wav"));
    let payload = serde_json::to_vec(&QwenAdapterRequest {
        input: &request.input,
        voice: request.voice.as_deref().filter(|voice| *voice != "default"),
        model_dir: &model_dir,
        output_path: &output_path,
    })
    .map_err(|error| TakokitError::Audio(format!("could not encode Qwen3-TTS request: {error}")))?;

    let mut child = Command::new(&python)
        .arg(&adapter)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            TakokitError::Audio(format!("could not start Qwen3-TTS adapter: {error}"))
        })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| TakokitError::Audio("Qwen3-TTS adapter stdin was unavailable".to_string()))?
        .write_all(&payload)
        .map_err(|error| {
            TakokitError::Audio(format!("could not send Qwen3-TTS request: {error}"))
        })?;
    let output = child.wait_with_output().map_err(|error| {
        TakokitError::Audio(format!("could not wait for Qwen3-TTS adapter: {error}"))
    })?;
    let response: QwenAdapterResponse =
        serde_json::from_slice(&output.stdout).map_err(|error| {
            TakokitError::Audio(format!(
                "Qwen3-TTS adapter returned invalid JSON ({error}): {}",
                String::from_utf8_lossy(&output.stdout).trim()
            ))
        })?;
    if !output.status.success() || !response.ok {
        return Err(TakokitError::Audio(format!(
            "Qwen3-TTS adapter failed: {}{}",
            response
                .error
                .unwrap_or_else(|| "unknown adapter failure".to_string()),
            stderr_suffix(&output.stderr)
        )));
    }
    let reported_path = response.output_path.ok_or_else(|| {
        TakokitError::Audio("Qwen3-TTS adapter did not return an output path".to_string())
    })?;
    if reported_path != output_path || !output_path.is_file() {
        return Err(TakokitError::Audio(format!(
            "Qwen3-TTS adapter did not create the requested WAV output at {}",
            output_path.display()
        )));
    }
    let bytes = std::fs::metadata(&output_path)
        .map_err(|error| TakokitError::Storage(error.to_string()))?
        .len();
    if response.bytes.is_some_and(|reported| reported != bytes) {
        return Err(TakokitError::Audio(
            "Qwen3-TTS adapter reported a byte count that does not match the WAV output"
                .to_string(),
        ));
    }
    Ok(SpeechResponse {
        id,
        model: plan.model.id.clone(),
        voice: response.voice.or(request.voice),
        engine: "qwen3-tts".to_string(),
        output_path,
        content_type: "audio/wav".to_string(),
        bytes,
        sample_rate: response.sample_rate,
    })
}

fn runner_python_path(runner_root: &Path) -> Option<PathBuf> {
    let venv = runner_root.join("env").join("venv");
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

fn blocked_adapter(model: &str) -> TakokitError {
    inference_missing(format!(
        "{model} has no verified managed adapter yet. Takokit will not return a fake audio result."
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
