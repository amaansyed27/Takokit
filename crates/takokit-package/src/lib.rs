use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use takokit_core::{
    CapabilityKind, ErrorCode, ModelCapability, ModelInfo, ModelRuntime, TakokitError,
};
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

    #[error("{model} does not support {capability_label}.")]
    CapabilityUnsupported {
        model: String,
        capability: CapabilityKind,
        capability_label: &'static str,
    },

    #[error("{model} supports {capability_label}, but runner {runner} is not installed or not implemented yet.")]
    RunnerNotInstalled {
        model: String,
        runner: String,
        capability: CapabilityKind,
        capability_label: &'static str,
    },

    #[error(
        "{model} supports {capability_label}, but runner {runner} is not supported on {platform}."
    )]
    RunnerUnsupportedOnPlatform {
        model: String,
        runner: String,
        capability: CapabilityKind,
        capability_label: &'static str,
        platform: String,
    },

    #[error("{model} supports {capability_label}, but inference is not implemented for runner {runner} yet.")]
    InferenceNotImplemented {
        model: String,
        runner: String,
        capability: CapabilityKind,
        capability_label: &'static str,
    },
}

impl From<PackageError> for TakokitError {
    fn from(value: PackageError) -> Self {
        match value {
            PackageError::ModelNotFound(id) => TakokitError::Resolution {
                code: ErrorCode::ModelNotFound,
                message: format!("model is not available in the local registry: {id}"),
            },
            PackageError::RunnerNotFound(id) => TakokitError::Resolution {
                code: ErrorCode::RunnerNotFound,
                message: format!("runner is not available in the local registry: {id}"),
            },
            PackageError::ModelNotInstalled(id) => TakokitError::Resolution {
                code: ErrorCode::ModelNotInstalled,
                message: format!("model is not installed: {id}"),
            },
            error @ PackageError::CapabilityUnsupported { .. } => TakokitError::Resolution {
                code: ErrorCode::CapabilityUnsupported,
                message: error.to_string(),
            },
            error @ PackageError::RunnerNotInstalled { .. } => TakokitError::Resolution {
                code: ErrorCode::RunnerNotInstalled,
                message: error.to_string(),
            },
            error @ PackageError::RunnerUnsupportedOnPlatform { .. } => TakokitError::Resolution {
                code: ErrorCode::RunnerUnsupportedOnPlatform,
                message: error.to_string(),
            },
            error @ PackageError::InferenceNotImplemented { .. } => TakokitError::Resolution {
                code: ErrorCode::InferenceNotImplemented,
                message: error.to_string(),
            },
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
    VoiceCloning,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityManifest {
    #[serde(default, alias = "speak")]
    pub tts: bool,
    #[serde(default, alias = "transcribe")]
    pub stt: bool,
    #[serde(default, alias = "clone")]
    pub voice_cloning: bool,
    #[serde(default)]
    pub live_transcription: bool,
    #[serde(default)]
    pub live_audio: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub model: ModelManifest,
    pub capability: CapabilityKind,
    pub runner: RunnerManifest,
    pub runner_installed: bool,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Ready,
    InferenceNotImplemented,
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

    pub fn is_runner_installed(&self, id: &str) -> bool {
        self.runner_manifest_path(id).is_file()
    }

    pub fn install_model(&self, manifest: &ModelManifest) -> PackageResult<PullReport> {
        std::fs::create_dir_all(self.root.join("models"))?;
        let path = self.model_manifest_path(&manifest.id);
        std::fs::write(&path, toml::to_string_pretty(manifest)?)?;

        Ok(PullReport {
            id: manifest.id.clone(),
            installed: true,
            manifest_path: path,
            note: "Installed from local mock registry. No model weights were downloaded."
                .to_string(),
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

    fn runner_manifest_path(&self, id: &str) -> PathBuf {
        self.root.join("runners").join(format!("{id}.toml"))
    }
}

impl ModelManifest {
    pub fn supports(&self, capability: CapabilityKind) -> bool {
        self.capabilities.supports(capability)
    }

    pub fn to_model_info(&self, installed: bool, runner_installed: bool) -> ModelInfo {
        ModelInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            summary: self.description.clone(),
            license: self.license.clone(),
            runtime: self.backend.to_model_runtime(),
            backend: self.backend.as_str().to_string(),
            runner: self.runner.clone(),
            hardware_notes: self.hardware.notes(),
            capabilities: self.capabilities.to_model_capabilities(),
            installed,
            runner_installed,
            execution_status: if self.id == "mock-tts" {
                "ready".to_string()
            } else if runner_installed {
                "runner installed; inference not implemented".to_string()
            } else {
                "runner not installed or not implemented".to_string()
            },
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
    pub fn supports(&self, capability: CapabilityKind) -> bool {
        match capability {
            CapabilityKind::TextToSpeech => self.tts,
            CapabilityKind::SpeechToText => self.stt,
            CapabilityKind::VoiceCloning => self.voice_cloning,
            CapabilityKind::LiveTranscription => self.live_transcription,
            CapabilityKind::LiveAudio => self.live_audio,
        }
    }

    pub fn to_model_capabilities(&self) -> Vec<ModelCapability> {
        let mut capabilities = Vec::new();
        if self.tts {
            capabilities.push(CapabilityKind::TextToSpeech);
        }
        if self.stt {
            capabilities.push(CapabilityKind::SpeechToText);
        }
        if self.voice_cloning {
            capabilities.push(CapabilityKind::VoiceCloning);
        }
        if self.live_transcription {
            capabilities.push(CapabilityKind::LiveTranscription);
        }
        if self.live_audio {
            capabilities.push(CapabilityKind::LiveAudio);
        }
        capabilities
    }
}

impl HardwareManifest {
    fn notes(&self) -> String {
        let acceleration = match (self.cpu, self.gpu) {
            (true, true) => "CPU or GPU",
            (true, false) => "CPU",
            (false, true) => "GPU",
            (false, false) => "unspecified hardware",
        };
        match self.min_ram.as_deref() {
            Some(min_ram) => format!("{acceleration}, minimum RAM {min_ram}"),
            None => acceleration.to_string(),
        }
    }
}

impl ModelBackend {
    fn as_str(&self) -> &'static str {
        match self {
            ModelBackend::Native => "native",
            ModelBackend::Onnx => "onnx",
            ModelBackend::Whispercpp => "whispercpp",
            ModelBackend::PythonManaged => "python-managed",
            ModelBackend::External => "external",
        }
    }

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

pub fn resolve_runner(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
    capability: CapabilityKind,
) -> PackageResult<ExecutionPlan> {
    let model = package_registry.model(model_id)?;
    if !model.supports(capability) {
        return Err(PackageError::CapabilityUnsupported {
            model: model.id,
            capability,
            capability_label: capability.label(),
        });
    }

    let runner = package_registry.runner(&model.runner)?;
    let platform = current_platform();
    if !runner
        .platforms
        .iter()
        .any(|item| item == &platform || item == "any")
    {
        return Err(PackageError::RunnerUnsupportedOnPlatform {
            model: model.id,
            runner: runner.id,
            capability,
            capability_label: capability.label(),
            platform,
        });
    }

    let runner_installed = installed_registry.is_runner_installed(&runner.id);
    if !runner_installed {
        return Err(PackageError::RunnerNotInstalled {
            model: model.id,
            runner: runner.id,
            capability,
            capability_label: capability.label(),
        });
    }

    Ok(ExecutionPlan {
        model,
        capability,
        runner,
        runner_installed,
        status: ExecutionStatus::InferenceNotImplemented,
    })
}

fn current_platform() -> String {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        std::env::consts::OS
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        std::env::consts::ARCH
    };

    format!("{os}-{arch}")
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
tts = true
stt = false
voice_cloning = false
live_transcription = false
live_audio = true

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
        assert!(manifest.capabilities.tts);
        assert!(!manifest.capabilities.stt);
        assert!(manifest.capabilities.live_audio);
        assert_eq!(manifest.hardware.min_ram.as_deref(), Some("4gb"));
    }

    #[test]
    fn parses_first_class_capabilities_from_manifest() {
        let manifest: ModelManifest = toml::from_str(MODEL_TOML).expect("model manifest");

        assert!(manifest.supports(CapabilityKind::TextToSpeech));
        assert!(manifest.supports(CapabilityKind::LiveAudio));
        assert!(!manifest.supports(CapabilityKind::SpeechToText));
        assert_eq!(
            manifest.capabilities.to_model_capabilities(),
            vec![CapabilityKind::TextToSpeech, CapabilityKind::LiveAudio]
        );
    }

    #[test]
    fn parses_runner_manifest() {
        let manifest: RunnerManifest = toml::from_str(RUNNER_TOML).expect("runner manifest");

        assert_eq!(manifest.id, "takokit-onnx");
        assert_eq!(manifest.kind, RunnerKind::Native);
        assert_eq!(
            manifest.platforms,
            vec!["windows-x64", "linux-x64", "macos-arm64"]
        );
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

        let error = registry
            .installed_model("kokoro")
            .expect_err("not installed");

        assert!(matches!(error, PackageError::ModelNotInstalled(id) if id == "kokoro"));
    }

    #[test]
    fn resolver_rejects_unsupported_capability() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));

        let error = resolve_runner(
            &registry,
            &installed,
            "kokoro",
            CapabilityKind::SpeechToText,
        )
        .expect_err("unsupported capability");

        assert!(matches!(
            error,
            PackageError::CapabilityUnsupported { model, capability, .. }
                if model == "kokoro" && capability == CapabilityKind::SpeechToText
        ));
    }

    #[test]
    fn resolver_reports_not_installed_runner() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));

