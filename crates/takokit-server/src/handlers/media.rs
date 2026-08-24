use super::*;
use axum::{
    body::Body,
    extract::Query,
    http::{header, HeaderMap, Response},
};
use serde::Deserialize;
use std::path::{Path as FsPath, PathBuf};

#[derive(Debug, Deserialize)]
pub struct LocalAudioQuery {
    pub path: String,
}

pub async fn local_audio(
    headers: HeaderMap,
    Query(query): Query<LocalAudioQuery>,
) -> Result<Response<Body>, ApiError> {
    // Require the same workspace context as the rest of the local GUI API. The
    // audio itself may live outside the workspace because users can explicitly
    // choose reference/source recordings from anywhere on their machine.
    let _workspace = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;

    let raw = query.path.trim();
    if raw.is_empty() {
        return Err(ApiError(TakokitError::InvalidRequest(
            "audio preview path is required".to_string(),
        )));
    }

    let path = PathBuf::from(raw);
    if !path.is_file() {
        return Err(ApiError(TakokitError::InvalidRequest(format!(
            "audio preview path is not a file: {}",
            path.display()
        ))));
    }

    let content_type = audio_content_type(&path).ok_or_else(|| {
        ApiError(TakokitError::InvalidRequest(
            "audio preview supports WAV, MP3, FLAC, OGG, M4A, AAC, and WMA files".to_string(),
        ))
    })?;

    let read_path = path.clone();
    let bytes = tokio::task::spawn_blocking(move || std::fs::read(&read_path))
        .await
        .map_err(|error| {
            ApiError(TakokitError::Execution(format!(
                "audio preview task failed: {error}"
            )))
        })?
        .map_err(|error| {
            ApiError(TakokitError::Storage(format!(
                "could not read audio preview {}: {error}",
                path.display()
            )))
        })?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_DISPOSITION, "inline")
        .body(Body::from(bytes))
        .map_err(|error| ApiError(TakokitError::Storage(error.to_string())))
}

fn audio_content_type(path: &FsPath) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav") => Some("audio/wav"),
        Some("mp3") => Some("audio/mpeg"),
        Some("flac") => Some("audio/flac"),
        Some("ogg") => Some("audio/ogg"),
        Some("m4a") => Some("audio/mp4"),
        Some("aac") => Some("audio/aac"),
        Some("wma") => Some("audio/x-ms-wma"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_preview_accepts_only_audio_extensions() {
        assert_eq!(audio_content_type(FsPath::new("voice.wav")), Some("audio/wav"));
        assert_eq!(audio_content_type(FsPath::new("voice.MP3")), Some("audio/mpeg"));
        assert_eq!(audio_content_type(FsPath::new("voice.flac")), Some("audio/flac"));
        assert_eq!(audio_content_type(FsPath::new("voice.txt")), None);
        assert_eq!(audio_content_type(FsPath::new("voice")), None);
    }
}
