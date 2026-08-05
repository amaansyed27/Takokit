use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use takokit_core::{TakokitError, TakokitResult};

const WORKSPACE_SELECTION_FILE: &str = "selected-workspace.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSurface {
    Cli,
    Tui,
    Gui,
    Api,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSource {
    Explicit,
    Persisted,
    CurrentDirectory,
    SafeDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWorkspace {
    pub root: PathBuf,
    pub source: WorkspaceSource,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PersistedWorkspace {
    path: PathBuf,
}

pub fn resolve_workspace(
    explicit: Option<PathBuf>,
    persisted: Option<PathBuf>,
    current_dir: Option<PathBuf>,
    surface: WorkspaceSurface,
) -> TakokitResult<ResolvedWorkspace> {
    let (path, source) = if let Some(path) = explicit {
        (absolute(path, current_dir.as_deref())?, WorkspaceSource::Explicit)
    } else if let Some(path) = persisted {
        (absolute(path, current_dir.as_deref())?, WorkspaceSource::Persisted)
    } else if matches!(surface, WorkspaceSurface::Cli | WorkspaceSurface::Tui) {
        let current = current_dir.ok_or_else(|| {
            TakokitError::Storage("cannot resolve the current workspace directory".to_string())
        })?;
        (absolute(current, None)?, WorkspaceSource::CurrentDirectory)
    } else {
        (safe_default_workspace()?, WorkspaceSource::SafeDefault)
    };

    validate_workspace_root(&path, source == WorkspaceSource::Explicit)?;
    Ok(ResolvedWorkspace { root: path, source })
}

pub fn safe_default_workspace() -> TakokitResult<PathBuf> {
    let base = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| TakokitError::Storage("cannot locate the user home directory".to_string()))?;
    Ok(base.join("Takokit"))
}

pub fn load_persisted_workspace(global_root: &Path) -> TakokitResult<Option<PathBuf>> {
    let path = global_root.join("runtime").join(WORKSPACE_SELECTION_FILE);
    if !path.is_file() {
        return Ok(None);
    }
    let source = fs::read_to_string(&path).map_err(storage_error)?;
    let record: PersistedWorkspace = serde_json::from_str(&source).map_err(storage_error)?;
    Ok(Some(record.path))
}

pub fn persist_workspace(global_root: &Path, workspace: &Path) -> TakokitResult<()> {
    validate_workspace_root(workspace, true)?;
    let directory = global_root.join("runtime");
    fs::create_dir_all(&directory).map_err(storage_error)?;
    let path = directory.join(WORKSPACE_SELECTION_FILE);
    let temporary = path.with_extension("tmp");
    let payload = serde_json::to_vec_pretty(&PersistedWorkspace {
        path: workspace.to_path_buf(),
    })
    .map_err(storage_error)?;
    fs::write(&temporary, payload).map_err(storage_error)?;
    fs::rename(&temporary, &path).map_err(storage_error)?;
    Ok(())
}

pub fn validate_workspace_root(path: &Path, explicitly_selected: bool) -> TakokitResult<()> {
    if !path.is_absolute() {
        return Err(workspace_error(path, "workspace path must be absolute"));
    }
    if path.as_os_str().is_empty() {
        return Err(workspace_error(path, "workspace path is empty"));
    }
    if path.is_file() {
        return Err(workspace_error(path, "workspace path points to a file"));
    }
    if is_filesystem_root(path) {
        return Err(workspace_error(
            path,
            "filesystem roots cannot be used as Takokit workspaces",
        ));
    }

    let normalized = normalize_for_compare(path);
    for protected in protected_roots() {
        let protected = normalize_for_compare(&protected);
        if normalized == protected || normalized.starts_with(&protected) {
            return Err(workspace_error(
                path,
                "workspace is inside a protected system or application directory",
            ));
        }
    }

    if !explicitly_selected {
        for unsafe_leaf in ["Desktop", "Downloads"] {
            if path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(unsafe_leaf))
            {
                return Err(workspace_error(
                    path,
                    "an inherited Desktop or Downloads directory is not a safe implicit workspace",
                ));
            }
        }
    }

    let existing = nearest_existing_ancestor(path).ok_or_else(|| {
        workspace_error(path, "no existing writable parent directory was found")
    })?;
    let metadata = fs::metadata(&existing).map_err(storage_error)?;
    if !metadata.is_dir() {
        return Err(workspace_error(
            path,
            "nearest existing workspace parent is not a directory",
        ));
    }
    if metadata.permissions().readonly() {
        return Err(workspace_error(
            path,
            "workspace directory is read-only; choose a writable user directory",
        ));
    }
    Ok(())
}

