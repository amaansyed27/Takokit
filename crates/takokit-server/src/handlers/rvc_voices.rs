use super::*;
use axum::extract::{Path as AxumPath, Query};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use takokit_core::{
    AddRvcSamplesRequest, CreateRvcVoiceRequest, ExportRvcVoiceRequest, ImportRvcPackageRequest,
    ImportRvcVoiceRequest, RvcTrainingConfig, RvcTrainingJob, RvcVoiceDetail,
    SelectRvcCheckpointRequest, SetRvcSampleIncludedRequest, StartRvcTrainingRequest,
    TestRvcVoiceRequest, VerifyRvcPackageRequest,
};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RemoveVoiceQuery {
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ExportVoiceBody {
    output: PathBuf,
    #[serde(default)]
    sign: bool,
    #[serde(default)]
    include_reference: bool,
}

fn scrub_job_internals(value: &mut Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    for field in ["owner_pid", "child_pid", "log_path", "cancellation_requested"] {
        object.remove(field);
    }
}

fn public_job(job: &RvcTrainingJob) -> Value {
    let mut value = serde_json::to_value(job).expect("RVC training job serialization");
    scrub_job_internals(&mut value);
    value
}

fn public_optional_job(job: Option<RvcTrainingJob>) -> Value {
    job.as_ref().map(public_job).unwrap_or(Value::Null)
}

fn public_detail(detail: &RvcVoiceDetail) -> Value {
    let mut value = serde_json::to_value(detail).expect("RVC voice detail serialization");
    if let Some(active_job) = value.get_mut("active_job") {
        scrub_job_internals(active_job);
    }
    value
}

pub async fn rvc_voice_list(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let items = state.rvc_voices.list().map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_voices","data":items}),
    ))
}

pub async fn rvc_voice_create(
    State(state): State<AppState>,
    Json(request): Json<CreateRvcVoiceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let project = state.rvc_voices.create(request).map_err(ApiError)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"kind":"rvc_voice","data":project})),
    ))
}

pub async fn rvc_voice_presets(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "kind":"rvc_training_presets",
        "data":state.rvc_voices.presets()
    }))
}

pub async fn rvc_voice_show(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let detail = state.rvc_voices.show(&voice).map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_voice_detail","data":public_detail(&detail)}),
    ))
}

pub async fn rvc_voice_remove(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Query(query): Query<RemoveVoiceQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let report = state
        .rvc_voices
        .remove(&voice, query.dry_run)
        .map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_voice_removal","data":report}),
    ))
}

pub async fn rvc_sample_add(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Json(request): Json<AddRvcSamplesRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let samples = state
        .rvc_voices
        .add_samples(&voice, request)
        .map_err(ApiError)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"kind":"rvc_samples","data":samples})),
    ))
}

pub async fn rvc_sample_list(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let samples = state.rvc_voices.samples(&voice).map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_samples","data":samples}),
    ))
}

pub async fn rvc_sample_update(
    State(state): State<AppState>,
    AxumPath((voice, sample)): AxumPath<(String, Uuid)>,
    Json(request): Json<SetRvcSampleIncludedRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sample = state
        .rvc_voices
        .set_sample_included(&voice, sample, request.included)
        .map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_sample","data":sample}),
    ))
}

pub async fn rvc_sample_remove(
    State(state): State<AppState>,
    AxumPath((voice, sample)): AxumPath<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    state
        .rvc_voices
        .remove_sample(&voice, sample)
        .map_err(ApiError)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn rvc_dataset_inspect(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let inspection = state.rvc_voices.inspect_dataset(&voice).map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_dataset_inspection","data":inspection}),
    ))
}

pub async fn rvc_dataset_clear(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    state.rvc_voices.clear_prepared(&voice).map_err(ApiError)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn rvc_preflight(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Json(config): Json<RvcTrainingConfig>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let report = state.rvc_voices.preflight(&voice, config).map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_hardware_preflight","data":report}),
    ))
}

