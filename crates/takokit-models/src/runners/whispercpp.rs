use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use takokit_core::{
    ErrorCode, TakokitError, TakokitResult, TranscriptionRequest, TranscriptionResponse,
};
use takokit_package::{ArtifactRole, ExecutionPlan, InstalledArtifactRecord};
use uuid::Uuid;

use super::TranscriptionRunner;

#[derive(Debug, Default, Clone)]
pub struct WhisperCppRunner;

#[async_trait]
impl TranscriptionRunner for WhisperCppRunner {
    async fn transcribe(
        &self,
        plan: &ExecutionPlan,
        request: TranscriptionRequest,
    ) -> TakokitResult<TranscriptionResponse> {
        transcribe_with_whispercpp(plan, request)
    }
}

pub fn transcribe_with_whispercpp(
    plan: &ExecutionPlan,
    request: TranscriptionRequest,
) -> TakokitResult<TranscriptionResponse> {
    if !request.file_path.is_file() {
        return Err(TakokitError::InvalidRequest(format!(
            "audio file does not exist: {}",
            request.file_path.display()
        )));
    }

    let model_path = resolve_whisper_model_path(plan)?;
    let takokit_root = infer_takokit_root_from_blob(&model_path).ok_or_else(|| {
        inference_missing(format!(
            "could not infer Takokit storage root from model artifact {}; pull the model again",
            model_path.display()
        ))
    })?;
    let binary = find_file_named(
        &takokit_root
            .join("runners")
            .join("whispercpp")
            .join("runtime"),
        executable_name("whisper-cli"),
    )
    .ok_or_else(|| {
        inference_missing(
            "whisper.cpp executable is missing; run `takokit runner install takokit-whispercpp`",
        )
    })?;

    let output_dir = takokit_root.join("outputs");
    std::fs::create_dir_all(&output_dir)
        .map_err(|error| TakokitError::Storage(error.to_string()))?;
    let id = Uuid::new_v4();
    let output_prefix = output_dir.join(format!("transcription-{id}"));
    let output = Command::new(&binary)
        .arg("-m")
        .arg(&model_path)
        .arg("-f")
        .arg(&request.file_path)
        .arg("-otxt")
        .arg("-of")
        .arg(&output_prefix)
        .arg("-nt")
        .output()
        .map_err(|error| TakokitError::Audio(format!("failed to start whisper.cpp: {error}")))?;

    if !output.status.success() {
        return Err(TakokitError::Audio(format!(
            "whisper.cpp failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let txt_path = output_prefix.with_extension("txt");
    let text = if txt_path.is_file() {
        std::fs::read_to_string(&txt_path)
            .map_err(|error| TakokitError::Storage(error.to_string()))?
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    Ok(TranscriptionResponse {
        id,
        model: plan.model.id.clone(),
        text: normalize_whisper_output(&text),
    })
}

fn resolve_whisper_model_path(plan: &ExecutionPlan) -> TakokitResult<PathBuf> {
    let record = plan.installed_model.as_ref().ok_or_else(|| {
        artifact_error(
            ErrorCode::ArtifactMissing,
            format!(
                "installed model record for {} is missing; pull the model before transcribing",
                plan.model.id
            ),
        )
    })?;
    let artifact = record
        .artifacts
        .iter()
        .find(|artifact| artifact.role == ArtifactRole::Model)
        .ok_or_else(|| {
            artifact_error(
                ErrorCode::ArtifactMissing,
                format!("{} has no installed Whisper model artifact", plan.model.id),
            )
        })?;

    resolve_downloaded_artifact(artifact)
}

fn resolve_downloaded_artifact(artifact: &InstalledArtifactRecord) -> TakokitResult<PathBuf> {
    if !artifact.downloaded {
        return Err(artifact_error(
            ErrorCode::ArtifactNotDownloaded,
            format!(
                "Whisper artifact {} is recorded but not downloaded",
                artifact.name
            ),
        ));
    }
    let path = artifact.local_path.clone().ok_or_else(|| {
        artifact_error(
            ErrorCode::ArtifactMissing,
            format!("Whisper artifact {} has no local path", artifact.name),
        )
    })?;
    if !path.is_file() {
        return Err(artifact_error(
            ErrorCode::ArtifactMissing,
            format!(
                "Whisper artifact {} is missing at {}",
                artifact.name,
                path.display()
            ),
        ));
    }
    Ok(path)
}

fn infer_takokit_root_from_blob(path: &Path) -> Option<PathBuf> {
    let sha_dir = path.parent()?;
    if sha_dir.file_name()?.to_str()? != "sha256" {
        return None;
    }
    let blobs_dir = sha_dir.parent()?;
    if blobs_dir.file_name()?.to_str()? != "blobs" {
        return None;
    }
    blobs_dir.parent().map(Path::to_path_buf)
}

fn normalize_whisper_output(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn find_file_named(root: &Path, name: String) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_named(&path, name.clone()) {
                return Some(found);
            }
        } else if path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case(&name))
            .unwrap_or(false)
        {
            return Some(path);
        }
    }
    None
}

fn inference_missing(message: impl Into<String>) -> TakokitError {
    TakokitError::Resolution {
        code: ErrorCode::InferenceNotImplemented,
        message: message.into(),
    }
}

fn artifact_error(code: ErrorCode, message: impl Into<String>) -> TakokitError {
    TakokitError::Resolution {
        code,
        message: message.into(),
    }
}
