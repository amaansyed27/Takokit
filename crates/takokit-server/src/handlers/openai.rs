use super::*;
use axum::extract::rejection::JsonRejection;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::path::{Path as FsPath, PathBuf};
use takokit_core::{CapabilityKind, SpeechRequest, TakokitError, TranscriptionRequest};
use takokit_models::{execute_speech, execute_transcription};
use takokit_package::{plan_model, resolve_execution_plan};
use uuid::Uuid;

const MAX_AUDIO_UPLOAD_BYTES: usize = 25 * 1024 * 1024;
const MAX_SPEECH_INPUT_CHARS: usize = 4096;

pub async fn openapi() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Takokit Local Audio API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "OpenAI-compatible audio endpoints under /v1 and Takokit-native endpoints under /api/v1."
        },
        "servers": [{ "url": "http://127.0.0.1:5050" }],
        "paths": {
            "/health": { "get": { "operationId": "health", "tags": ["Local"] } },
            "/v1/models": { "get": { "operationId": "listModels", "tags": ["OpenAI compatible"] } },
            "/v1/models/{model}": { "get": { "operationId": "retrieveModel", "tags": ["OpenAI compatible"] } },
            "/v1/audio/speech": { "post": { "operationId": "createSpeech", "tags": ["OpenAI compatible"] } },
            "/v1/audio/transcriptions": { "post": { "operationId": "createTranscription", "tags": ["OpenAI compatible"] } },
            "/api/v1/status": { "get": { "operationId": "takokitStatus", "tags": ["Takokit native"] } },
            "/api/v1/models": { "get": { "operationId": "takokitModels", "tags": ["Takokit native"] } },
            "/api/v1/audio/speech": { "post": { "operationId": "takokitSpeech", "tags": ["Takokit native"] } },
            "/api/v1/audio/transcriptions": { "post": { "operationId": "takokitTranscription", "tags": ["Takokit native"] } },
            "/api/v1/audio/conversions": { "post": { "operationId": "takokitConversion", "tags": ["Takokit native"] } },
            "/api/v1/voices/clone": { "post": { "operationId": "takokitVoiceClone", "tags": ["Takokit native"] } },
            "/api/v1/voices/rvc": { "get": { "operationId": "takokitRvcVoices", "tags": ["Takokit native"] } }
        }
    }))
}

#[derive(Debug, Serialize)]
pub struct OpenAiModel {
    id: String,
    object: &'static str,
    created: u64,
    owned_by: &'static str,
}

#[derive(Debug, Serialize)]
pub struct OpenAiModelList {
    object: &'static str,
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenAiSpeechRequest {
    model: String,
    input: String,
    voice: String,
    #[serde(default)]
    instructions: Option<String>,
    #[serde(default = "wav_format")]
    response_format: String,
    #[serde(default = "normal_speed")]
    speed: f32,
}

fn wav_format() -> String {
    "wav".to_string()
}

fn normal_speed() -> f32 {
    1.0
}

pub async fn openai_models(
    State(state): State<AppState>,
) -> Result<Json<OpenAiModelList>, OpenAiError> {
    let manifests = state
        .package_registry
        .models()
        .map_err(TakokitError::from)?;
    let mut data = Vec::new();
    for manifest in manifests {
        if !(manifest.capabilities.tts || manifest.capabilities.stt) {
            continue;
        }
        let plan = plan_model(
            &state.package_registry,
            &state.installed_registry,
            &manifest.id,
        )
        .map_err(TakokitError::from)?;
        if plan.executable {
            data.push(model_object(manifest.id));
        }
    }
    Ok(Json(OpenAiModelList {
        object: "list",
        data,
    }))
}

pub async fn openai_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OpenAiModel>, OpenAiError> {
    let manifest = state
        .package_registry
        .model(&id)
        .map_err(TakokitError::from)?;
    let plan = plan_model(
        &state.package_registry,
        &state.installed_registry,
        &manifest.id,
    )
    .map_err(TakokitError::from)?;
    if !plan.executable || !(manifest.capabilities.tts || manifest.capabilities.stt) {
        return Err(OpenAiError::not_found(
            "model",
            format!("Model '{id}' is not installed and executable for the audio API."),
        ));
    }
    Ok(Json(model_object(manifest.id)))
}

fn model_object(id: String) -> OpenAiModel {
    OpenAiModel {
        id,
        object: "model",
        created: 0,
        owned_by: "takokit",
    }
}

pub async fn openai_speech(
    State(state): State<AppState>,
    payload: Result<Json<OpenAiSpeechRequest>, JsonRejection>,
) -> Result<Response, OpenAiError> {
    let Json(request) = payload.map_err(|error| {
        OpenAiError::invalid(
            None,
            format!("Malformed speech request: {}", error.body_text()),
        )
    })?;
    if request.input.chars().count() > MAX_SPEECH_INPUT_CHARS {
        return Err(OpenAiError::invalid(
            Some("input"),
            "Speech input must be at most 4096 characters.",
        ));
    }
    if request.response_format != "wav" {
        return Err(OpenAiError::invalid(
            Some("response_format"),
            "Takokit currently supports only response_format='wav'.",
        ));
    }
    if (request.speed - 1.0).abs() > f32::EPSILON {
        return Err(OpenAiError::invalid(
            Some("speed"),
            "Takokit currently supports only speed=1.0.",
        ));
    }

    let plan = resolve_execution_plan(
        &state.package_registry,
        &state.installed_registry,
        &request.model,
        CapabilityKind::TextToSpeech,
    )
    .map_err(TakokitError::from)?;
    let temp = ApiTempDir::create(state.store.root(), "speech")?;
    let result = execute_speech(
        &plan,
        SpeechRequest {
            model: request.model,
            input: request.input,
            voice: Some(request.voice),
            response_format: Some("wav".to_string()),
            language: None,
            instruction: request.instructions,
            reference_text: None,
        },
        temp.path(),
    )
    .await?;
    let bytes = tokio::fs::read(&result.output_path)
        .await
        .map_err(|error| {
            OpenAiError::runtime(format!("Could not read generated audio: {error}"))
        })?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "audio/wav")
        .header(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&bytes.len().to_string()).unwrap(),
        )
        .body(Body::from(bytes))
        .map_err(|error| OpenAiError::runtime(error.to_string()))
}

