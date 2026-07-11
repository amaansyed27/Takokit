use super::*;

pub async fn voices(State(state): State<AppState>) -> Json<VoicesResponse> {
    Json(VoicesResponse {
        data: state.registry.voices().to_vec(),
    })
}

pub async fn speech(
    State(state): State<AppState>,
    Json(request): Json<SpeechRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let _execution = state
        .register_execution(request.model.clone(), "text_to_speech")
        .await;
    if request.model != "mock-tts" {
        let plan = resolve_execution_plan(
            &state.package_registry,
            &state.installed_registry,
            &request.model,
            CapabilityKind::TextToSpeech,
        )
        .map_err(Into::into)
        .map_err(ApiError)?;

        let response = execute_speech(&plan, request, &state.store.outputs_dir())
            .await
            .map_err(ApiError)?;
        return Ok((StatusCode::OK, Json(response)));
    }

    let response = state
        .tts
        .synthesize(request, &state.store.outputs_dir())
        .await
        .map_err(ApiError)?;

    Ok((StatusCode::OK, Json(response)))
}

pub async fn transcriptions(
    State(state): State<AppState>,
    Json(request): Json<TranscriptionRequest>,
) -> Result<Json<takokit_core::TranscriptionResponse>, ApiError> {
    let model = request
        .model
        .clone()
        .unwrap_or_else(|| "whisper-base".to_string());
    let _execution = state
        .register_execution(model.clone(), "speech_to_text")
        .await;
    let plan = resolve_execution_plan(
        &state.package_registry,
        &state.installed_registry,
        &model,
        CapabilityKind::SpeechToText,
    )
    .map_err(Into::into)
    .map_err(ApiError)?;

    let response = execute_transcription(&plan, request)
        .await
        .map_err(ApiError)?;
    Ok(Json(response))
}

pub async fn clone_voice(
    Json(_request): Json<CloneVoiceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError(TakokitError::NotImplemented {
        feature: "voice cloning",
        reason: "clone adapters require explicit model runner integration",
    }))
}

pub async fn train_voice(
    Json(_request): Json<TrainVoiceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError(TakokitError::NotImplemented {
        feature: "voice training",
        reason: "training jobs and dataset preparation are planned for a later phase",
    }))
}
