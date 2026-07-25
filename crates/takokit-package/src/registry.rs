//! Read-only access to bundled manifests plus the versioned Takokit registry index.

use crate::*;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

const DEFAULT_REGISTRY_URL: &str =
    "https://takokit-library.vercel.app/v1/registry.json";
const MAX_REGISTRY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct PackageRegistry {
    root: PathBuf,
    cache_path: Option<PathBuf>,
    remote_url: Option<String>,
}

impl PackageRegistry {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            cache_path: None,
            remote_url: None,
        }
    }

    pub fn bundled() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../registry");
        let home = std::env::var("TAKOKIT_HOME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".takokit")));
        let remote_url = std::env::var("TAKOKIT_REGISTRY_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_REGISTRY_URL.to_string());
        Self {
            root,
            cache_path: home.map(|home| home.join("manifests").join("registry").join("index.json")),
            remote_url: Some(remote_url),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn model(&self, reference: &str) -> PackageResult<ModelManifest> {
        if let Ok(index) = self.registry_index() {
            if let Ok(resolved) = index.resolve(reference) {
                let release = index
                    .release(&resolved)
                    .ok_or_else(|| PackageError::ModelNotFound(reference.to_string()))?;
                if let Some(source) = release.manifest_toml.as_deref() {
                    let manifest: ModelManifest = toml::from_str(source)?;
                    return Ok(manifest);
                }
                return self.read_model_manifest(&resolved.target);
            }
        }
        self.read_model_manifest(reference)
    }

    /// Pulls refresh the small control-plane index first. Network failure is
    /// deliberately non-fatal: the verified bundled or cached index remains usable.
    pub fn model_for_pull(&self, reference: &str) -> PackageResult<ModelManifest> {
        if !registry_offline() {
            let _ = self.sync_remote();
        }
        self.model(reference)
    }

    pub fn resolve_model_reference(
        &self,
        reference: &str,
    ) -> PackageResult<ResolvedModelReference> {
        if let Ok(index) = self.registry_index() {
            return index.resolve(reference);
        }
        let manifest = self.read_model_manifest(reference)?;
        let source = toml::to_string(&manifest)?;
        Ok(ResolvedModelReference {
            requested: reference.to_string(),
            canonical: format!("{}:latest", manifest.id),
            namespace: "library".to_string(),
            name: manifest.id.clone(),
            tag: "latest".to_string(),
            digest: manifest_digest(&source),
            target: manifest.id,
        })
    }

    pub fn canonical_reference_for_id(&self, id: &str) -> String {
        self.registry_index()
            .ok()
            .and_then(|index| index.canonical_for_target(id))
            .unwrap_or_else(|| id.to_string())
    }

    pub fn registry_models(&self) -> PackageResult<Vec<RegistryModel>> {
        Ok(self.registry_index()?.models)
    }

    pub fn sync_remote(&self) -> PackageResult<bool> {
        if registry_offline() {
            return Ok(false);
        }
        let Some(url) = self.remote_url.as_deref() else {
            return Ok(false);
        };
        let Some(cache_path) = self.cache_path.as_deref() else {
            return Ok(false);
        };
        let response = ureq::get(url)
            .timeout(Duration::from_secs(8))
            .call()
            .map_err(|error| PackageError::ArtifactDownloadFailed {
                artifact: "takokit-registry".to_string(),
                reason: error.to_string(),
            })?;
        let source =
            response
                .into_string()
                .map_err(|error| PackageError::ArtifactDownloadFailed {
                    artifact: "takokit-registry".to_string(),
                    reason: error.to_string(),
                })?;
        if source.len() > MAX_REGISTRY_BYTES {
            return Err(PackageError::ArtifactDownloadFailed {
                artifact: "takokit-registry".to_string(),
                reason: format!(
                    "registry response exceeded the {} byte safety limit",
                    MAX_REGISTRY_BYTES
                ),
            });
        }
        let index: RegistryIndex = serde_json::from_str(&source)?;
        index.validate()?;
        let parent = cache_path
            .parent()
            .ok_or_else(|| PackageError::ArtifactInstallFailed {
                artifact: "takokit-registry".to_string(),
                reason: "registry cache path has no parent".to_string(),
            })?;
        std::fs::create_dir_all(parent)?;
        let temporary = cache_path.with_extension("json.tmp");
        std::fs::write(&temporary, source)?;
        if cache_path.exists() {
            std::fs::remove_file(cache_path)?;
        }
        std::fs::rename(temporary, cache_path)?;
        Ok(true)
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

    fn registry_index(&self) -> PackageResult<RegistryIndex> {
        if let Some(cache_path) = self.cache_path.as_deref() {
            if cache_path.is_file() {
                if let Ok(index) = RegistryIndex::read(cache_path) {
                    return Ok(index);
                }
            }
        }
        RegistryIndex::read(&self.root.join("index.json"))
    }

    fn read_model_manifest(&self, id: &str) -> PackageResult<ModelManifest> {
        self.read_model(id)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => PackageError::ModelNotFound(id.to_string()),
                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
    }

    fn read_model(&self, id: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join("models").join(format!("{id}.toml")))
    }

    fn read_runner(&self, id: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join("runners").join(format!("{id}.toml")))
    }
}

fn registry_offline() -> bool {
    std::env::var("TAKOKIT_REGISTRY_OFFLINE")
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
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
