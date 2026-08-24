use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use takokit_core::{
    CapabilitiesResponse, CapabilityInfo, CapabilityKind, DaemonBuildIdentity, DaemonMode,
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
    acquire_maintenance_lock, initialize_runner_runtime, install_model_complete,
    install_python_adapter, model_info_from_plan, plan_model, python_adapter_record,
    python_adapter_records, remove_model_complete, resolve_execution_plan, runner_runtime_layout,
    InstallModelOptions, InstalledModelsResponse, LibraryModelManifest, LibraryRunnerManifest,
    ModelPlan, ModelRemovalReport, RemoveModelOptions, RunnerInfo, RunnerLifecycleState,
};

use crate::AppState;

mod error;
mod inference;
mod media;
mod packages;
mod progress;
mod sessions;
mod system;

pub use error::ApiError;
pub use inference::*;
pub use media::*;
pub use packages::*;
pub use progress::*;
pub use sessions::*;
pub use system::*;
