pub mod onnx;
pub mod whispercpp;

use async_trait::async_trait;
use std::path::Path;
use takokit_core::{
    ErrorCode, SpeechRequest, SpeechResponse, TakokitError, TakokitResult, TranscriptionRequest,
    TranscriptionResponse,
};
use takokit_package::{ExecutionPlan, RunnerKind};

use self::onnx::OnnxRunner;
use self::whispercpp::WhisperCppRunner;

#[async_trait]
pub trait SpeechRunner: Send + Sync {
    async fn speak(
        &self,
        plan: &ExecutionPlan,
        request: SpeechRequest,
        output_dir: &Path,
    ) -> TakokitResult<SpeechResponse>;
}

#[async_trait]
pub trait TranscriptionRunner: Send + Sync {
    async fn transcribe(
        &self,
        plan: &ExecutionPlan,
        request: TranscriptionRequest,
    ) -> TakokitResult<TranscriptionResponse>;
}

pub async fn execute_speech(
    plan: &ExecutionPlan,
    request: SpeechRequest,
    output_dir: &Path,
) -> TakokitResult<SpeechResponse> {
    match plan.runner.kind {
        RunnerKind::Onnx => OnnxRunner.speak(plan, request, output_dir).await,
        _ => Err(runner_not_implemented(format!(
            "Runner {} contract resolved, but speech execution is not implemented yet.",
            plan.runner.id
        ))),
    }
}

pub async fn execute_transcription(
    plan: &ExecutionPlan,
    request: TranscriptionRequest,
) -> TakokitResult<TranscriptionResponse> {
    match plan.runner.kind {
        RunnerKind::Onnx => OnnxRunner.transcribe(plan, request).await,
        RunnerKind::Whispercpp => WhisperCppRunner.transcribe(plan, request).await,
        _ => Err(runner_not_implemented(format!(
            "Runner {} contract resolved, but transcription execution is not implemented yet.",
            plan.runner.id
        ))),
    }
}

pub(crate) fn onnx_not_implemented() -> TakokitError {
    runner_not_implemented(
        "ONNX runner contract resolved, but real ONNX execution is not implemented yet.",
    )
}

pub(crate) fn piper_not_implemented() -> TakokitError {
    runner_not_implemented(
        "Piper artifacts and config resolved, but phonemizer/token preparation and ONNX session execution are not implemented yet.",
    )
}

fn runner_not_implemented(message: impl Into<String>) -> TakokitError {
    TakokitError::Resolution {
        code: ErrorCode::InferenceNotImplemented,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use takokit_core::{CapabilityKind, ErrorCode, SpeechRequest, TakokitError};
    use takokit_package::{resolve_execution_plan, InstalledRegistry, PackageRegistry};

    use super::execute_speech;

    #[tokio::test]
    async fn onnx_speech_executor_returns_inference_not_implemented() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));
        let model = registry.model("kokoro").expect("model");
        let runner = registry.runner("takokit-onnx").expect("runner");
        installed.install_model(&model).expect("install model");
        installed.install_runner(&runner).expect("install runner");

        let plan = resolve_execution_plan(
            &registry,
            &installed,
            "kokoro",
            CapabilityKind::TextToSpeech,
        )
        .expect("execution plan");
        let error = execute_speech(
            &plan,
            SpeechRequest {
                model: "kokoro".to_string(),
                input: "Hello".to_string(),
                voice: Some("default".to_string()),
                response_format: Some("wav".to_string()),
            },
            temp.path(),
        )
        .await
        .expect_err("onnx scaffold is not implemented");

        assert!(matches!(
            error,
            TakokitError::Resolution {
                code: ErrorCode::InferenceNotImplemented,
                message
            } if message == "ONNX runner contract resolved, but real ONNX execution is not implemented yet."
        ));
    }

    fn write_test_registry(root: &Path) {
        let models = root.join("models");
        let runners = root.join("runners");
        std::fs::create_dir_all(&models).expect("models dir");
        std::fs::create_dir_all(&runners).expect("runners dir");
        std::fs::write(
            models.join("kokoro.toml"),
            r#"
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
"#,
        )
        .expect("model toml");
        std::fs::write(
            runners.join("takokit-onnx.toml"),
            r#"
id = "takokit-onnx"
name = "Takokit ONNX Runner"
version = "0.1.0"
kind = "onnx"
platforms = ["windows-x64", "linux-x64", "macos-arm64"]
description = "Native ONNX runner for CPU-friendly models."
"#,
        )
        .expect("runner toml");
    }
}
