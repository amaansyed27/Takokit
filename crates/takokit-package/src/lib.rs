use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use takokit_core::{ModelCapability, ModelInfo, ModelRuntime, TakokitError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("manifest IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("manifest parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("manifest encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("model is not available in the local registry: {0}")]
    ModelNotFound(String),

    #[error("runner is not available in the local registry: {0}")]
    RunnerNotFound(String),

    #[error("model is not installed: {0}")]
    ModelNotInstalled(String),
}

impl From<PackageError> for TakokitError {
    fn from(value: PackageError) -> Self {
        match value {
            PackageError::ModelNotFound(id) => {
                TakokitError::Model(format!("model is not available in the local registry: {id}"))
            }
            PackageError::RunnerNotFound(id) => {
                TakokitError::Model(format!("runner is not available in the local registry: {id}"))
            }
            PackageError::ModelNotInstalled(id) => {
                TakokitError::Model(format!("model is not installed: {id}"))
            }
            PackageError::Io(error) => TakokitError::Storage(error.to_string()),
            PackageError::Toml(error) => TakokitError::Model(error.to_string()),
            PackageError::TomlSer(error) => TakokitError::Model(error.to_string()),
        }
    }
}

pub type PackageResult<T> = Result<T, PackageError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: ModelKind,
    pub backend: ModelBackend,
    pub runner: String,
    pub license: String,
    pub description: String,
    pub capabilities: CapabilityManifest,
    pub hardware: HardwareManifest,
    pub artifacts: ArtifactManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelKind {
    Tts,
    Stt,
    VoiceClone,
    VoiceTrain,
    VoiceConvert,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelBackend {
    Native,
    Onnx,
    Whispercpp,
    PythonManaged,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityManifest {
    pub speak: bool,
    pub transcribe: bool,
    pub clone: bool,
    pub train: bool,
    pub convert: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardwareManifest {
    pub cpu: bool,
    pub gpu: bool,
    pub min_ram: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactManifest {
    pub weights: Vec<ArtifactEntry>,
    pub voices: Vec<ArtifactEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub name: String,
    pub sha256: String,
    pub bytes: Option<u64>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: RunnerKind,
    pub platforms: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RunnerKind {
    Native,
    Onnx,
    Whispercpp,
    PythonManaged,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: RunnerKind,
    pub platforms: Vec<String>,
    pub description: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullReport {
    pub id: String,
    pub installed: bool,
    pub manifest_path: PathBuf,
    pub note: String,
}

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

    fn read_model(&self, id: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join("models").join(format!("{id}.toml")))
    }

    fn read_runner(&self, id: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join("runners").join(format!("{id}.toml")))
    }
}

#[derive(Debug, Clone)]
pub struct InstalledRegistry {
    root: PathBuf,
}

impl InstalledRegistry {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn installed_model(&self, id: &str) -> PackageResult<ModelManifest> {
        std::fs::read_to_string(self.model_manifest_path(id))
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => PackageError::ModelNotInstalled(id.to_string()),
                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
    }

    pub fn is_model_installed(&self, id: &str) -> bool {
        self.model_manifest_path(id).is_file()
    }

    pub fn install_model(&self, manifest: &ModelManifest) -> PackageResult<PullReport> {
        std::fs::create_dir_all(self.root.join("models"))?;
        let path = self.model_manifest_path(&manifest.id);
        std::fs::write(&path, toml::to_string_pretty(manifest)?)?;

        Ok(PullReport {
            id: manifest.id.clone(),
            installed: true,
            manifest_path: path,
            note: "Installed from local mock registry. No model weights were downloaded.".to_string(),
        })
    }

    pub fn remove_model(&self, id: &str) -> PackageResult<bool> {
        let path = self.model_manifest_path(id);
        if !path.exists() {
            return Err(PackageError::ModelNotInstalled(id.to_string()));
        }

        std::fs::remove_file(path)?;
        Ok(true)
    }

    fn model_manifest_path(&self, id: &str) -> PathBuf {
        self.root.join("models").join(format!("{id}.toml"))
    }
}

impl ModelManifest {
    pub fn to_model_info(&self, installed: bool) -> ModelInfo {
        ModelInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            summary: self.description.clone(),
            license: self.license.clone(),
            runtime: self.backend.to_model_runtime(),
            capabilities: self.capabilities.to_model_capabilities(),
            installed,
        }
    }
}

impl RunnerManifest {
    pub fn to_runner_info(&self, installed: bool) -> RunnerInfo {
        RunnerInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            kind: self.kind.clone(),
            platforms: self.platforms.clone(),
            description: self.description.clone(),
            installed,
        }
    }
}

impl CapabilityManifest {
    fn to_model_capabilities(&self) -> Vec<ModelCapability> {
        let mut capabilities = Vec::new();
        if self.speak {
            capabilities.push(ModelCapability::TextToSpeech);
        }
        if self.transcribe {
            capabilities.push(ModelCapability::SpeechToText);
        }
        if self.clone {
            capabilities.push(ModelCapability::VoiceCloning);
        }
        if self.train {
            capabilities.push(ModelCapability::VoiceTraining);
        }
        if self.convert {
            capabilities.push(ModelCapability::VoiceConversion);
        }
        capabilities
    }
}

impl ModelBackend {
    fn to_model_runtime(&self) -> ModelRuntime {
        match self {
            ModelBackend::Native => ModelRuntime::NativeRust,
            ModelBackend::Onnx => ModelRuntime::Onnx,
            ModelBackend::Whispercpp => ModelRuntime::WhisperCpp,
            ModelBackend::PythonManaged => ModelRuntime::Python,
            ModelBackend::External => ModelRuntime::External,
        }
    }
}

fn read_manifest_dir<T>(dir: &Path) -> PackageResult<Vec<T>>
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
mod tests {
    use super::*;

    const MODEL_TOML: &str = r#"
id = "kokoro"
name = "Kokoro"
version = "0.1.0"
kind = "tts"
backend = "onnx"
runner = "takokit-onnx"
license = "apache-2.0"
description = "Fast local text-to-speech model."

[capabilities]
speak = true
transcribe = false
clone = false
train = false
convert = false

[hardware]
cpu = true
gpu = false
min_ram = "4gb"

[artifacts]
weights = []
voices = []
"#;

    const RUNNER_TOML: &str = r#"
id = "takokit-onnx"
name = "Takokit ONNX Runner"
version = "0.1.0"
kind = "native"
platforms = ["windows-x64", "linux-x64", "macos-arm64"]
description = "Native ONNX runner for CPU-friendly models."
"#;

    #[test]
    fn parses_model_manifest() {
        let manifest: ModelManifest = toml::from_str(MODEL_TOML).expect("model manifest");

        assert_eq!(manifest.id, "kokoro");
        assert_eq!(manifest.kind, ModelKind::Tts);
        assert_eq!(manifest.backend, ModelBackend::Onnx);
        assert_eq!(manifest.runner, "takokit-onnx");
        assert!(manifest.capabilities.speak);
        assert!(!manifest.capabilities.transcribe);
        assert_eq!(manifest.hardware.min_ram.as_deref(), Some("4gb"));
    }

    #[test]
    fn parses_runner_manifest() {
        let manifest: RunnerManifest = toml::from_str(RUNNER_TOML).expect("runner manifest");

        assert_eq!(manifest.id, "takokit-onnx");
        assert_eq!(manifest.kind, RunnerKind::Native);
        assert_eq!(manifest.platforms, vec!["windows-x64", "linux-x64", "macos-arm64"]);
    }

    #[test]
    fn registry_finds_model_manifest_by_id() {
        let temp = tempfile::tempdir().expect("tempdir");
        let models = temp.path().join("models");
        std::fs::create_dir_all(&models).expect("models dir");
        std::fs::write(models.join("kokoro.toml"), MODEL_TOML).expect("model toml");

        let registry = PackageRegistry::new(temp.path());
        let manifest = registry.model("kokoro").expect("model lookup");

        assert_eq!(manifest.name, "Kokoro");
    }

    #[test]
    fn installed_registry_reports_not_installed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = InstalledRegistry::new(temp.path());

        let error = registry.installed_model("kokoro").expect_err("not installed");

        assert!(matches!(error, PackageError::ModelNotInstalled(id) if id == "kokoro"));
    }
}
