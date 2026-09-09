use super::*;
use axum::{
    body::{Body, Bytes},
    extract::{Path, Query},
    http::{header, HeaderMap, Response},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path as FsPath, PathBuf},
    time::UNIX_EPOCH,
};

const MAX_WORKSPACE_FILE_BYTES: usize = 100 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct UploadWorkspaceFileQuery {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceFileSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub kind: &'static str,
    pub content_type: &'static str,
    pub bytes: u64,
    pub modified_at: u64,
}

pub async fn workspace_files(headers: HeaderMap) -> Result<Json<serde_json::Value>, ApiError> {
    let store = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    let workspace_root = store.workspace_root().to_path_buf();
    let files = tokio::task::spawn_blocking(move || list_workspace_files(&workspace_root))
        .await
        .map_err(|error| {
            ApiError(TakokitError::Execution(format!(
                "workspace files task failed: {error}"
            )))
        })?
        .map_err(ApiError)?;

    Ok(Json(serde_json::json!({ "files": files })))
}

pub async fn upload_workspace_file(
    headers: HeaderMap,
    Query(query): Query<UploadWorkspaceFileQuery>,
    body: Bytes,
) -> Result<Json<WorkspaceFileSummary>, ApiError> {
    let store = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    if body.is_empty() {
        return Err(ApiError(TakokitError::InvalidRequest(
            "workspace file cannot be empty".to_string(),
        )));
    }
    if body.len() > MAX_WORKSPACE_FILE_BYTES {
        return Err(ApiError(TakokitError::InvalidRequest(
            "workspace files are limited to 100 MB each".to_string(),
        )));
    }

    let name = sanitize_file_name(&query.name)?;
    if supported_file(FsPath::new(&name)).is_none() {
        return Err(ApiError(TakokitError::InvalidRequest(
            "workspace files support WAV, MP3, FLAC, OGG, M4A, AAC, WMA, TXT, MD, JSON, and CSV"
                .to_string(),
        )));
    }
    let workspace_root = store.workspace_root().to_path_buf();
    let bytes = body.to_vec();
    let summary =
        tokio::task::spawn_blocking(move || -> Result<WorkspaceFileSummary, TakokitError> {
            let root = canonical_workspace_files_root(&workspace_root, true)?.ok_or_else(|| {
                TakokitError::Storage("workspace files directory was not created".to_string())
            })?;
            let destination = create_unique_workspace_file(&root, &name, &bytes)?;
            workspace_file_summary(&destination)
        })
        .await
        .map_err(|error| {
            ApiError(TakokitError::Execution(format!(
                "workspace upload task failed: {error}"
            )))
        })?
        .map_err(ApiError)?;

    Ok(Json(summary))
}

