use axum::http::HeaderMap;
use percent_encoding::percent_decode_str;
use std::path::PathBuf;
use takokit_core::{SessionRecord, TakokitError};
use takokit_store::{
    resolve_workspace, safe_default_workspace, validate_workspace_root, WorkspaceStore,
    WorkspaceSurface,
};
use uuid::Uuid;

pub const WORKSPACE_HEADER: &str = "x-takokit-workspace";
pub const SESSION_HEADER: &str = "x-takokit-session";

#[derive(Debug, Clone)]
pub struct RequestWorkspace {
    pub store: WorkspaceStore,
    pub session: SessionRecord,
}

impl RequestWorkspace {
    pub fn from_headers(headers: &HeaderMap, title: &str) -> Result<Self, TakokitError> {
        let store = store_from_headers(headers)?;
        let requested = session_id_from_headers(headers);
        let selected = match requested {
            Some(id) => Some(id),
            None => store
                .active_session()?
                .filter(|id| store.session_dir(*id).join("session.json").is_file()),
        };
        let session = store.open_session(selected, Some(title))?;
        Ok(Self { store, session })
    }

    pub fn session_id(&self) -> Uuid {
        self.session.summary.id
    }

    pub fn outputs_dir(&self) -> PathBuf {
        self.store.session_outputs_dir(self.session_id())
    }
}

pub fn store_from_headers(headers: &HeaderMap) -> Result<WorkspaceStore, TakokitError> {
    Ok(WorkspaceStore::new(workspace_root(headers)?))
}

pub fn session_id_from_headers(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get(SESSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}

pub fn encoded_workspace_header(path: &str) -> String {
    percent_encoding::utf8_percent_encode(path, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn workspace_root(headers: &HeaderMap) -> Result<PathBuf, TakokitError> {
    if let Some(value) = headers.get(WORKSPACE_HEADER) {
        let encoded = value.to_str().map_err(|error| {
            TakokitError::Storage(format!("invalid Takokit workspace header: {error}"))
        })?;
        let decoded = percent_decode_str(encoded)
            .decode_utf8()
            .map_err(|error| TakokitError::Storage(format!("invalid workspace path: {error}")))?;
        let resolved = resolve_workspace(
            Some(PathBuf::from(decoded.as_ref())),
            None,
            None,
            WorkspaceSurface::Api,
        )?;
        return Ok(resolved.root);
    }

    let default = safe_default_workspace()?;
    validate_workspace_root(&default, false)?;
    Ok(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn request_workspace_decodes_paths_and_resumes_sessions() {
        let root =
            std::env::temp_dir().join(format!("takokit-server-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let store = WorkspaceStore::new(&root);
        let session = store.create_session(Some("test")).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            WORKSPACE_HEADER,
            HeaderValue::from_str(&encoded_workspace_header(&root.to_string_lossy())).unwrap(),
        );
        headers.insert(
            SESSION_HEADER,
            HeaderValue::from_str(&session.summary.id.to_string()).unwrap(),
        );
        let context = RequestWorkspace::from_headers(&headers, "fallback").unwrap();
        assert_eq!(context.session_id(), session.summary.id);
        assert_eq!(context.store.workspace_root(), root.as_path());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_session_header_reuses_active_session() {
        let root = std::env::temp_dir().join(format!("takokit-server-active-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let store = WorkspaceStore::new(&root);
        let session = store.create_session(Some("active")).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            WORKSPACE_HEADER,
            HeaderValue::from_str(&encoded_workspace_header(&root.to_string_lossy())).unwrap(),
        );
        let context = RequestWorkspace::from_headers(&headers, "fallback").unwrap();
        assert_eq!(context.session_id(), session.summary.id);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_header_uses_safe_default_not_daemon_current_directory() {
        let headers = HeaderMap::new();
        let store = store_from_headers(&headers).unwrap();
        assert_eq!(store.workspace_root(), safe_default_workspace().unwrap());
        assert_ne!(store.workspace_root(), std::env::current_dir().unwrap());
    }

    #[test]
    fn reading_workspace_context_does_not_create_tako() {
        let root = std::env::temp_dir().join(format!("takokit-server-readonly-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            WORKSPACE_HEADER,
            HeaderValue::from_str(&encoded_workspace_header(&root.to_string_lossy())).unwrap(),
        );
        let store = store_from_headers(&headers).unwrap();
        assert!(store.list_sessions(None).unwrap().is_empty());
        assert!(!root.join(".tako").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
