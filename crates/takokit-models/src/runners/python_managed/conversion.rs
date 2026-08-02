use std::path::{Path, PathBuf};
use takokit_core::{
    RvcCheckpointMetadata, TakokitError, TakokitResult, VoiceConversionExecutionStatus,
    VoiceConversionQualityStatus, VoiceConversionRequest, VoiceConversionResponse,
};
use takokit_package::{runtime_model_id, ExecutionPlan};
use uuid::Uuid;

use super::{
    adapter_id, adapter_layout, output_bytes, resolve_target_voice, run_adapter,
    validate_file_output, ManagedAdapterRequest,
};

pub(super) fn convert_with_adapter(
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
    request
        .settings()
        .validate()
        .map_err(TakokitError::InvalidRequest)?;

    let adapter = adapter_id(plan)?;
    let layout = adapter_layout(plan, adapter)?;
    let target_voice = resolve_target_voice(plan, &request.target_voice)?;
    std::fs::create_dir_all(output_dir)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;

    let id = Uuid::new_v4();
    let output_path = output_dir.join(format!("conversion-{id}.wav"));
    let model_dir = plan.storage_root.join("models").join(&plan.model.id);
    let cache_dir = plan.storage_root.join("cache");
    let runtime_model = runtime_model_id(&plan.model);
    let payload = ManagedAdapterRequest {
        operation: "convert",
        model_id: runtime_model,
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
        f0_method: Some(request.f0_method.as_str()),
        pitch_shift: Some(request.pitch_shift),
        index_rate: Some(request.index_rate),
        rms_mix_rate: Some(request.rms_mix_rate),
        protect: Some(request.protect),
        filter_radius: Some(request.filter_radius),
        epochs: None,
    };
    let response = run_adapter(adapter, &layout, &payload)?;
    validate_file_output(adapter, &output_path, response.output_path.as_deref())?;

    let effective_settings = response
        .effective_settings
        .unwrap_or_else(|| request.settings());
    let checkpoint = if adapter == "rvc" {
        response.checkpoint.ok_or_else(|| {
            TakokitError::Audio(
                "RVC completed without returning checkpoint and index evidence".to_string(),
            )
        })?
    } else {
        non_rvc_target_metadata(&target_voice)
    };

    Ok(VoiceConversionResponse {
        id,
        model: plan.model.id.clone(),
        target_voice: request.target_voice,
        output_path: output_path.clone(),
        content_type: "audio/wav".to_string(),
        bytes: output_bytes(&output_path)?,
        sample_rate: response.sample_rate,
        execution_status: VoiceConversionExecutionStatus::Passed,
        quality_status: VoiceConversionQualityStatus::NotEvaluated,
        quality_review_required: true,
        quality_notice: "Execution produced a valid WAV. Listen to the source, target reference and output before recording a quality pass.".to_string(),
        effective_settings,
        checkpoint,
    })
}

fn non_rvc_target_metadata(target: &str) -> RvcCheckpointMetadata {
    let path = PathBuf::from(target);
    let bytes = std::fs::metadata(&path)
        .map(|metadata| metadata.len())
        .unwrap_or_default();
    RvcCheckpointMetadata {
        checkpoint_path: path,
        checkpoint_sha256: "not-applicable".to_string(),
        checkpoint_bytes: bytes,
        index_path: None,
        index_sha256: None,
        index_bytes: None,
        pairing_status: "not_applicable".to_string(),
        target_reference_path: Some(PathBuf::from(target)),
        quality_baseline_ready: true,
    }
}
