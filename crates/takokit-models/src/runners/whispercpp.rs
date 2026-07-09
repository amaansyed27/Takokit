use async_trait::async_trait;
use takokit_core::{TakokitResult, TranscriptionRequest, TranscriptionResponse};
use takokit_package::ExecutionPlan;

use super::{runner_not_implemented, TranscriptionRunner};

#[derive(Debug, Default, Clone)]
pub struct WhisperCppRunner;

#[async_trait]
impl TranscriptionRunner for WhisperCppRunner {
    async fn transcribe(
        &self,
        plan: &ExecutionPlan,
        _request: TranscriptionRequest,
    ) -> TakokitResult<TranscriptionResponse> {
        Err(runner_not_implemented(format!(
            "Runner {} contract resolved, but transcription execution is not implemented yet.",
            plan.runner.id
        )))
    }
}
