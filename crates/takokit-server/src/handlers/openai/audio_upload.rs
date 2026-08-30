use super::OpenAiError;
use std::path::Path;

pub(super) fn validate_audio_upload(extension: &str, bytes: &[u8]) -> Result<(), OpenAiError> {
    let valid = match extension {
        "wav" => {
            bytes.len() >= 44 && bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE")
        }
        "mp3" => {
            bytes.len() >= 4
                && (bytes.starts_with(b"ID3") || (bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0))
        }
        "flac" => bytes.len() >= 4 && bytes.starts_with(b"fLaC"),
        "ogg" => bytes.len() >= 4 && bytes.starts_with(b"OggS"),
        "m4a" => bytes.len() >= 12 && bytes.get(4..8) == Some(b"ftyp"),
        "webm" => bytes.len() >= 4 && bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(OpenAiError::invalid(
            Some("file"),
            "Audio upload is malformed or does not match its declared file type.",
        ))
    }
}

pub(super) fn safe_audio_extension(
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
        .and_then(|name| Path::new(name).extension())
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