        let error = resolve_runner(
            &registry,
            &installed,
            "kokoro",
            CapabilityKind::TextToSpeech,
        )
        .expect_err("missing runner");

        assert!(matches!(
            error,
            PackageError::RunnerNotInstalled { model, runner, capability, .. }
                if model == "kokoro" && runner == "takokit-onnx" && capability == CapabilityKind::TextToSpeech
        ));
    }

    #[test]
    fn resolver_returns_execution_plan_when_runner_is_installed() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let installed_root = temp.path().join("installed");
        let installed = InstalledRegistry::new(&installed_root);
        std::fs::create_dir_all(installed_root.join("runners")).expect("runners dir");
        std::fs::write(
            installed_root.join("runners").join("takokit-onnx.toml"),
            RUNNER_TOML,
        )
        .expect("installed runner");
        let registry = PackageRegistry::new(temp.path());

        let plan = resolve_runner(
            &registry,
            &installed,
            "kokoro",
            CapabilityKind::TextToSpeech,
        )
        .expect("execution plan");

        assert_eq!(plan.model.id, "kokoro");
        assert_eq!(plan.runner.id, "takokit-onnx");
        assert_eq!(plan.capability, CapabilityKind::TextToSpeech);
        assert!(plan.runner_installed);
        assert_eq!(plan.status, ExecutionStatus::InferenceNotImplemented);
    }

    fn write_test_registry(root: &Path) {
        let models = root.join("models");
        let runners = root.join("runners");
        std::fs::create_dir_all(&models).expect("models dir");
        std::fs::create_dir_all(&runners).expect("runners dir");
        std::fs::write(models.join("kokoro.toml"), MODEL_TOML).expect("model toml");
        std::fs::write(runners.join("takokit-onnx.toml"), RUNNER_TOML).expect("runner toml");
    }
}