pub async fn openai_transcription(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Response, OpenAiError> {
    let temp = ApiTempDir::create(state.store.root(), "transcription")?;
    let mut file_path = None;
    let mut model = None;
    let mut response_format = "json".to_string();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| OpenAiError::invalid(None, "Malformed multipart form data."))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                if file_path.is_some() {
                    return Err(OpenAiError::invalid(
                        Some("file"),
                        "Only one file is allowed.",
                    ));
                }
                let content_type = field.content_type().map(str::to_string);
                let filename = field.file_name().map(str::to_string);
                let extension = safe_audio_extension(content_type.as_deref(), filename.as_deref())?;
                let bytes = field.bytes().await.map_err(|_| {
                    OpenAiError::invalid(Some("file"), "Could not read audio upload.")
                })?;
                if bytes.is_empty() {
                    return Err(OpenAiError::invalid(Some("file"), "Audio upload is empty."));
                }
                if bytes.len() > MAX_AUDIO_UPLOAD_BYTES {
                    return Err(OpenAiError::too_large());
                }
                let path = temp.path().join(format!("upload.{extension}"));
                tokio::fs::write(&path, &bytes).await.map_err(|error| {
                    OpenAiError::runtime(format!("Could not stage audio: {error}"))
                })?;
                file_path = Some(path);
            }
            "model" => model = Some(field.text().await.map_err(bad_multipart)?),
            "response_format" => response_format = field.text().await.map_err(bad_multipart)?,
            "temperature" => {
                let value = field.text().await.map_err(bad_multipart)?;
                if value.parse::<f32>().ok().is_none_or(|value| value != 0.0) {
                    return Err(OpenAiError::invalid(
                        Some("temperature"),
                        "Takokit currently supports only temperature=0.",
                    ));
                }
            }
            "language" | "prompt" => {
                let value = field.text().await.map_err(bad_multipart)?;
                if !value.trim().is_empty() {
                    return Err(OpenAiError::invalid(
                        Some(name.as_str()),
                        format!("The '{name}' parameter is not supported by Takokit yet."),
                    ));
                }
            }
            _ => {
                return Err(OpenAiError::invalid(
                    Some(name.as_str()),
                    format!("Unsupported parameter '{name}'."),
                ))
            }
        }
    }
    if !matches!(response_format.as_str(), "json" | "text") {
        return Err(OpenAiError::invalid(
            Some("response_format"),
            "Takokit transcription supports response_format='json' or 'text'.",
        ));
    }
    let file_path =
        file_path.ok_or_else(|| OpenAiError::invalid(Some("file"), "File is required."))?;
    let model = model.ok_or_else(|| OpenAiError::invalid(Some("model"), "Model is required."))?;
    let plan = resolve_execution_plan(
        &state.package_registry,
        &state.installed_registry,
        &model,
        CapabilityKind::SpeechToText,
    )
    .map_err(TakokitError::from)?;
    let result = execute_transcription(
        &plan,
        TranscriptionRequest {
            file_path,
            model: Some(model),
        },
    )
    .await?;
    if response_format == "text" {
        Ok((
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            result.text,
        )
            .into_response())
    } else {
        Ok(Json(serde_json::json!({ "text": result.text })).into_response())
    }
}