pub async fn workspace_file_content(
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response<Body>, ApiError> {
    let store = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    let path = resolve_workspace_file(store.workspace_root(), &id)?;
    let (_, content_type) = supported_file(&path).ok_or_else(|| {
        ApiError(TakokitError::InvalidRequest(
            "unsupported workspace file type".to_string(),
        ))
    })?;
    let read_path = path.clone();
    let bytes = tokio::task::spawn_blocking(move || std::fs::read(&read_path))
        .await
        .map_err(|error| {
            ApiError(TakokitError::Execution(format!(
                "workspace file read task failed: {error}"
            )))
        })?
        .map_err(|error| {
            ApiError(TakokitError::Storage(format!(
                "could not read {}: {error}",
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

pub async fn delete_workspace_file(
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let store = crate::workspace::store_from_headers(&headers).map_err(ApiError)?;
    let path = resolve_workspace_file(store.workspace_root(), &id)?;
    tokio::task::spawn_blocking(move || std::fs::remove_file(&path))
        .await
        .map_err(|error| {
            ApiError(TakokitError::Execution(format!(
                "workspace file removal task failed: {error}"
            )))
        })?
        .map_err(|error| {
            ApiError(TakokitError::Storage(format!(
                "could not remove workspace file: {error}"
            )))
        })?;
    Ok(StatusCode::NO_CONTENT)
}

fn workspace_files_root(workspace_root: &FsPath) -> PathBuf {
    workspace_root.join(".tako").join("files")
}

fn canonical_workspace_files_root(
    workspace_root: &FsPath,
    create: bool,
) -> Result<Option<PathBuf>, TakokitError> {
    let canonical_workspace = std::fs::canonicalize(workspace_root).map_err(|error| {
        TakokitError::Storage(format!(
            "could not resolve workspace root {}: {error}",
            workspace_root.display()
        ))
    })?;
    let root = workspace_files_root(workspace_root);
    if create {
        std::fs::create_dir_all(&root).map_err(|error| {
            TakokitError::Storage(format!(
                "could not create workspace files directory {}: {error}",
                root.display()
            ))
        })?;
    } else if !root.exists() {
        return Ok(None);
    }

    let metadata = std::fs::symlink_metadata(&root).map_err(|error| {
        TakokitError::Storage(format!(
            "could not inspect workspace files directory {}: {error}",
            root.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(TakokitError::Storage(
            "workspace files directory must be a real directory inside the workspace".to_string(),
        ));
    }
    let canonical_root = std::fs::canonicalize(&root).map_err(|error| {
        TakokitError::Storage(format!(
            "could not resolve workspace files directory {}: {error}",
            root.display()
        ))
    })?;
    if !canonical_root.starts_with(&canonical_workspace) {
        return Err(TakokitError::Storage(
            "workspace files directory escaped the selected workspace".to_string(),
        ));
    }
    Ok(Some(canonical_root))
}

fn list_workspace_files(workspace_root: &FsPath) -> Result<Vec<WorkspaceFileSummary>, TakokitError> {
    let Some(root) = canonical_workspace_files_root(workspace_root, false)? else {
        return Ok(Vec::new());
    };
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|error| {
        TakokitError::Storage(format!(
            "could not read workspace files at {}: {error}",
            root.display()
        ))
    })? {
        let entry = entry.map_err(|error| TakokitError::Storage(error.to_string()))?;
        let path = entry.path();
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        if supported_file(&path).is_some() {
            files.push(workspace_file_summary(&path)?);
        }
    }
    files.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(files)
}

fn workspace_file_summary(path: &FsPath) -> Result<WorkspaceFileSummary, TakokitError> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            TakokitError::Storage(format!(
                "workspace file has an invalid name: {}",
                path.display()
            ))
        })?;
    let (kind, content_type) = supported_file(path).ok_or_else(|| {
        TakokitError::InvalidRequest(format!("unsupported workspace file: {}", path.display()))
    })?;
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        TakokitError::Storage(format!(
            "could not inspect workspace file {}: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(TakokitError::Storage(
            "workspace file must be a regular file".to_string(),
        ));
    }
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs())
        .unwrap_or(0);
    Ok(WorkspaceFileSummary {
        id: file_name.to_string(),
        name: file_name.to_string(),
        path: path.display().to_string(),
        kind,
        content_type,
        bytes: metadata.len(),
        modified_at,
    })
}

fn resolve_workspace_file(workspace_root: &FsPath, id: &str) -> Result<PathBuf, ApiError> {
    let name = sanitize_file_name(id)?;
    if name != id {
        return Err(ApiError(TakokitError::InvalidRequest(
            "invalid workspace file id".to_string(),
        )));
    }
    let root = canonical_workspace_files_root(workspace_root, false)
        .map_err(ApiError)?
        .ok_or_else(|| {
            ApiError(TakokitError::InvalidRequest(
                "workspace file does not exist".to_string(),
            ))
        })?;
    let path = root.join(name);
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| {
        ApiError(TakokitError::InvalidRequest(format!(
            "workspace file does not exist: {}",
            path.display()
        )))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ApiError(TakokitError::InvalidRequest(
            "workspace file must be a regular file inside the workspace".to_string(),
        )));
    }
    let canonical = std::fs::canonicalize(&path).map_err(|error| {
        ApiError(TakokitError::Storage(format!(
            "could not resolve workspace file {}: {error}",
            path.display()
        )))
    })?;
    if !canonical.starts_with(&root) {
        return Err(ApiError(TakokitError::InvalidRequest(
            "workspace file escaped the workspace files directory".to_string(),
        )));
    }
    Ok(canonical)
}

fn create_unique_workspace_file(
    root: &FsPath,
    name: &str,
    bytes: &[u8],
) -> Result<PathBuf, TakokitError> {
    for index in 1..10_000 {
        let candidate_name = if index == 1 {
            name.to_string()
        } else {
            suffixed_file_name(name, index)
        };
        let destination = root.join(candidate_name);
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
                    let _ = std::fs::remove_file(&destination);
                    return Err(TakokitError::Storage(format!(
                        "could not save workspace file {}: {error}",
                        destination.display()
                    )));
                }
                return Ok(destination);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(TakokitError::Storage(format!(
                    "could not create workspace file {}: {error}",
                    destination.display()
                )))
            }
        }
    }
    Err(TakokitError::Storage(
        "could not allocate a unique workspace file name".to_string(),
    ))
}

