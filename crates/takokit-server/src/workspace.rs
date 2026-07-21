use axum::http::HeaderMap;
use percent_encoding::percent_decode_str;
use std::path::PathBuf;
use takokit_core::{SessionRecord, TakokitError};
use takokit_store::WorkspaceStore;
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
        let session_id = session_id_from_headers(headers);
        match store.open_session(session_id, Some(title)) {
            Ok(session) => Ok(Self { store, session }),
            Err(error) => {
                #[cfg(test)]
                if headers.get(WORKSPACE_HEADER).is_none() {
                    return isolated_test_workspace(session_id, title);
                }
                Err(error)
            }
        }
    }

    pub fn session_id(&self) -> Uuid {
        self.session.summary.id
    }

    pub fn outputs_dir(&self) -> PathBuf {
        self.store.session_outputs_dir(self.session_id())
    }
}

pub fn store_from_headers(headers: &HeaderMap) -> Result<WorkspaceStore, TakokitError> {
    let store = WorkspaceStore::new(workspace_root(headers)?);
    store.ensure_layout()?;
    Ok(store)
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
        return absolute_workspace(PathBuf::from(decoded.as_ref()));
    }
    std::env::current_dir().map_err(|error| {
        TakokitError::Storage(format!("cannot resolve working directory: {error}"))
    })
}

fn absolute_workspace(path: PathBuf) -> Result<PathBuf, TakokitError> {
    if path.is_absolute() {
        return Ok(path);
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| TakokitError::Storage(format!("cannot resolve workspace path: {error}")))
}

#[cfg(test)]
fn isolated_test_workspace(
    session_id: Option<Uuid>,
    title: &str,
) -> Result<RequestWorkspace, TakokitError> {
    let root = std::env::temp_dir().join(format!(
        "takokit-server-request-workspace-{}",
        Uuid::new_v4()
    ));
    let store = WorkspaceStore::new(root);
    store.ensure_layout()?;
    let session = store.open_session(session_id, Some(title))?;
    Ok(RequestWorkspace { store, session })
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
    fn implicit_test_workspace_recovers_from_conflicting_local_state() {
        let mut headers = HeaderMap::new();
        headers.remove(WORKSPACE_HEADER);
        let context = RequestWorkspace::from_headers(&headers, "isolated test").unwrap();
        assert!(context.store.workspace_root().exists());
    }
}