fn bad_multipart(_: axum::extract::multipart::MultipartError) -> OpenAiError {
    OpenAiError::invalid(None, "Malformed multipart form data.")
}

fn safe_audio_extension(
    content_type: Option<&str>,
    filename: Option<&str>,
) -> Result<&'static str, OpenAiError> {
    let mime_extension = match content_type.unwrap_or("").split(';').next().unwrap_or("") {
        "audio/wav" | "audio/x-wav" | "audio/wave" => Some("wav"),
        "audio/mpeg" | "audio/mp3" => Some("mp3"),
        "audio/flac" => Some("flac"),
        "audio/ogg" => Some("ogg"),
        "audio/mp4" | "video/mp4" => Some("m4a"),
        "audio/webm" | "video/webm" => Some("webm"),
        "application/octet-stream" | "" => None,
        _ => {
            return Err(OpenAiError::invalid(
                Some("file"),
                "Unsupported audio MIME type.",
            ))
        }
    };
    if let Some(extension) = mime_extension {
        return Ok(extension);
    }
    let extension = filename
        .and_then(|name| FsPath::new(name).extension())
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "wav" => Ok("wav"),
        "mp3" | "mpeg" | "mpga" => Ok("mp3"),
        "flac" => Ok("flac"),
        "ogg" => Ok("ogg"),
        "mp4" | "m4a" => Ok("m4a"),
        "webm" => Ok("webm"),
        _ => Err(OpenAiError::invalid(
            Some("file"),
            "Unsupported audio file type.",
        )),
    }
}

struct ApiTempDir(PathBuf);

impl ApiTempDir {
    fn create(root: &FsPath, task: &str) -> Result<Self, OpenAiError> {
        let path = root
            .join("runtime")
            .join("api-temp")
            .join(format!("{task}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path).map_err(|error| {
            OpenAiError::runtime(format!("Could not create temporary storage: {error}"))
        })?;
        Ok(Self(path))
    }

    fn path(&self) -> &FsPath {
        &self.0
    }
}

impl Drop for ApiTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug)]
pub struct OpenAiError {
    status: StatusCode,
    message: String,
    kind: &'static str,
    param: Option<String>,
    code: &'static str,
}

impl OpenAiError {
    fn invalid(param: Option<&str>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
            kind: "invalid_request_error",
            param: param.map(str::to_string),
            code: "invalid_request",
        }
    }

    fn not_found(param: &str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
            kind: "invalid_request_error",
            param: Some(param.to_string()),
            code: "model_not_found",
        }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
            kind: "server_error",
            param: None,
            code: "runtime_error",
        }
    }

    fn too_large() -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: "Audio upload exceeds the 25 MiB limit.".to_string(),
            kind: "invalid_request_error",
            param: Some("file".to_string()),
            code: "request_too_large",
        }
    }
}

impl From<TakokitError> for OpenAiError {
    fn from(error: TakokitError) -> Self {
        let message = sanitize_error(&error.to_string());
        match &error {
            TakokitError::Resolution { code, .. } if matches!(code, ErrorCode::ModelNotFound) => {
                Self::not_found("model", message)
            }
            TakokitError::Resolution { code, .. }
                if matches!(
                    code,
                    ErrorCode::ModelNotInstalled | ErrorCode::RuntimeNotReady
                ) =>
            {
                Self {
                    status: StatusCode::BAD_REQUEST,
                    message,
                    kind: "invalid_request_error",
                    param: Some("model".to_string()),
                    code: "model_not_installed",
                }
            }
            TakokitError::Resolution { code, .. }
                if matches!(code, ErrorCode::CapabilityUnsupported) =>
            {
                Self {
                    status: StatusCode::BAD_REQUEST,
                    message,
                    kind: "invalid_request_error",
                    param: Some("model".to_string()),
                    code: "incompatible_model",
                }
            }
            TakokitError::InvalidRequest(_) | TakokitError::Audio(_) => {
                Self::invalid(None, message)
            }
            _ => Self::runtime(message),
        }
    }
}

impl IntoResponse for OpenAiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": {
                    "message": self.message,
                    "type": self.kind,
                    "param": self.param,
                    "code": self.code
                }
            })),
        )
            .into_response()
    }
}

fn sanitize_error(message: &str) -> String {
    if message.contains(":\\") || message.contains(":/") {
        "Takokit could not complete the local audio request. Check server logs using `tako server logs`."
            .to_string()
    } else {
        message.to_string()
    }
}
