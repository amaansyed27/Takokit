use async_trait::async_trait;
use std::path::Path;
use takokit_core::{
    SpeechRequest, SpeechResponse, TakokitResult, TranscriptionRequest, TranscriptionResponse,
};
use takokit_package::ExecutionPlan;

use super::{onnx_not_implemented, SpeechRunner, TranscriptionRunner};

#[derive(Debug, Default, Clone)]
pub struct OnnxRunner;

#[async_trait]
impl SpeechRunner for OnnxRunner {
    async fn speak(
        &self,
        _plan: &ExecutionPlan,
        _request: SpeechRequest,
        _output_dir: &Path,
    ) -> TakokitResult<SpeechResponse> {
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