fn absolute(path: PathBuf, current_dir: Option<&Path>) -> TakokitResult<PathBuf> {
    if path.is_absolute() {
        return Ok(clean(path));
    }
    let current = match current_dir {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir().map_err(storage_error)?,
    };
    Ok(clean(current.join(path)))
}

fn clean(path: PathBuf) -> PathBuf {
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                output.pop();
            }
            other => output.push(other.as_os_str()),
        }
    }
    output
}

fn is_filesystem_root(path: &Path) -> bool {
    path.parent().is_none()
        || path
            .components()
            .filter(|component| !matches!(component, Component::Prefix(_) | Component::RootDir))
            .next()
            .is_none()
}

fn protected_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        for variable in ["WINDIR", "ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(value) = std::env::var_os(variable) {
                roots.push(PathBuf::from(value));
            }
        }
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            roots.push(parent.to_path_buf());
        }
    }
    roots
}

fn normalize_for_compare(path: &Path) -> PathBuf {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| clean(path.to_path_buf()));
    #[cfg(windows)]
    {
        return PathBuf::from(canonical.to_string_lossy().to_lowercase());
    }
    #[cfg(not(windows))]
    canonical
}

fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn workspace_error(path: &Path, message: &str) -> TakokitError {
    TakokitError::Storage(format!(
        "invalid workspace {}: {message}",
        path.display()
    ))
}

fn storage_error(error: impl std::fmt::Display) -> TakokitError {
    TakokitError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_workspace_wins_over_every_other_source() {
        let temporary = tempfile::tempdir().unwrap();
        let explicit = temporary.path().join("explicit");
        let persisted = temporary.path().join("persisted");
        let current = temporary.path().join("current");
        let resolved = resolve_workspace(
            Some(explicit.clone()),
            Some(persisted),
            Some(current),
            WorkspaceSurface::Cli,
        )
        .unwrap();
        assert_eq!(resolved.root, explicit);
        assert_eq!(resolved.source, WorkspaceSource::Explicit);
        assert!(!resolved.root.join(".tako").exists());
    }

    #[test]
    fn cli_uses_deliberate_current_directory_without_creating_tako() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("voice project");
        fs::create_dir_all(&workspace).unwrap();
        let resolved = resolve_workspace(
            None,
            None,
            Some(workspace.clone()),
            WorkspaceSurface::Cli,
        )
        .unwrap();
        assert_eq!(resolved.root, workspace);
        assert_eq!(resolved.source, WorkspaceSource::CurrentDirectory);
        assert!(!resolved.root.join(".tako").exists());
    }

    #[test]
    fn gui_uses_safe_default_instead_of_process_current_directory() {
        let resolved = resolve_workspace(
            None,
            None,
            Some(std::env::temp_dir().join("inherited")),
            WorkspaceSurface::Gui,
        )
        .unwrap();
        assert_eq!(resolved.source, WorkspaceSource::SafeDefault);
        assert!(resolved.root.ends_with("Takokit"));
    }

    #[test]
    fn spaces_and_unicode_paths_are_valid() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Voice Projects").join("आवाज़");
        let resolved = resolve_workspace(
            Some(workspace.clone()),
            None,
            Some(temporary.path().to_path_buf()),
            WorkspaceSurface::Cli,
        )
        .unwrap();
        assert_eq!(resolved.root, workspace);
    }

    #[test]
    fn file_and_root_paths_are_rejected() {
        let temporary = tempfile::tempdir().unwrap();
        let file = temporary.path().join("not-a-directory");
        fs::write(&file, b"fixture").unwrap();
        assert!(validate_workspace_root(&file, true).is_err());
        let root = Path::new(std::path::MAIN_SEPARATOR_STR);
        assert!(validate_workspace_root(root, true).is_err());
    }

    #[test]
    fn persisted_workspace_round_trips_without_tako_creation() {
        let temporary = tempfile::tempdir().unwrap();
        let global = temporary.path().join("global");
        let workspace = temporary.path().join("selected");
        persist_workspace(&global, &workspace).unwrap();
        assert_eq!(load_persisted_workspace(&global).unwrap(), Some(workspace.clone()));
        assert!(!workspace.join(".tako").exists());
    }
}
