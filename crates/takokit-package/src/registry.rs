//! Read-only access to bundled manifests plus the versioned Takokit registry index.

use crate::*;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

const DEFAULT_REGISTRY_URL: &str = "https://takokit-library.vercel.app/v1/registry.json";
const MAX_REGISTRY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct PackageRegistry {
    root: PathBuf,
    cache_path: Option<PathBuf>,
    remote_url: Option<String>,
    custom_models_dir: Option<PathBuf>,
}

impl PackageRegistry {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            cache_path: None,
            remote_url: None,
            custom_models_dir: None,
        }
    }

    pub fn bundled() -> Self {
        let root = bundled_registry_root();
        let home = std::env::var("TAKOKIT_HOME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".takokit")));
        let remote_url = std::env::var("TAKOKIT_REGISTRY_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_REGISTRY_URL.to_string());
        let cache_path = home
            .as_ref()
            .map(|home| home.join("manifests").join("registry").join("index.json"));
        let custom_models_dir = home
            .as_ref()
            .map(|home| home.join("manifests").join("custom").join("models"));
        Self {
            root,
            cache_path,
            remote_url: Some(remote_url),
            custom_models_dir,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn with_custom_models_dir(mut self, directory: impl AsRef<Path>) -> Self {
        self.custom_models_dir = Some(directory.as_ref().to_path_buf());
        self
    }

    pub fn custom_models_dir(&self) -> Option<&Path> {
        self.custom_models_dir.as_deref()
    }

    pub fn model(&self, reference: &str) -> PackageResult<ModelManifest> {
        if let Some(manifest) = self.custom_model(reference)? {
            return Ok(manifest);
        }
        if let Ok(index) = self.registry_index() {
            if let Ok(resolved) = index.resolve(reference) {
                let release = index
                    .release(&resolved)
                    .ok_or_else(|| PackageError::ModelNotFound(reference.to_string()))?;
                if let Some(source) = release.manifest_toml.as_deref() {
                    let mut manifest: ModelManifest = toml::from_str(source)?;
                    manifest.license = release.license.clone();
                    normalize_model_license(&mut manifest);
                    return Ok(manifest);
                }
                return self.read_model_manifest(&resolved.target);
            }
        }
        self.read_model_manifest(reference)
    }

    pub fn model_for_pull(&self, reference: &str) -> PackageResult<ModelManifest> {
        if let Some(manifest) = self.custom_model(reference)? {
            return Ok(manifest);
        }
        if !registry_offline() {
            let _ = self.sync_remote();
        }
        self.model(reference)
    }

    pub fn resolve_model_reference(
        &self,
        reference: &str,
    ) -> PackageResult<ResolvedModelReference> {
        if let Some(id) = custom_reference_id(reference)? {
            if let Some(directory) = self.custom_models_dir.as_deref() {
                let path = directory.join(format!("{id}.toml"));
                if path.is_file() {
                    let spec = read_custom_model_spec(&path)?;
                    let manifest_source = toml::to_string(&spec)?;
                    return Ok(ResolvedModelReference {
                        requested: reference.to_string(),
                        canonical: format!("local/{id}:latest"),
                        namespace: "local".to_string(),
                        name: id.clone(),
                        tag: "latest".to_string(),
                        digest: manifest_digest(&manifest_source),
                        target: id,
                    });
                }
            }
        }
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
        if self
            .custom_models_dir
            .as_deref()
            .is_some_and(|directory| directory.join(format!("{id}.toml")).is_file())
        {
            return format!("local/{id}:latest");
        }
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
        let mut models: Vec<ModelManifest> = read_manifest_dir(&self.root.join("models"))?;
        for model in &mut models {
            normalize_model_license(model);
        }
        models.extend(self.custom_models()?);
        models.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(models)
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

    fn custom_model(&self, reference: &str) -> PackageResult<Option<ModelManifest>> {
        let Some(id) = custom_reference_id(reference)? else {
            return Ok(None);
        };
        let Some(directory) = self.custom_models_dir.as_deref() else {
            return Ok(None);
        };
        let path = directory.join(format!("{id}.toml"));
        if !path.is_file() {
            return Ok(None);
        }
        let spec = read_custom_model_spec(&path)?;
        Ok(Some(materialize_custom_model(self, &spec)?))
    }

    fn custom_models(&self) -> PackageResult<Vec<ModelManifest>> {
        let Some(directory) = self.custom_models_dir.as_deref() else {
            return Ok(Vec::new());
        };
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut entries = std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        let mut models = Vec::new();
        for entry in entries {
            if entry.path().extension().and_then(|value| value.to_str()) != Some("toml") {
                continue;
            }
            let spec = read_custom_model_spec(&entry.path())?;
            let mut model = materialize_custom_model(self, &spec)?;
            normalize_model_license(&mut model);
            models.push(model);
        }
        Ok(models)
    }

    pub(crate) fn read_bundled_model(&self, id: &str) -> PackageResult<ModelManifest> {
        self.read_model_manifest(id)
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
            .and_then(|source| {
                let mut manifest: ModelManifest = toml::from_str(&source)?;
                normalize_model_license(&mut manifest);
                Ok(manifest)
            })
    }

    fn read_model(&self, id: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join("models").join(format!("{id}.toml")))
    }
    fn read_runner(&self, id: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join("runners").join(format!("{id}.toml")))
    }
}

fn bundled_registry_root() -> PathBuf {
    if let Some(root) = std::env::var_os("TAKOKIT_REGISTRY_DIR")
        .map(PathBuf::from)
        .filter(|path| path.join("index.json").is_file())
    {
        return root;
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(bin) = executable.parent() {
            let candidates = [
                bin.join("resources").join("registry"),
                bin.parent()
                    .unwrap_or(bin)
                    .join("resources")
                    .join("registry"),
            ];
            for candidate in candidates {
                if candidate.join("index.json").is_file() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../registry")
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

#[cfg(test)]
mod installed_resource_tests {
    use super::*;
    #[test]
    fn bundled_registry_has_index() {
        assert!(PackageRegistry::bundled()
            .root()
            .join("index.json")
            .is_file());
    }
}
