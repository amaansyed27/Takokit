use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use takokit_core::{
    CapabilitiesResponse, CapabilityInfo, CapabilityKind, DaemonIdentity, DaemonMode,
    DaemonShutdownRequest, ErrorCode, HealthResponse, ModelDetailResponse, ModelInstallReport,
    ModelsResponse, ProcessInfo, PullModelRequest, PullModelResponse, PullRunnerRequest,
    RunnerDetailResponse, RunnersResponse, SpeechRequest, TakokitError, TrainVoiceRequest,
    TrainVoiceResponse, TranscriptionRequest, VoiceConversionRequest, VoiceConversionResponse,
    VoicesResponse,
};
use takokit_models::{
    execute_speech, execute_transcription, execute_voice_conversion, execute_voice_training,
    TextToSpeechEngine,
};
use takokit_package::{
    initialize_runner_runtime, install_model_complete, install_python_adapter,
    model_info_from_plan, plan_model, python_adapter_record, python_adapter_records,
    resolve_execution_plan, runner_runtime_layout, InstallModelOptions, LibraryModelManifest,
    LibraryRunnerManifest, ModelPlan, RunnerInfo, RunnerLifecycleState,
};

use crate::AppState;

mod error;
mod inference;
mod packages;
mod sessions;
mod system;

pub use error::ApiError;
pub use inference::*;
pub use packages::*;
pub use sessions::*;
pub use system::*;
