use super::*;
use crate::RequestWorkspace;
use axum::http::HeaderMap;
use takokit_core::{
    CloneVoiceResponse, CreateVoiceProfileRequest, NewSessionEvent, SessionEventState, SessionTask,
    VoiceInfo,
};
use takokit_store::VoiceProfileStore;

pub async fn voices(State(state): State<AppState>) -> Json<VoicesResponse> {
    let mut data = state.registry.voices().to_vec();
    if let Ok(profiles) = VoiceProfileStore::new(state.store.voices_dir()).list() {
        data.extend(profiles.into_iter().map(|profile| VoiceInfo {
            id: profile.id,
            name: profile.name,
            source: "local-profile".to_string(),
            model_id: Some(profile.model_id),
            consent_required: false,
        }));
    }
    Json(VoicesResponse { data })
}

pub async fn speech(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SpeechRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let workspace = RequestWorkspace::from_headers(&headers, "Speech").map_err(ApiError)?;
    let session_id = workspace.session_id();
    let model = request.model.clone();
    let input = request.input.clone();
    let _execution = state
        .register_execution(model.clone(), "text_to_speech")
        .await;

    let result: Result<takokit_core::SpeechResponse, TakokitError> = if request.model != "mock-tts"
    {
        match resolve_execution_plan(
            &state.package_registry,
            &state.installed_registry,
            &request.model,
            CapabilityKind::TextToSpeech,
        )
        .map_err(TakokitError::from)
        {
            Ok(plan) => execute_speech(&plan, request, &workspace.outputs_dir()).await,
            Err(error) => Err(error),
        }
    } else {
        state
            .tts
            .synthesize(request, &workspace.outputs_dir())
            .await
    };

    match result {
        Ok(response) => {
            workspace
                .store
                .append_event(
                    session_id,
                    NewSessionEvent {
                        task: SessionTask::TextToSpeech,
                        state: SessionEventState::Completed,
                        model: Some(model),
                        input: Some(input),
                        source_path: None,
                        output_path: Some(response.output_path.clone()),
                        text: None,
                        message: Some(format!(
                            "Generated {} bytes using {}",
                            response.bytes, response.engine
                        )),
                    },
                )
                .map_err(ApiError)?;
            Ok((StatusCode::OK, Json(response)))
        }
        Err(error) => {
            let _ = workspace.store.append_event(
                session_id,
                NewSessionEvent {
                    task: SessionTask::TextToSpeech,
                    state: SessionEventState::Failed,
                    model: Some(model),
                    input: Some(input),
                    source_path: None,
                    output_path: None,
                    text: None,
                    message: Some(error.to_string()),
                },
            );
            Err(ApiError(error))
        }
    }
}

pub async fn transcriptions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TranscriptionRequest>,
) -> Result<Json<takokit_core::TranscriptionResponse>, ApiError> {
    let workspace = RequestWorkspace::from_headers(&headers, "Transcription").map_err(ApiError)?;
    let session_id = workspace.session_id();
    let source_path = request.file_path.clone();
    let model = request
        .model
        .clone()
        .unwrap_or_else(|| "whisper-base".to_string());
    let _execution = state
        .register_execution(model.clone(), "speech_to_text")
        .await;
    let result: Result<takokit_core::TranscriptionResponse, TakokitError> =
        match resolve_execution_plan(
            &state.package_registry,
            &state.installed_registry,
            &model,
            CapabilityKind::SpeechToText,
        )
        .map_err(TakokitError::from)
        {
            Ok(plan) => execute_transcription(&plan, request).await,
            Err(error) => Err(error),
        };

    match result {
        Ok(response) => {
            let filename = format!("transcript-{}.txt", response.id);
            let output_path = workspace
                .store
                .write_text_output(session_id, &filename, &response.text)
                .map_err(ApiError)?;
            workspace
                .store
                .append_event(
                    session_id,
                    NewSessionEvent {
                        task: SessionTask::SpeechToText,
                        state: SessionEventState::Completed,
                        model: Some(model),
                        input: None,
                        source_path: Some(source_path),
                        output_path: Some(output_path),
                        text: Some(response.text.clone()),
                        message: Some("Transcript saved in the project session.".to_string()),
                    },
                )
                .map_err(ApiError)?;
            Ok(Json(response))
        }
        Err(error) => {
            let _ = workspace.store.append_event(
                session_id,
                NewSessionEvent {
                    task: SessionTask::SpeechToText,
                    state: SessionEventState::Failed,
                    model: Some(model),
                    input: None,
                    source_path: Some(source_path),
                    output_path: None,
                    text: None,
                    message: Some(error.to_string()),
                },
            );
            Err(ApiError(error))
        }
    }
}

pub async fn clone_voice(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateVoiceProfileRequest>,
) -> Result<Json<CloneVoiceResponse>, ApiError> {
    let workspace = RequestWorkspace::from_headers(&headers, "Voice cloning").map_err(ApiError)?;
    let model = state
        .package_registry
        .model(&request.model)
        .map_err(TakokitError::from)
        .map_err(ApiError)?;
    if !model.capabilities.voice_cloning {
        return Err(ApiError(TakokitError::InvalidRequest(format!(
            "model {} does not support voice cloning",
            request.model
        ))));
    }
    let plan = plan_model(
        &state.package_registry,
        &state.installed_registry,
        &request.model,
    )
    .map_err(TakokitError::from)
    .map_err(ApiError)?;
    if !plan.executable {
        return Err(ApiError(TakokitError::Resolution {
            code: ErrorCode::RuntimeNotReady,
            message: format!(
                "model {} is not ready: {}. Next: {}",
                request.model,
                if plan.missing.is_empty() {
                    "runtime setup is incomplete".to_string()
                } else {
                    plan.missing.join("; ")
                },
                plan.next_command
            ),
        }));
    }

    let profile = VoiceProfileStore::new(state.store.voices_dir())
        .create(
            &request.name,
            &request.model,
            &request.sample_path,
            request.consent_affirmed,
            request.consent_note,
        )
        .map_err(ApiError)?;
    workspace
        .store
        .append_event(
            workspace.session_id(),
            NewSessionEvent {
                task: SessionTask::VoiceCloning,
                state: SessionEventState::Completed,
                model: Some(profile.model_id.clone()),
                input: Some(profile.name.clone()),
                source_path: Some(request.sample_path),
                output_path: Some(profile.sample_path.clone()),
                text: None,
                message: Some(format!(
                    "Created reusable voice profile {}. Use voice ID {} with compatible models.",
                    profile.name, profile.id
                )),
            },
        )
        .map_err(ApiError)?;
    Ok(Json(CloneVoiceResponse { data: profile }))
}

pub async fn train_voice(
    headers: HeaderMap,
    Json(request): Json<TrainVoiceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let workspace = RequestWorkspace::from_headers(&headers, "Voice training").map_err(ApiError)?;
    let error = TakokitError::NotImplemented {
        feature: "voice training",
        reason: "training jobs and dataset preparation are not yet release-ready",
    };
    let _ = workspace.store.append_event(
        workspace.session_id(),
        NewSessionEvent {
            task: SessionTask::VoiceTraining,
            state: SessionEventState::Failed,
            model: None,
            input: Some(request.name),
            source_path: Some(request.samples_path),
            output_path: None,
            text: None,
            message: Some(error.to_string()),
        },
    );
    Err(ApiError(error))
}