fn suffixed_file_name(name: &str, index: usize) -> String {
    let path = FsPath::new(name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    match path.extension().and_then(|value| value.to_str()) {
        Some(extension) => format!("{stem}-{index}.{extension}"),
        None => format!("{stem}-{index}"),
    }
}

fn sanitize_file_name(raw: &str) -> Result<String, ApiError> {
    let leaf = raw.trim().rsplit(['/', '\\']).next().unwrap_or("").trim();
    if leaf.is_empty() || leaf == "." || leaf == ".." {
        return Err(ApiError(TakokitError::InvalidRequest(
            "workspace file name is required".to_string(),
        )));
    }
    let cleaned: String = leaf
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    if cleaned.trim_matches(['.', ' ']).is_empty() {
        return Err(ApiError(TakokitError::InvalidRequest(
            "workspace file name is invalid".to_string(),
        )));
    }
    Ok(cleaned.trim().to_string())
}

fn supported_file(path: &FsPath) -> Option<(&'static str, &'static str)> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav") => Some(("audio", "audio/wav")),
        Some("mp3") => Some(("audio", "audio/mpeg")),
        Some("flac") => Some(("audio", "audio/flac")),
        Some("ogg") => Some(("audio", "audio/ogg")),
        Some("m4a") => Some(("audio", "audio/mp4")),
        Some("aac") => Some(("audio", "audio/aac")),
        Some("wma") => Some(("audio", "audio/x-ms-wma")),
        Some("txt") => Some(("text", "text/plain; charset=utf-8")),
        Some("md") => Some(("text", "text/markdown; charset=utf-8")),
        Some("json") => Some(("text", "application/json; charset=utf-8")),
        Some("csv") => Some(("text", "text/csv; charset=utf-8")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_sanitization_drops_paths_and_windows_reserved_characters() {
        assert_eq!(
            sanitize_file_name(r#"C:\\temp\\voice?.wav"#).unwrap(),
            "voice_.wav"
        );
        assert_eq!(sanitize_file_name("../notes.txt").unwrap(), "notes.txt");
    }

    #[test]
    fn supported_workspace_files_are_audio_or_text() {
        assert_eq!(supported_file(FsPath::new("voice.wav")).unwrap().0, "audio");
        assert_eq!(supported_file(FsPath::new("notes.md")).unwrap().0, "text");
        assert!(supported_file(FsPath::new("archive.zip")).is_none());
    }

    #[test]
    fn listing_missing_library_does_not_create_workspace_state() {
        let root = std::env::temp_dir().join(format!("takokit-files-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        assert!(list_workspace_files(&root).unwrap().is_empty());
        assert!(!root.join(".tako").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn create_new_upload_allocation_never_overwrites_existing_file() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path();
        let files = canonical_workspace_files_root(workspace, true)
            .unwrap()
            .unwrap();
        std::fs::write(files.join("notes.txt"), b"original").unwrap();
        let created = create_unique_workspace_file(&files, "notes.txt", b"second").unwrap();
        assert_eq!(created.file_name().unwrap(), "notes-2.txt");
        assert_eq!(std::fs::read(files.join("notes.txt")).unwrap(), b"original");
        assert_eq!(std::fs::read(created).unwrap(), b"second");
    }

    #[cfg(unix)]
    #[test]
    fn workspace_content_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let files = canonical_workspace_files_root(root.path(), true)
            .unwrap()
            .unwrap();
        let external = root.path().join("secret.txt");
        std::fs::write(&external, b"foreign").unwrap();
        symlink(&external, files.join("linked.txt")).unwrap();
        assert!(resolve_workspace_file(root.path(), "linked.txt").is_err());
        assert!(list_workspace_files(root.path()).unwrap().is_empty());
    }
}