pub async fn rvc_prepare(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Json(request): Json<StartRvcTrainingRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let job = state.rvc_voices.prepare(&voice, request).map_err(ApiError)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "kind":"rvc_training_job",
            "data":public_job(&job)
        })),
    ))
}

pub async fn rvc_train(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Json(request): Json<StartRvcTrainingRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let job = state
        .rvc_voices
        .start_training(&voice, request)
        .map_err(ApiError)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "kind":"rvc_training_job",
            "data":public_job(&job)
        })),
    ))
}

pub async fn rvc_train_recover(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let job = state
        .rvc_voices
        .recover_training(&voice)
        .map_err(ApiError)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "kind":"rvc_training_job",
            "data":public_job(&job)
        })),
    ))
}

pub async fn rvc_train_status(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let job = state
        .rvc_voices
        .training_status(&voice)
        .map_err(ApiError)?;
    Ok(Json(serde_json::json!({
        "kind":"rvc_training_job",
        "data":public_optional_job(job)
    })))
}

pub async fn rvc_train_logs(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let text = state
        .rvc_voices
        .training_logs(
            &voice,
            query
                .max_bytes
                .unwrap_or(256 * 1024)
                .min(2 * 1024 * 1024),
        )
        .map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_training_logs","data":{"text":text}}),
    ))
}

pub async fn rvc_train_cancel(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let job = state
        .rvc_voices
        .cancel_training(&voice)
        .map_err(ApiError)?;
    Ok(Json(serde_json::json!({
        "kind":"rvc_training_job",
        "data":public_job(&job)
    })))
}

pub async fn rvc_checkpoints(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let checkpoints = state.rvc_voices.checkpoints(&voice).map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_checkpoints","data":checkpoints}),
    ))
}

pub async fn rvc_indexes(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let indexes = state.rvc_voices.indexes(&voice).map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_indexes","data":indexes}),
    ))
}

pub async fn rvc_select_checkpoint(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Json(request): Json<SelectRvcCheckpointRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let managed = state
        .rvc_voices
        .select_checkpoint(&voice, request)
        .map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"managed_rvc_voice","data":managed}),
    ))
}

pub async fn rvc_import(
    State(state): State<AppState>,
    Json(request): Json<ImportRvcVoiceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let project = state
        .rvc_voices
        .import_existing(request)
        .map_err(ApiError)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"kind":"rvc_voice","data":project})),
    ))
}

pub async fn rvc_export(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Json(body): Json<ExportVoiceBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let output = state
        .rvc_voices
        .export_package(
            &voice,
            ExportRvcVoiceRequest {
                output: body.output,
                sign: body.sign,
                include_reference: body.include_reference,
            },
        )
        .map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_voice_package","data":{"path":output}}),
    ))
}

pub async fn rvc_package_verify(
    State(state): State<AppState>,
    Json(request): Json<VerifyRvcPackageRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let report = state
        .rvc_voices
        .verify_package(&request.package)
        .map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_package_verification","data":report}),
    ))
}

pub async fn rvc_package_import(
    State(state): State<AppState>,
    Json(request): Json<ImportRvcPackageRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let project = state
        .rvc_voices
        .import_package(request)
        .map_err(ApiError)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"kind":"rvc_voice","data":project})),
    ))
}

pub async fn rvc_test_voice(
    State(state): State<AppState>,
    AxumPath(voice): AxumPath<String>,
    Json(request): Json<TestRvcVoiceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let output_dir = request
        .workspace_root
        .as_ref()
        .map(|root| root.join("outputs"))
        .unwrap_or_else(|| state.store.root().join("outputs"));
    let response = state
        .rvc_voices
        .test_voice(
            &voice,
            request.input,
            &state.package_registry,
            &state.installed_registry,
            &output_dir,
        )
        .await
        .map_err(ApiError)?;
    Ok(Json(
        serde_json::json!({"kind":"rvc_voice_test","data":response}),
    ))
}
