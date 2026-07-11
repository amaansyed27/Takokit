//! Read-only access to bundled package and discovery manifests.

use crate::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PackageRegistry {
    root: PathBuf,
}

impl PackageRegistry {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn bundled() -> Self {
        Self::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../registry"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn model(&self, id: &str) -> PackageResult<ModelManifest> {
        self.read_model(id)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => PackageError::ModelNotFound(id.to_string()),
                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
    }

    pub fn runner(&self, id: &str) -> PackageResult<RunnerManifest> {
        self.read_runner(id)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => PackageError::RunnerNotFound(id.to_string()),
                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
    }

    pub fn models(&self) -> PackageResult<Vec<ModelManifest>> {
        read_manifest_dir(&self.root.join("models"))
    }

    pub fn runners(&self) -> PackageResult<Vec<RunnerManifest>> {
        read_manifest_dir(&self.root.join("runners"))
    }

    pub fn library_models(&self) -> PackageResult<Vec<LibraryModelManifest>> {
        read_manifest_dir(&self.root.join("library").join("models"))
    }

    pub fn library_runners(&self) -> PackageResult<Vec<LibraryRunnerManifest>> {
        read_manifest_dir(&self.root.join("library").join("runners"))
    }

    fn read_model(&self, id: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join("models").join(format!("{id}.toml")))
    }

    fn read_runner(&self, id: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join("runners").join(format!("{id}.toml")))
    }
}

pub(crate) fn read_manifest_dir<T>(dir: &Path) -> PackageResult<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let mut manifests = Vec::new();
    if !dir.exists() {
        return Ok(manifests);
    }

    let mut entries = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        if entry.path().extension().and_then(|value| value.to_str()) == Some("toml") {
            let source = std::fs::read_to_string(entry.path())?;
            manifests.push(toml::from_str(&source)?);
        }
    }

    Ok(manifests)
}
