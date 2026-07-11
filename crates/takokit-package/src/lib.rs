mod artifact_io;
mod artifact_reuse;
mod orchestrator;
mod planning;

use artifact_io::{
    download_to_temp, executable_name, extract_zip_safely, find_file_named, install_artifact,
    sha256_file,
};
pub use orchestrator::install_model_complete;
use planning::{
    license_warning, model_artifact_state, model_execution_status, model_lifecycle_state,
    model_task_label, next_plan_command, runner_missing_component,
};

use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::{
    CapabilityKind, ErrorCode, ModelCapability, ModelInfo, ModelRuntime, TakokitError,
};
use thiserror::Error;

const WHISPERCPP_WIN_X64_URL: &str =
    "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.1/whisper-bin-x64.zip";
const WHISPERCPP_WIN_X64_SHA256: &str =
    "7d8be46ecd31828e1eb7a2ecdd0d6b314feafd82163038ab6092594b0a063539";
const KOKORO_ONNX_PACKAGE: &str = "kokoro-onnx==0.5.0";
const KOKORO_ONNX_ADAPTER: &str = include_str!("../../../runners/onnx/kokoro_adapter.py");
const QWEN3_TTS_PACKAGE: &str = "qwen-tts==0.1.1";
const QWEN3_TTS_ADAPTER: &str = include_str!("../../../runners/python/qwen3_tts_adapter.py");
/// This is deliberately kept in source control.  Bootstrap copies a verified
/// development override/PATH binary into Takokit's private tools directory and
/// refuses to use it when it does not report this version.
const TAKOKIT_UV_VERSION: &str = "0.11.24";
const PYTHON_MANAGED_ADAPTERS: &[(&str, &str)] = &[
    ("qwen3_tts", "qwen3-tts"),
    ("chatterbox", "chatterbox"),
    ("f5_tts", "f5-tts"),
    ("cosyvoice2", "cosyvoice2"),
    ("dia", "dia"),
    ("fish_speech", "fish-speech"),
    ("openvoice", "openvoice"),
    ("gpt_sovits", "gpt-sovits"),
    ("rvc", "rvc"),
];

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("model installation failed during {stage:?}: {source}")]
    InstallStage {
        stage: InstallFailureStage,
        #[source]
        source: Box<PackageError>,
    },
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

    #[error("runner is not installed: {0}")]
    RunnerPackageNotInstalled(String),

    #[error("artifact URL missing for {model}: {artifact}")]
    ArtifactUrlMissing { model: String, artifact: String },

    #[error("artifact checksum missing for {model}: {artifact}")]
    ArtifactChecksumMissing { model: String, artifact: String },

    #[error("artifact download failed for {artifact}: {reason}")]
    ArtifactDownloadFailed { artifact: String, reason: String },

    #[error("artifact checksum mismatch for {artifact}: expected {expected}, got {actual}")]
    ArtifactChecksumMismatch {
        artifact: String,
        expected: String,
        actual: String,
    },

    #[error("artifact install failed for {artifact}: {reason}")]
    ArtifactInstallFailed { artifact: String, reason: String },

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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallFailureStage {
    RunnerContract,
    RunnerRuntime,
    Adapter,
    Artifacts,
    Materialization,
    FinalVerification,
}

impl PackageError {
    pub(crate) fn at_stage(stage: InstallFailureStage, source: PackageError) -> Self {
        Self::InstallStage {
            stage,
            source: Box::new(source),
        }
    }
}

impl From<PackageError> for TakokitError {
    fn from(value: PackageError) -> Self {
        match value {
            PackageError::InstallStage { stage, source } => TakokitError::Resolution {
                code: ErrorCode::ArtifactInstallFailed,
                message: format!("model installation failed during {stage:?}: {source}"),
            },
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
            PackageError::RunnerPackageNotInstalled(id) => TakokitError::Resolution {
                code: ErrorCode::RunnerNotInstalled,
                message: format!("runner is not installed: {id}"),
            },
            error @ PackageError::ArtifactUrlMissing { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactUrlMissing,
                message: error.to_string(),
            },
            error @ PackageError::ArtifactChecksumMissing { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactChecksumMissing,
                message: error.to_string(),
            },
            error @ PackageError::ArtifactDownloadFailed { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactDownloadFailed,
                message: error.to_string(),
            },
            error @ PackageError::ArtifactChecksumMismatch { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactChecksumMismatch,
                message: error.to_string(),
            },
            error @ PackageError::ArtifactInstallFailed { .. } => TakokitError::Resolution {
                code: ErrorCode::ArtifactInstallFailed,
                message: error.to_string(),
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
    pub family: String,
    pub version: String,
    pub kind: ModelKind,
    pub backend: ModelBackend,
    pub runner: String,
    #[serde(default)]
    pub required_adapter: Option<String>,
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
    OmniAudio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelBackend {
    Native,
    Onnx,
    Whispercpp,
    PythonManaged,
    TransformersAudio,
    Nemo,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactManifest {
    #[serde(default)]
    pub metadata_only: bool,
    #[serde(default)]
    pub weights: Vec<ArtifactEntry>,
    #[serde(default)]
    pub configs: Vec<ArtifactEntry>,
    #[serde(default)]
    pub voices: Vec<ArtifactEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub name: String,
    pub sha256: String,
    pub bytes: Option<u64>,
    pub url: Option<String>,
    #[serde(default)]
    pub role: ArtifactRole,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    Model,
    Config,
    Voice,
    Other,
}

impl Default for ArtifactRole {
    fn default() -> Self {
        Self::Model
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: RunnerKind,
    pub platforms: Vec<String>,
    #[serde(default)]
    pub supported_model_families: Vec<String>,
    #[serde(default)]
    pub supported_tasks: Vec<CapabilityKind>,
    #[serde(default)]
    pub dependency_strategy: RunnerDependencyStrategy,
    #[serde(default)]
    pub install_state: RunnerLifecycleState,
    #[serde(default)]
    pub notes: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RunnerKind {
    Native,
    Onnx,
    Whispercpp,
    PythonManaged,
    TransformersAudio,
    Nemo,
    External,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RunnerDependencyStrategy {
    BundledNative,
    Managed,
    ExternalToolchain,
    NotImplemented,
}

impl Default for RunnerDependencyStrategy {
    fn default() -> Self {
        Self::NotImplemented
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RunnerLifecycleState {
    RuntimeMissing,
    #[serde(alias = "metadata_only")]
    ContractInstalled,
    RuntimeInstalled,
    Ready,
    Failed,
}

impl Default for RunnerLifecycleState {
    fn default() -> Self {
        Self::RuntimeMissing
    }
}

impl RunnerLifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            RunnerLifecycleState::RuntimeMissing => "runtime-missing",
            RunnerLifecycleState::ContractInstalled => "contract-installed",
            RunnerLifecycleState::RuntimeInstalled => "runtime-installed",
            RunnerLifecycleState::Ready => "ready",
            RunnerLifecycleState::Failed => "failed",
        }
    }
}

impl std::fmt::Display for RunnerLifecycleState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelLifecycleState {
    MetadataOnly,
    ArtifactsReady,
    RunnerReady,
    Executable,
    Failed,
}

impl ModelLifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            ModelLifecycleState::MetadataOnly => "metadata-only",
            ModelLifecycleState::ArtifactsReady => "artifacts-ready",
            ModelLifecycleState::RunnerReady => "runner-ready",
            ModelLifecycleState::Executable => "executable",
            ModelLifecycleState::Failed => "failed",
        }
    }
}

impl std::fmt::Display for ModelLifecycleState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LibraryModelManifest {
    pub id: String,
    pub name: String,
    pub family: String,
    pub source_kind: LibrarySourceKind,
    pub base_model: Option<String>,
    pub upstream_url: String,
    pub huggingface_url: Option<String>,
    pub github_url: Option<String>,
    pub paper_url: Option<String>,
    pub license: String,
    pub commercial_use: CommercialUse,
    pub tasks: Vec<LibraryTask>,
    pub runner: String,
    pub runtime_status: LibraryRuntimeStatus,
    pub quality_tier: QualityTier,
    pub hardware_notes: String,
    pub languages: Vec<String>,
    pub notes: String,
    pub safety_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LibraryRunnerManifest {
    pub id: String,
    pub name: String,
    pub kind: RunnerKind,
    pub upstream_url: Option<String>,
    pub github_url: Option<String>,
    pub runtime_status: LibraryRuntimeStatus,
    pub supported_platforms: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LibrarySourceKind {
    Original,
    Fork,
    OptimizedExport,
    Quantized,
    Community,
    VoicePack,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CommercialUse {
    Yes,
    No,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryTask {
    Tts,
    Stt,
    VoiceCloning,
    VoiceConversion,
    LiveTranscription,
    LiveAudio,
    OmniAudio,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryRuntimeStatus {
    Supported,
    Experimental,
    Planned,
    MetadataOnly,
    BlockedLicense,
    ExternalRunnerNeeded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum QualityTier {
    Lightweight,
    Balanced,
    Sota,
    Research,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: RunnerKind,
    pub platforms: Vec<String>,
    pub supported_model_families: Vec<String>,
    pub supported_tasks: Vec<CapabilityKind>,
    pub dependency_strategy: RunnerDependencyStrategy,
    pub install_state: RunnerLifecycleState,
    pub notes: String,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstallModelOptions {
    pub metadata_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledModelRecord {
    pub id: String,
    pub version: String,
    pub source: String,
    pub manifest_path: PathBuf,
    pub runner: String,
    pub installed_at: String,
    pub artifacts: Vec<InstalledArtifactRecord>,
    pub status: InstalledPackageStatus,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledRunnerRecord {
    pub id: String,
    pub version: String,
    pub kind: String,
    pub manifest_path: PathBuf,
    pub installed_at: String,
    pub platforms: Vec<String>,
    pub status: RunnerLifecycleState,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledArtifactRecord {
    pub name: String,
    pub sha256: String,
    pub bytes: Option<u64>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub role: ArtifactRole,
    pub local_path: Option<PathBuf>,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstalledPackageStatus {
    MetadataOnly,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub model: ModelManifest,
    pub capability: CapabilityKind,
    pub runner: RunnerManifest,
    pub runner_installed: bool,
    pub status: ExecutionStatus,
    pub installed_model: Option<InstalledModelRecord>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelPlan {
    pub model_id: String,
    pub model_name: String,
    pub family: String,
    pub task: String,
    pub required_runner: String,
    pub lifecycle_state: ModelLifecycleState,
    pub artifact_state: ModelLifecycleState,
    pub runner_contract_state: RunnerLifecycleState,
    pub runner_runtime_state: RunnerLifecycleState,
    pub executable: bool,
    pub missing: Vec<String>,
    pub next_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonManagedRunnerLayout {
    pub root: PathBuf,
    pub runtime: PathBuf,
    pub env: PathBuf,
    pub packages: PathBuf,
    pub wheels: PathBuf,
    pub logs: PathBuf,
    pub manifests: PathBuf,
    pub cache: PathBuf,
    pub adapters: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterLifecycleState {
    NotInstalled,
    Installing,
    Ready,
    Failed,
}

impl std::fmt::Display for AdapterLifecycleState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NotInstalled => "not-installed",
            Self::Installing => "installing",
            Self::Ready => "ready",
            Self::Failed => "failed",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdapterRecord {
    pub id: String,
    pub model_family: String,
    pub state: AdapterLifecycleState,
    pub dependency_strategy: String,
    pub input_contract: String,
    pub output_contract: String,
    pub logs: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerRuntimeLayout {
    pub root: PathBuf,
    pub logs: PathBuf,
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

    pub fn installed_model_record(&self, id: &str) -> PackageResult<InstalledModelRecord> {
        std::fs::read_to_string(self.model_record_path(id))
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => PackageError::ModelNotInstalled(id.to_string()),
                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
            .or_else(|error| match error {
                PackageError::ModelNotInstalled(_) if self.model_manifest_path(id).is_file() => {
                    let manifest = self.installed_model(id)?;
                    Ok(installed_model_record(
                        &manifest,
                        self.model_manifest_path(id),
                    ))
                }
                _ => Err(error),
            })
    }

    pub fn installed_runner_record(&self, id: &str) -> PackageResult<InstalledRunnerRecord> {
        std::fs::read_to_string(self.runner_record_path(id))
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => {
                    PackageError::RunnerPackageNotInstalled(id.to_string())
                }
                _ => PackageError::Io(error),
            })
            .and_then(|source| Ok(toml::from_str(&source)?))
            .or_else(|error| match error {
                PackageError::RunnerPackageNotInstalled(_)
                    if self.runner_manifest_path(id).is_file() =>
                {
                    let source = std::fs::read_to_string(self.runner_manifest_path(id))?;
                    let manifest: RunnerManifest = toml::from_str(&source)?;
                    Ok(installed_runner_record(
                        &manifest,
                        self.runner_manifest_path(id),
                    ))
                }
                _ => Err(error),
            })
    }

    pub fn installed_model_records(&self) -> PackageResult<Vec<InstalledModelRecord>> {
        read_manifest_dir(&self.root.join("installed-models"))
    }

    pub fn installed_runner_records(&self) -> PackageResult<Vec<InstalledRunnerRecord>> {
        read_manifest_dir(&self.root.join("installed-runners"))
    }

    pub fn is_model_installed(&self, id: &str) -> bool {
        self.model_record_path(id).is_file() || self.model_manifest_path(id).is_file()
    }

    pub fn is_runner_installed(&self, id: &str) -> bool {
        self.runner_record_path(id).is_file() || self.runner_manifest_path(id).is_file()
    }

    pub fn install_model(&self, manifest: &ModelManifest) -> PackageResult<PullReport> {
        self.install_model_with_options(manifest, InstallModelOptions::default())
    }

    pub fn install_model_with_options(
        &self,
        manifest: &ModelManifest,
        options: InstallModelOptions,
    ) -> PackageResult<PullReport> {
        std::fs::create_dir_all(self.root.join("models"))?;
        std::fs::create_dir_all(self.root.join("installed-models"))?;
        let path = self.model_manifest_path(&manifest.id);
        let previous = self.installed_model_record(&manifest.id).ok();
        if options.metadata_only
            && previous
                .as_ref()
                .is_some_and(|record| record.status == InstalledPackageStatus::Ready)
        {
            return Ok(PullReport {
                id: manifest.id.clone(),
                installed: false,
                manifest_path: path,
                note: "Metadata-only request preserved the existing verified ready installation."
                    .to_string(),
            });
        }
        if options.metadata_only
            && previous
                .as_ref()
                .is_some_and(|record| record.status == InstalledPackageStatus::MetadataOnly)
        {
            return Ok(PullReport {
                id: manifest.id.clone(),
                installed: false,
                manifest_path: path,
                note: "Metadata-only model record is already present.".to_string(),
            });
        }
        let artifact_set = self.install_artifacts(manifest, options, previous.as_ref())?;
        self.materialize_model_artifacts(manifest, &artifact_set)?;
        let record = installed_model_record_with_artifacts(manifest, path.clone(), artifact_set);

        write_model_install_files(
            &path,
            &self.model_record_path(&manifest.id),
            &toml::to_string_pretty(manifest)?,
            &toml::to_string_pretty(&record)?,
        )?;

        Ok(PullReport {
            id: manifest.id.clone(),
            installed: true,
            manifest_path: path,
            note: record.note,
        })
    }

    pub fn install_runner(&self, manifest: &RunnerManifest) -> PackageResult<PullReport> {
        std::fs::create_dir_all(self.root.join("runners"))?;
        std::fs::create_dir_all(self.root.join("installed-runners"))?;
        let path = self.runner_manifest_path(&manifest.id);
        std::fs::write(&path, toml::to_string_pretty(manifest)?)?;
        let record = self
            .installed_runner_record(&manifest.id)
            .map(|mut record| {
                // Pulling a current manifest must never downgrade a ready or
                // failed runtime record back to a contract-only state.
                record.version = manifest.version.clone();
                record.kind = manifest.kind.as_str().to_string();
                record.manifest_path = path.clone();
                record.platforms = manifest.platforms.clone();
                record
            })
            .unwrap_or_else(|_| installed_runner_record(manifest, path.clone()));
        std::fs::write(
            self.runner_record_path(&manifest.id),
            toml::to_string_pretty(&record)?,
        )?;

        Ok(PullReport {
            id: manifest.id.clone(),
            installed: true,
            manifest_path: path,
            note: record.note,
        })
    }

    pub fn install_runner_runtime(
        &self,
        manifest: &RunnerManifest,
        status: RunnerLifecycleState,
        note: impl Into<String>,
    ) -> PackageResult<PullReport> {
        std::fs::create_dir_all(self.root.join("runners"))?;
        std::fs::create_dir_all(self.root.join("installed-runners"))?;
        let path = self.runner_manifest_path(&manifest.id);
        if !path.is_file() {
            std::fs::write(&path, toml::to_string_pretty(manifest)?)?;
        }
        let mut record = self
            .installed_runner_record(&manifest.id)
            .unwrap_or_else(|_| installed_runner_record(manifest, path.clone()));
        record.status = status;
        record.note = note.into();
        record.installed_at = timestamp_now();
        std::fs::write(
            self.runner_record_path(&manifest.id),
            toml::to_string_pretty(&record)?,
        )?;

        Ok(PullReport {
            id: manifest.id.clone(),
            installed: true,
            manifest_path: path,
            note: record.note,
        })
    }

    pub fn remove_model(&self, id: &str) -> PackageResult<bool> {
        let manifest_path = self.model_manifest_path(id);
        let record_path = self.model_record_path(id);
        if !manifest_path.exists() && !record_path.exists() {
            return Err(PackageError::ModelNotInstalled(id.to_string()));
        }

        remove_file_if_exists(manifest_path)?;
        remove_file_if_exists(record_path)?;
        Ok(true)
    }

    pub fn remove_runner(&self, id: &str) -> PackageResult<bool> {
        let manifest_path = self.runner_manifest_path(id);
        let record_path = self.runner_record_path(id);
        if !manifest_path.exists() && !record_path.exists() {
            return Err(PackageError::RunnerPackageNotInstalled(id.to_string()));
        }

        remove_file_if_exists(manifest_path)?;
        remove_file_if_exists(record_path)?;
        Ok(true)
    }

    fn model_manifest_path(&self, id: &str) -> PathBuf {
        self.root.join("models").join(format!("{id}.toml"))
    }

    fn runner_manifest_path(&self, id: &str) -> PathBuf {
        self.root.join("runners").join(format!("{id}.toml"))
    }

    fn model_record_path(&self, id: &str) -> PathBuf {
        self.root
            .join("installed-models")
            .join(format!("{id}.toml"))
    }

    fn runner_record_path(&self, id: &str) -> PathBuf {
        self.root
            .join("installed-runners")
            .join(format!("{id}.toml"))
    }

    pub fn storage_root(&self) -> PathBuf {
        self.root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.root.clone())
    }

    fn install_artifacts(
        &self,
        manifest: &ModelManifest,
        options: InstallModelOptions,
        previous: Option<&InstalledModelRecord>,
    ) -> PackageResult<InstalledArtifactSet> {
        let artifacts = manifest.artifacts.all().collect::<Vec<_>>();
        let metadata_only = options.metadata_only || manifest.artifacts.metadata_only;

        if artifacts.is_empty() || metadata_only {
            return Ok(InstalledArtifactSet {
                records: installed_artifacts(&manifest.artifacts),
                status: InstalledPackageStatus::MetadataOnly,
                note: if artifacts.is_empty() {
                    "Installed model metadata from local mock registry. No model weights were downloaded."
                        .to_string()
                } else {
                    "Installed model metadata from local mock registry. Artifact URLs are recorded, but downloads were skipped because this manifest is metadata-only."
                        .to_string()
                },
            });
        }

        let root = self.storage_root();
        let downloads_dir = root.join("cache").join("downloads");
        let blob_dir = root.join("blobs").join("sha256");
        std::fs::create_dir_all(&downloads_dir)?;
        std::fs::create_dir_all(&blob_dir)?;

        let mut records = Vec::new();
        for artifact in artifacts {
            let prior = previous.and_then(|record| {
                record
                    .artifacts
                    .iter()
                    .find(|record| record.name == artifact.name)
            });
            let local_path =
                match prior.filter(|record| artifact_reuse::is_verified(record, artifact)) {
                    Some(record) => record
                        .local_path
                        .clone()
                        .expect("verified artifact has path"),
                    None => install_artifact(manifest, artifact, &downloads_dir, &blob_dir)?,
                };
            records.push(InstalledArtifactRecord {
                name: artifact.name.clone(),
                sha256: artifact.sha256.clone(),
                bytes: artifact.bytes,
                url: artifact.url.clone(),
                role: artifact.role,
                local_path: Some(local_path),
                downloaded: true,
            });
        }

        Ok(InstalledArtifactSet {
            records,
            status: InstalledPackageStatus::Ready,
            note: "Installed model metadata and verified artifacts into content-addressed blobs."
                .to_string(),
        })
    }

    fn materialize_model_artifacts(
        &self,
        manifest: &ModelManifest,
        artifact_set: &InstalledArtifactSet,
    ) -> PackageResult<()> {
        if artifact_set.status != InstalledPackageStatus::Ready {
            return Ok(());
        }
        let model_dir = self.storage_root().join("models").join(&manifest.id);
        for artifact in &artifact_set.records {
            let relative = Path::new(&artifact.name);
            if relative.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            }) {
                return Err(PackageError::ArtifactInstallFailed {
                    artifact: artifact.name.clone(),
                    reason: "artifact name must be a safe relative path".to_string(),
                });
            }
            let source = artifact.local_path.as_ref().ok_or_else(|| {
                PackageError::ArtifactInstallFailed {
                    artifact: artifact.name.clone(),
                    reason: "downloaded artifact is missing its blob path".to_string(),
                }
            })?;
            let destination = model_dir.join(relative);
            let parent =
                destination
                    .parent()
                    .ok_or_else(|| PackageError::ArtifactInstallFailed {
                        artifact: artifact.name.clone(),
                        reason: "artifact path has no parent directory".to_string(),
                    })?;
            std::fs::create_dir_all(parent)?;
            if destination.is_file()
                && artifact.bytes.is_none_or(|expected| {
                    std::fs::metadata(&destination)
                        .map(|m| m.len() == expected)
                        .unwrap_or(false)
                })
                && sha256_file(&destination)
                    .map(|checksum| checksum == artifact.sha256)
                    .unwrap_or(false)
            {
                continue;
            }
            if destination.is_file() {
                std::fs::remove_file(&destination)?;
            }
            if std::fs::hard_link(source, &destination).is_err() {
                std::fs::copy(source, &destination)?;
            }
        }
        Ok(())
    }
}

impl ModelManifest {
    pub fn supports(&self, capability: CapabilityKind) -> bool {
        self.capabilities.supports(capability)
    }

    pub fn to_model_info(&self, installed: bool, runner_installed: bool) -> ModelInfo {
        let fallback = ModelPlan {
            model_id: self.id.clone(),
            model_name: self.name.clone(),
            family: self.family.clone(),
            task: model_task_label(self),
            required_runner: self.runner.clone(),
            lifecycle_state: if installed {
                ModelLifecycleState::ArtifactsReady
            } else {
                ModelLifecycleState::MetadataOnly
            },
            artifact_state: if installed {
                ModelLifecycleState::ArtifactsReady
            } else {
                ModelLifecycleState::MetadataOnly
            },
            runner_contract_state: if runner_installed {
                RunnerLifecycleState::ContractInstalled
            } else {
                RunnerLifecycleState::RuntimeMissing
            },
            runner_runtime_state: if runner_installed {
                RunnerLifecycleState::ContractInstalled
            } else {
                RunnerLifecycleState::RuntimeMissing
            },
            executable: false,
            missing: if installed {
                vec![format!("runner contract: {}", self.runner)]
            } else {
                vec!["verified artifacts".to_string()]
            },
            next_command: if installed {
                format!("takokit runner pull {}", self.runner)
            } else {
                format!("takokit pull {}", self.id)
            },
        };
        self.to_model_info_from_plan(&fallback, installed, runner_installed)
    }

    pub fn to_model_info_from_plan(
        &self,
        plan: &ModelPlan,
        installed: bool,
        runner_installed: bool,
    ) -> ModelInfo {
        ModelInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            family: self.family.clone(),
            version: self.version.clone(),
            summary: self.description.clone(),
            license: self.license.clone(),
            license_warning: license_warning(&self.license),
            runtime: self.backend.to_model_runtime(),
            backend: self.backend.as_str().to_string(),
            runner: self.runner.clone(),
            hardware_notes: self.hardware.notes(),
            artifact_count: self.artifacts.all().count(),
            capabilities: self.capabilities.to_model_capabilities(),
            installed,
            runner_installed,
            runner_runtime_state: plan.runner_runtime_state.to_string(),
            lifecycle_state: plan.lifecycle_state.to_string(),
            executable: plan.executable,
            missing: plan.missing.clone(),
            next_command: plan.next_command.clone(),
            execution_status: model_execution_status(plan),
        }
    }
}

impl RunnerManifest {
    pub fn to_runner_info(&self, installed: bool) -> RunnerInfo {
        self.to_runner_info_with_state(
            installed,
            if installed {
                RunnerLifecycleState::ContractInstalled
            } else {
                self.install_state
            },
        )
    }

    pub fn to_runner_info_with_state(
        &self,
        installed: bool,
        install_state: RunnerLifecycleState,
    ) -> RunnerInfo {
        RunnerInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            kind: self.kind.clone(),
            platforms: self.platforms.clone(),
            supported_model_families: self.supported_model_families.clone(),
            supported_tasks: self.supported_tasks.clone(),
            dependency_strategy: self.dependency_strategy,
            install_state,
            notes: self.notes.clone(),
            description: self.description.clone(),
            installed,
        }
    }
}

impl RunnerKind {
    fn as_str(&self) -> &'static str {
        match self {
            RunnerKind::Native => "native",
            RunnerKind::Onnx => "onnx",
            RunnerKind::Whispercpp => "whispercpp",
            RunnerKind::PythonManaged => "python-managed",
            RunnerKind::TransformersAudio => "transformers-audio",
            RunnerKind::Nemo => "nemo",
            RunnerKind::External => "external",
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

impl ArtifactManifest {
    pub fn all(&self) -> impl Iterator<Item = &ArtifactEntry> {
        self.weights
            .iter()
            .chain(self.configs.iter())
            .chain(self.voices.iter())
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
            ModelBackend::TransformersAudio => "transformers-audio",
            ModelBackend::Nemo => "nemo",
            ModelBackend::External => "external",
        }
    }

    fn to_model_runtime(&self) -> ModelRuntime {
        match self {
            ModelBackend::Native => ModelRuntime::NativeRust,
            ModelBackend::Onnx => ModelRuntime::Onnx,
            ModelBackend::Whispercpp => ModelRuntime::WhisperCpp,
            ModelBackend::PythonManaged => ModelRuntime::Python,
            ModelBackend::TransformersAudio => ModelRuntime::External,
            ModelBackend::Nemo => ModelRuntime::External,
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

pub fn resolve_execution_plan(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
    capability: CapabilityKind,
) -> PackageResult<ExecutionPlan> {
    let model = package_registry.model(model_id)?;
    let installed_model = if model.id == "mock-tts" {
        None
    } else if installed_registry.is_model_installed(&model.id) {
        Some(installed_registry.installed_model_record(&model.id)?)
    } else {
        return Err(PackageError::ModelNotInstalled(model.id));
    };

    if !model.supports(capability) {
        return Err(PackageError::CapabilityUnsupported {
            model: model.id,
            capability,
            capability_label: capability.label(),
        });
    }

    let runner = package_registry.runner(&model.runner)?;
    let platform = current_platform_id();
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
        status: ExecutionStatus::Planned,
        installed_model,
    })
}

pub fn resolve_runner(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
    capability: CapabilityKind,
) -> PackageResult<ExecutionPlan> {
    resolve_execution_plan(package_registry, installed_registry, model_id, capability)
}

pub fn model_info_from_plan(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
) -> PackageResult<ModelInfo> {
    let model = package_registry.model(model_id)?;
    let plan = plan_model(package_registry, installed_registry, model_id)?;
    Ok(model.to_model_info_from_plan(
        &plan,
        installed_registry.is_model_installed(&model.id),
        installed_registry.is_runner_installed(&model.runner),
    ))
}

pub fn plan_model(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_id: &str,
) -> PackageResult<ModelPlan> {
    let model = package_registry.model(model_id)?;
    let runner = package_registry.runner(&model.runner)?;
    let installed_model = installed_registry.installed_model_record(&model.id).ok();
    let installed_runner = installed_registry.installed_runner_record(&runner.id).ok();

    let artifact_state = model_artifact_state(&model, installed_model.as_ref());
    let runner_contract_state = if installed_runner.is_some() {
        RunnerLifecycleState::ContractInstalled
    } else {
        RunnerLifecycleState::RuntimeMissing
    };
    let runner_runtime_state = installed_runner
        .as_ref()
        .map(|record| record.status)
        .unwrap_or(RunnerLifecycleState::RuntimeMissing);
    let adapter = model.required_adapter.as_deref().and_then(|id| {
        python_adapter_record(&installed_registry.storage_root(), id)
            .ok()
            .map(|record| (id, record.state))
    });
    let adapter_ready = adapter
        .as_ref()
        .is_some_and(|(_, state)| *state == AdapterLifecycleState::Ready);
    let lifecycle_state = model_lifecycle_state(
        &model,
        &runner,
        artifact_state,
        runner_runtime_state,
        adapter_ready,
    );
    let executable = lifecycle_state == ModelLifecycleState::Executable;
    let mut missing = Vec::new();

    if matches!(artifact_state, ModelLifecycleState::MetadataOnly) {
        missing.push("verified artifacts".to_string());
    }
    if runner_contract_state == RunnerLifecycleState::RuntimeMissing {
        missing.push(format!("runner contract: {}", runner.id));
    }
    if runner_runtime_state != RunnerLifecycleState::Ready {
        missing.push(runner_missing_component(&model, &runner));
    }
    if let Some((adapter, state)) = adapter.as_ref() {
        if *state != AdapterLifecycleState::Ready {
            missing.push(format!("managed adapter {adapter} ({state})"));
        }
    }
    if lifecycle_state == ModelLifecycleState::RunnerReady {
        missing.push(runner_missing_component(&model, &runner));
    }
    if executable {
        missing.clear();
    }

    Ok(ModelPlan {
        model_id: model.id.clone(),
        model_name: model.name.clone(),
        family: model.family.clone(),
        task: model_task_label(&model),
        required_runner: runner.id.clone(),
        lifecycle_state,
        artifact_state,
        runner_contract_state,
        runner_runtime_state,
        executable,
        missing,
        next_command: next_plan_command(
            &model,
            installed_model.is_some(),
            runner_runtime_state,
            adapter,
            executable,
        ),
    })
}

pub fn python_managed_runner_layout(takokit_root: &Path) -> PythonManagedRunnerLayout {
    let root = takokit_root.join("runners").join("python-managed");
    PythonManagedRunnerLayout {
        runtime: root.join("runtime"),
        env: root.join("env"),
        packages: root.join("packages"),
        wheels: root.join("wheels"),
        logs: root.join("logs"),
        manifests: root.join("manifests"),
        cache: root.join("cache"),
        adapters: root.join("adapters"),
        root,
    }
}

pub fn runner_runtime_layout(
    takokit_root: &Path,
    manifest: &RunnerManifest,
) -> RunnerRuntimeLayout {
    let root = if manifest.id == "takokit-python-managed" {
        python_managed_runner_layout(takokit_root).root
    } else {
        let suffix = manifest.id.strip_prefix("takokit-").unwrap_or(&manifest.id);
        takokit_root.join("runners").join(suffix)
    };

    RunnerRuntimeLayout {
        logs: root.join("logs"),
        root,
    }
}

pub fn initialize_runner_runtime(
    takokit_root: &Path,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) -> PackageResult<PullReport> {
    let layout = runner_runtime_layout(takokit_root, manifest);
    std::fs::create_dir_all(&layout.logs)?;

    match manifest.kind {
        RunnerKind::PythonManaged => {
            match install_python_managed_runtime(takokit_root, installed_registry, manifest) {
                Ok(report) => Ok(report),
                Err(error) => {
                    let _ = installed_registry.install_runner_runtime(
                        manifest,
                        RunnerLifecycleState::Failed,
                        format!(
                            "Managed Python runtime install failed: {error}. Logs: {}",
                            layout.logs.display()
                        ),
                    );
                    Err(error)
                }
            }
        }
        RunnerKind::Onnx => match install_onnx_runtime(installed_registry, manifest, &layout) {
            Ok(report) => Ok(report),
            Err(error) => {
                let _ = installed_registry.install_runner_runtime(
                    manifest,
                    RunnerLifecycleState::Failed,
                    format!(
                        "ONNX runtime install failed: {error}. Logs: {}",
                        layout.logs.display()
                    ),
                );
                Err(error)
            }
        },
        RunnerKind::Whispercpp => match install_whispercpp_runtime(installed_registry, manifest, &layout) {
            Ok(report) => Ok(report),
            Err(error) => {
                let _ = installed_registry.install_runner_runtime(
                    manifest,
                    RunnerLifecycleState::Failed,
                    format!(
                        "whisper.cpp runtime install failed: {error}. Logs: {}",
                        layout.logs.display()
                    ),
                );
                Err(error)
            }
        },
        RunnerKind::TransformersAudio => installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::RuntimeInstalled,
            format!(
                "Transformers audio runner runtime directory initialized at {}. Missing component: managed transformers audio adapter.",
                layout.root.display()
            ),
        ),
        RunnerKind::Nemo => installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::RuntimeInstalled,
            format!(
                "NeMo runner runtime directory initialized at {}. Missing component: NeMo adapter and managed dependencies.",
                layout.root.display()
            ),
        ),
        RunnerKind::Native | RunnerKind::External => installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::Failed,
            "No runtime installer is defined for this runner kind.",
        ),
    }
}

/// Returns the uv executable Takokit will use.  Runner installation must never
/// fall back to PATH: that makes a successful bootstrap depend on a shell that
/// happened to be open when it ran.
pub fn find_uv(takokit_root: &Path) -> Option<PathBuf> {
    let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
    let managed = takokit_root.join("tools").join("uv").join(executable);
    if managed.is_file() {
        return Some(managed);
    }
    if let Ok(path) = std::env::var("UV") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn find_uv_bootstrap_source() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("UV") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|entry| entry.join(executable))
        .find(|candidate| candidate.is_file())
}

/// Attempts a Windows-first uv bootstrap and writes all output to the Takokit
/// log directory. It deliberately returns an error instead of allowing a
/// runner to be marked ready when bootstrap cannot complete.
pub fn bootstrap_uv(takokit_root: &Path) -> PackageResult<PathBuf> {
    let logs = takokit_root.join("logs");
    std::fs::create_dir_all(&logs)?;
    let log = logs.join("uv-bootstrap.log");
    let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
    let managed = takokit_root.join("tools").join("uv").join(executable);
    if managed.is_file() && verify_uv_version(&managed, &log)? {
        return Ok(managed);
    }
    let source = find_uv_bootstrap_source().ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: "uv bootstrap".to_string(),
        reason: format!(
            "no uv bootstrap source was found. Set UV to a pinned uv {} binary, then rerun. See {}",
            TAKOKIT_UV_VERSION,
            log.display()
        ),
    })?;
    std::fs::create_dir_all(managed.parent().expect("managed uv parent"))?;
    std::fs::copy(&source, &managed)?;
    let source_hash =
        sha256_file(&source).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: error.to_string(),
        })?;
    std::fs::write(&log, format!(
        "Takokit managed uv bootstrap\nsource: {}\nmanaged_path: {}\nrequested_version: {}\nsha256: {}\n",
        source.display(), managed.display(), TAKOKIT_UV_VERSION, source_hash
    ))?;
    if verify_uv_version(&managed, &log)? {
        Ok(managed)
    } else {
        let _ = std::fs::remove_file(&managed);
        Err(PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!(
                "managed uv does not report pinned version {}; see {}",
                TAKOKIT_UV_VERSION,
                log.display()
            ),
        })
    }
}

fn verify_uv_version(path: &Path, log: &Path) -> PackageResult<bool> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "uv bootstrap".to_string(),
            reason: format!("could not run {}: {error}", path.display()),
        })?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)?
        .write_all(
            format!(
                "verified_command: {} --version\nreported_version: {}\n",
                path.display(),
                version
            )
            .as_bytes(),
        )?;
    Ok(output.status.success() && version.starts_with(&format!("uv {TAKOKIT_UV_VERSION}")))
}

fn write_python_adapter_manifests(layout: &PythonManagedRunnerLayout) -> PackageResult<()> {
    for (adapter, model_family) in PYTHON_MANAGED_ADAPTERS {
        let adapter_dir = layout.adapters.join(adapter);
        std::fs::create_dir_all(&adapter_dir)?;
        let manifest = adapter_dir.join("adapter.toml");
        if !manifest.is_file() {
            write_adapter_record(
                &manifest,
                &AdapterRecord {
                    id: (*adapter).to_string(),
                    model_family: (*model_family).to_string(),
                    state: AdapterLifecycleState::NotInstalled,
                    dependency_strategy: "takokit-managed-python".to_string(),
                    input_contract: "json request on stdin".to_string(),
                    output_contract: "json response on stdout".to_string(),
                    logs: "../../logs".to_string(),
                    notes: "Adapter slot only. Takokit has not installed Python dependencies or model weights for this adapter.".to_string(),
                },
            )?;
        }
    }
    Ok(())
}

pub fn python_adapter_records(takokit_root: &Path) -> PackageResult<Vec<AdapterRecord>> {
    let layout = python_managed_runner_layout(takokit_root);
    let mut records = Vec::new();
    if !layout.adapters.is_dir() {
        return Ok(records);
    }
    let mut entries = std::fs::read_dir(&layout.adapters)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path().join("adapter.toml");
        if path.is_file() {
            records.push(toml::from_str(&std::fs::read_to_string(path)?)?);
        }
    }
    Ok(records)
}

pub fn python_adapter_record(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let path = python_managed_runner_layout(takokit_root)
        .adapters
        .join(adapter)
        .join("adapter.toml");
    std::fs::read_to_string(&path)
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => PackageError::ArtifactInstallFailed {
                artifact: adapter.to_string(),
                reason: format!("adapter is not available; run `takokit runner install takokit-python-managed`: {}", path.display()),
            },
            _ => PackageError::Io(error),
        })
        .and_then(|source| Ok(toml::from_str(&source)?))
}

pub fn install_python_adapter(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let layout = python_managed_runner_layout(takokit_root);
    write_python_adapter_manifests(&layout)?;
    let manifest_path = layout.adapters.join(adapter).join("adapter.toml");
    let mut record = python_adapter_record(takokit_root, adapter)?;
    record.state = AdapterLifecycleState::Installing;
    record.notes =
        "Takokit is installing this adapter in the managed Python environment.".to_string();
    write_adapter_record(&manifest_path, &record)?;

    let result = match adapter {
        "qwen3_tts" => install_qwen3_tts_adapter(&layout),
        _ => Err(PackageError::ArtifactInstallFailed {
            artifact: adapter.to_string(),
            reason: "no executable adapter has been verified for this model family yet; Takokit left the adapter in not-installed state.".to_string(),
        }),
    };
    match result {
        Ok(note) => {
            record.state = AdapterLifecycleState::Ready;
            record.notes = note;
            write_adapter_record(&manifest_path, &record)?;
            Ok(record)
        }
        Err(error) => {
            record.state = AdapterLifecycleState::Failed;
            record.notes = format!("Adapter install failed: {error}");
            write_adapter_record(&manifest_path, &record)?;
            Err(error)
        }
    }
}

pub fn adapter_for_model(model_id: &str) -> Option<&'static str> {
    PYTHON_MANAGED_ADAPTERS
        .iter()
        .find(|(_, family)| *family == model_id)
        .map(|(adapter, _)| *adapter)
}

fn install_python_managed_runtime(
    takokit_root: &Path,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) -> PackageResult<PullReport> {
    let layout = python_managed_runner_layout(takokit_root);
    for path in [
        &layout.root,
        &layout.runtime,
        &layout.env,
        &layout.packages,
        &layout.wheels,
        &layout.logs,
        &layout.manifests,
        &layout.cache,
        &layout.adapters,
    ] {
        std::fs::create_dir_all(path)?;
    }
    write_python_adapter_manifests(&layout)?;
    let venv = layout.env.join("venv");
    let log = layout.logs.join("runtime-install.log");
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            "3.12".into(),
            "--allow-existing".into(),
            venv.clone().into(),
        ],
    )?;
    let python = runner_python_path(&venv).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: "managed Python runtime".to_string(),
        reason: format!(
            "uv created no Python executable below {}; see {}",
            venv.display(),
            log.display()
        ),
    })?;
    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "Managed Python runtime is ready at {} using {}. No model adapter is installed yet; use `takokit adapter install qwen3_tts`. Runtime log: {}",
            layout.root.display(),
            python.display(),
            log.display()
        ),
    )
}

fn install_qwen3_tts_adapter(layout: &PythonManagedRunnerLayout) -> PackageResult<String> {
    let venv = layout.env.join("venv");
    let python = runner_python_path(&venv).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: "qwen3_tts adapter".to_string(),
        reason: format!("managed Python runtime is missing; run `takokit runner install takokit-python-managed` first (expected {})", venv.display()),
    })?;
    let adapter_dir = layout.adapters.join("qwen3_tts");
    std::fs::create_dir_all(&adapter_dir)?;
    let log = adapter_dir.join("install.log");
    let takokit_root = layout.root.parent().and_then(Path::parent).ok_or_else(|| {
        PackageError::ArtifactInstallFailed {
            artifact: "qwen3_tts adapter".to_string(),
            reason: "cannot resolve Takokit storage root".to_string(),
        }
    })?;
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log,
        &uv,
        &[
            "pip".into(),
            "install".into(),
            "--python".into(),
            python.into(),
            "--no-progress".into(),
            QWEN3_TTS_PACKAGE.into(),
            "soundfile".into(),
        ],
    )?;
    std::fs::write(adapter_dir.join("qwen3_tts.py"), QWEN3_TTS_ADAPTER)?;
    Ok(format!(
        "Ready. Takokit installed {QWEN3_TTS_PACKAGE} and the JSON adapter. Model artifacts are pulled separately by `takokit pull qwen3-tts`. Install log: {}",
        log.display()
    ))
}

fn write_adapter_record(path: &Path, record: &AdapterRecord) -> PackageResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: record.id.clone(),
            reason: "adapter manifest path has no parent directory".to_string(),
        })?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, toml::to_string_pretty(record)?)?;
    Ok(())
}

fn install_whispercpp_runtime(
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
    layout: &RunnerRuntimeLayout,
) -> PackageResult<PullReport> {
    let runtime_dir = layout.root.join("runtime");
    let downloads_dir = layout.root.join("cache").join("downloads");
    std::fs::create_dir_all(&runtime_dir)?;
    std::fs::create_dir_all(&downloads_dir)?;

    if !(cfg!(target_os = "windows") && cfg!(target_arch = "x86_64")) {
        return installed_registry.install_runner_runtime(
            manifest,
            RunnerLifecycleState::RuntimeInstalled,
            format!(
                "whisper.cpp runtime directory initialized at {}. Automatic binary installation is currently implemented for Windows x64 only.",
                layout.root.display()
            ),
        );
    }

    let archive_path = downloads_dir.join("whisper-bin-x64-v1.9.1.zip");
    if !archive_path.is_file() {
        download_to_temp(WHISPERCPP_WIN_X64_URL, "whisper-bin-x64.zip", &archive_path)?;
    }
    let actual =
        sha256_file(&archive_path).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "whisper-bin-x64.zip".to_string(),
            reason: error.to_string(),
        })?;
    if actual != WHISPERCPP_WIN_X64_SHA256 {
        let _ = std::fs::remove_file(&archive_path);
        return Err(PackageError::ArtifactChecksumMismatch {
            artifact: "whisper-bin-x64.zip".to_string(),
            expected: WHISPERCPP_WIN_X64_SHA256.to_string(),
            actual,
        });
    }

    extract_zip_safely(&archive_path, &runtime_dir, "whisper-bin-x64.zip")?;
    let binary =
        find_file_named(&runtime_dir, executable_name("whisper-cli")).ok_or_else(|| {
            PackageError::ArtifactInstallFailed {
                artifact: "whisper-bin-x64.zip".to_string(),
                reason: "archive did not contain whisper-cli executable".to_string(),
            }
        })?;

    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "whisper.cpp v1.9.1 runtime installed at {}. Executable: {}",
            runtime_dir.display(),
            binary.display()
        ),
    )
}

fn install_onnx_runtime(
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
    layout: &RunnerRuntimeLayout,
) -> PackageResult<PullReport> {
    let runtime_dir = layout.root.join("runtime");
    let venv_dir = runtime_dir.join("venv");
    let adapters_dir = layout.root.join("adapters");
    let log_path = layout.logs.join("install-kokoro-onnx.log");
    std::fs::create_dir_all(&runtime_dir)?;
    std::fs::create_dir_all(&adapters_dir)?;

    let takokit_root = layout.root.parent().and_then(Path::parent).ok_or_else(|| {
        PackageError::ArtifactInstallFailed {
            artifact: "kokoro-onnx runtime".to_string(),
            reason: "cannot resolve Takokit storage root".to_string(),
        }
    })?;
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_command(
        &log_path,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            "3.12".into(),
            "--allow-existing".into(),
            venv_dir.clone().into(),
        ],
    )?;
    let python =
        runner_python_path(&venv_dir).ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: "kokoro-onnx runtime".to_string(),
            reason: format!(
                "uv created no Python executable below {}; see {}",
                venv_dir.display(),
                log_path.display()
            ),
        })?;
    run_logged_command(
        &log_path,
        &uv,
        &[
            "pip".into(),
            "install".into(),
            "--python".into(),
            python.clone().into(),
            "--no-progress".into(),
            KOKORO_ONNX_PACKAGE.into(),
        ],
    )?;
    std::fs::write(adapters_dir.join("kokoro.py"), KOKORO_ONNX_ADAPTER)?;

    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "Kokoro ONNX runtime is ready at {} using Python {} and {}. Piper remains blocked by the typed piper_text_frontend_not_implemented boundary. Install log: {}",
            layout.root.display(),
            python.display(),
            KOKORO_ONNX_PACKAGE,
            log_path.display()
        ),
    )
}

fn runner_python_path(venv_dir: &Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![venv_dir.join("Scripts").join("python.exe")]
    } else {
        vec![
            venv_dir.join("bin").join("python3"),
            venv_dir.join("bin").join("python"),
        ]
    };
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn run_logged_command(
    log_path: &Path,
    program: impl AsRef<Path>,
    args: &[PathOrArg],
) -> PackageResult<()> {
    let program = program.as_ref();
    let mut command = Command::new(program);
    for arg in args {
        command.arg(arg.as_os_str());
    }
    let output = command
        .output()
        .map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "managed runtime command".to_string(),
            reason: format!(
                "could not start {}: {error}; see {}",
                program.display(),
                log_path.display()
            ),
        })?;
    let mut log = String::new();
    log.push_str(&format!("$ {}", program.display()));
    for arg in args {
        log.push(' ');
        log.push_str(&arg.as_os_str().to_string_lossy());
    }
    log.push('\n');
    log.push_str(&String::from_utf8_lossy(&output.stdout));
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    log.push('\n');
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?
        .write_all(log.as_bytes())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(PackageError::ArtifactInstallFailed {
            artifact: "managed runtime command".to_string(),
            reason: format!(
                "{} exited with {}; see {}",
                program.display(),
                output.status,
                log_path.display()
            ),
        })
    }
}

#[derive(Debug, Clone)]
enum PathOrArg {
    Arg(String),
    Path(PathBuf),
}

impl From<&str> for PathOrArg {
    fn from(value: &str) -> Self {
        Self::Arg(value.to_string())
    }
}

impl From<String> for PathOrArg {
    fn from(value: String) -> Self {
        Self::Arg(value)
    }
}

impl From<PathBuf> for PathOrArg {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl PathOrArg {
    fn as_os_str(&self) -> &std::ffi::OsStr {
        match self {
            Self::Arg(value) => value.as_ref(),
            Self::Path(value) => value.as_os_str(),
        }
    }
}

fn installed_model_record(
    manifest: &ModelManifest,
    manifest_path: PathBuf,
) -> InstalledModelRecord {
    installed_model_record_with_artifacts(
        manifest,
        manifest_path,
        InstalledArtifactSet {
            records: installed_artifacts(&manifest.artifacts),
            status: InstalledPackageStatus::MetadataOnly,
            note:
                "Installed model metadata from local mock registry. No model weights were downloaded."
                    .to_string(),
        },
    )
}

fn installed_model_record_with_artifacts(
    manifest: &ModelManifest,
    manifest_path: PathBuf,
    artifacts: InstalledArtifactSet,
) -> InstalledModelRecord {
    InstalledModelRecord {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        source: "local-mock-registry".to_string(),
        manifest_path,
        runner: manifest.runner.clone(),
        installed_at: timestamp_now(),
        artifacts: artifacts.records,
        status: artifacts.status,
        note: artifacts.note,
    }
}

fn installed_runner_record(
    manifest: &RunnerManifest,
    manifest_path: PathBuf,
) -> InstalledRunnerRecord {
    InstalledRunnerRecord {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        kind: manifest.kind.as_str().to_string(),
        manifest_path,
        installed_at: timestamp_now(),
        platforms: manifest.platforms.clone(),
        status: RunnerLifecycleState::ContractInstalled,
        note: runner_contract_note(manifest).to_string(),
    }
}

fn runner_contract_note(manifest: &RunnerManifest) -> &'static str {
    match manifest.kind {
        RunnerKind::Whispercpp => {
            "Installed runner contract from local registry. Run `takokit runner install takokit-whispercpp` to install or verify the whisper.cpp runtime."
        }
        RunnerKind::Onnx => {
            "Installed runner contract from local registry. Run `takokit runner install takokit-onnx` to initialize the ONNX runner; Piper remains blocked on a verified text frontend."
        }
        RunnerKind::PythonManaged => {
            "Installed runner contract from local registry. Run `takokit runner install takokit-python-managed` to initialize the managed Python layout and adapter slots."
        }
        RunnerKind::TransformersAudio => {
            "Installed runner contract from local registry. Runtime adapter is planned and not installable yet."
        }
        RunnerKind::Nemo => {
            "Installed runner contract from local registry. NeMo runtime adapter is planned and not installable yet."
        }
        RunnerKind::Native => {
            "Installed native runner contract from local registry. Run runner doctor for current runtime readiness."
        }
        RunnerKind::External => {
            "Installed external runner contract from local registry. Run runner doctor for current runtime readiness."
        }
    }
}

fn installed_artifacts(manifest: &ArtifactManifest) -> Vec<InstalledArtifactRecord> {
    manifest
        .all()
        .map(|artifact| InstalledArtifactRecord {
            name: artifact.name.clone(),
            sha256: artifact.sha256.clone(),
            bytes: artifact.bytes,
            url: artifact.url.clone(),
            role: artifact.role,
            local_path: None,
            downloaded: false,
        })
        .collect()
}

fn write_model_install_files(
    manifest_path: &Path,
    record_path: &Path,
    manifest_toml: &str,
    record_toml: &str,
) -> PackageResult<()> {
    let manifest_tmp = sibling_temp_path(manifest_path);
    let record_tmp = sibling_temp_path(record_path);
    std::fs::write(&manifest_tmp, manifest_toml)?;
    std::fs::write(&record_tmp, record_toml).map_err(|error| {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&record_tmp);
        PackageError::Io(error)
    })?;

    let manifest_backup = backup_existing_file(manifest_path).map_err(|error| {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&record_tmp);
        error
    })?;
    if let Err(error) = std::fs::rename(&manifest_tmp, manifest_path) {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&record_tmp);
        restore_backup(manifest_path, manifest_backup);
        return Err(PackageError::Io(error));
    }

    let record_backup = match backup_existing_file(record_path) {
        Ok(backup) => backup,
        Err(error) => {
            let _ = std::fs::remove_file(&record_tmp);
            let _ = std::fs::remove_file(manifest_path);
            restore_backup(manifest_path, manifest_backup);
            return Err(error);
        }
    };
    if let Err(error) = std::fs::rename(&record_tmp, record_path) {
        let _ = std::fs::remove_file(&record_tmp);
        let _ = std::fs::remove_file(manifest_path);
        restore_backup(manifest_path, manifest_backup);
        restore_backup(record_path, record_backup);
        return Err(PackageError::Io(error));
    }

    remove_backup(manifest_backup);
    remove_backup(record_backup);
    Ok(())
}

fn sibling_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("install");
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| timestamp_now());
    path.with_file_name(format!("{file_name}.{suffix}.tmp"))
}

fn backup_existing_file(path: &Path) -> PackageResult<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    let backup_path = sibling_temp_path(path).with_extension("bak");
    std::fs::rename(path, &backup_path)?;
    Ok(Some(backup_path))
}

fn restore_backup(path: &Path, backup: Option<PathBuf>) {
    if let Some(backup) = backup {
        let _ = std::fs::rename(backup, path);
    }
}

fn remove_backup(backup: Option<PathBuf>) {
    if let Some(backup) = backup {
        let _ = std::fs::remove_file(backup);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledArtifactSet {
    records: Vec<InstalledArtifactRecord>,
    status: InstalledPackageStatus,
    note: String,
}

fn timestamp_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn remove_file_if_exists(path: PathBuf) -> PackageResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PackageError::Io(error)),
    }
}

pub fn current_platform_id() -> String {
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
family = "kokoro"
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
kind = "onnx"
platforms = ["windows-x64", "linux-x64", "macos-arm64"]
description = "Native ONNX runner for CPU-friendly models."
"#;

    const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    #[test]
    fn parses_model_manifest() {
        let manifest: ModelManifest = toml::from_str(MODEL_TOML).expect("model manifest");

        assert_eq!(manifest.id, "kokoro");
        assert_eq!(manifest.family, "kokoro");
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
        assert_eq!(manifest.kind, RunnerKind::Onnx);
        assert_eq!(
            manifest.platforms,
            vec!["windows-x64", "linux-x64", "macos-arm64"]
        );
    }

    #[test]
    fn parses_artifact_manifest_with_model_and_config_roles() {
        let source = format!(
            r#"
{MODEL_TOML}

[[artifacts.weights]]
name = "en_US-lessac-medium.onnx"
url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
sha256 = "{HELLO_SHA256}"
bytes = 63200000
role = "model"

[[artifacts.configs]]
name = "en_US-lessac-medium.onnx.json"
url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"
sha256 = "{HELLO_SHA256}"
bytes = 4890
role = "config"
"#
        )
        .replace("[artifacts]\nweights = []\nvoices = []", "[artifacts]");
        let manifest: ModelManifest = toml::from_str(&source).expect("model manifest");

        assert_eq!(manifest.artifacts.weights[0].role, ArtifactRole::Model);
        assert_eq!(manifest.artifacts.configs[0].role, ArtifactRole::Config);
        assert_eq!(manifest.artifacts.all().count(), 2);
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
        let manifest = registry.model("kokoro").expect("model");
        installed.install_model(&manifest).expect("install model");

        let error = resolve_execution_plan(
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
    fn resolver_reports_model_not_installed_before_unsupported_capability() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));

        let error = resolve_execution_plan(
            &registry,
            &installed,
            "kokoro",
            CapabilityKind::SpeechToText,
        )
        .expect_err("model not installed");

        assert!(matches!(error, PackageError::ModelNotInstalled(id) if id == "kokoro"));
    }

    #[test]
    fn install_model_writes_installed_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));
        let manifest = registry.model("kokoro").expect("model");

        let report = installed.install_model(&manifest).expect("install model");
        let record = installed
            .installed_model_record("kokoro")
            .expect("installed model record");

        assert_eq!(report.id, "kokoro");
        assert_eq!(record.id, "kokoro");
        assert_eq!(record.version, "0.1.0");
        assert_eq!(record.runner, "takokit-onnx");
        assert_eq!(record.source, "local-mock-registry");
        assert_eq!(record.status, InstalledPackageStatus::MetadataOnly);
        assert!(record.manifest_path.ends_with("models/kokoro.toml"));
    }

    #[test]
    fn install_model_missing_artifact_checksum_returns_typed_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("fixture.onnx");
        std::fs::write(&source, b"hello").expect("fixture");
        let manifest = artifact_test_manifest(&source, "");
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        let error = installed
            .install_model_with_options(&manifest, InstallModelOptions::default())
            .expect_err("missing checksum");

        assert!(matches!(
            error,
            PackageError::ArtifactChecksumMissing { model, artifact }
                if model == "piper-lessac" && artifact == "fixture.onnx"
        ));
    }

    #[test]
    fn checksum_mismatch_deletes_temporary_download() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("fixture.onnx");
        std::fs::write(&source, b"hello").expect("fixture");
        let manifest = artifact_test_manifest(&source, "0000");
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        let error = installed
            .install_model_with_options(&manifest, InstallModelOptions::default())
            .expect_err("checksum mismatch");

        assert!(matches!(
            error,
            PackageError::ArtifactChecksumMismatch { artifact, .. } if artifact == "fixture.onnx"
        ));

        let downloads = temp.path().join("cache").join("downloads");
        let leftovers = std::fs::read_dir(downloads)
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn checksum_mismatch_does_not_leave_installed_model_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("fixture.onnx");
        std::fs::write(&source, b"hello").expect("fixture");
        let manifest = artifact_test_manifest(&source, "0000");
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        let error = installed
            .install_model_with_options(&manifest, InstallModelOptions::default())
            .expect_err("checksum mismatch");

        assert!(matches!(
            error,
            PackageError::ArtifactChecksumMismatch { artifact, .. } if artifact == "fixture.onnx"
        ));
        assert!(!installed.model_manifest_path("piper-lessac").exists());
        assert!(!installed.model_record_path("piper-lessac").exists());
        assert!(!installed.is_model_installed("piper-lessac"));
    }

    #[test]
    fn successful_local_artifact_install_writes_downloaded_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("fixture.onnx");
        std::fs::write(&source, b"hello").expect("fixture");
        let manifest = artifact_test_manifest(&source, HELLO_SHA256);
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        installed
            .install_model_with_options(&manifest, InstallModelOptions::default())
            .expect("install model");
        assert!(installed.model_manifest_path("piper-lessac").exists());
        assert!(installed.model_record_path("piper-lessac").exists());
        assert!(installed.is_model_installed("piper-lessac"));
        let record = installed
            .installed_model_record("piper-lessac")
            .expect("installed record");

        assert_eq!(record.status, InstalledPackageStatus::Ready);
        assert_eq!(record.artifacts.len(), 1);
        assert_eq!(record.artifacts[0].role, ArtifactRole::Model);
        assert!(record.artifacts[0].downloaded);
        let local_path = record.artifacts[0].local_path.as_ref().expect("local path");
        assert!(local_path.ends_with(Path::new("blobs").join("sha256").join(HELLO_SHA256)));
        assert_eq!(std::fs::read(local_path).expect("blob"), b"hello");
    }

    #[test]
    fn second_local_pull_reuses_verified_artifact_without_reading_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("fixture.onnx");
        std::fs::write(&source, b"hello").expect("fixture");
        let manifest = artifact_test_manifest(&source, HELLO_SHA256);
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        installed.install_model(&manifest).expect("first install");
        std::fs::remove_file(&source).expect("make fixture unavailable");
        let report = installed.install_model(&manifest).expect("verified reuse");

        assert!(report.installed);
        assert_eq!(
            installed
                .installed_model_record("piper-lessac")
                .expect("record")
                .status,
            InstalledPackageStatus::Ready
        );
    }

    #[test]
    fn corrupt_artifact_repairs_only_corrupt_entry() {
        let temp = tempfile::tempdir().expect("tempdir");
        let model_source = temp.path().join("fixture.onnx");
        let config_source = temp.path().join("fixture.onnx.json");
        std::fs::write(&model_source, b"hello").expect("model fixture");
        std::fs::write(&config_source, br#"{"audio":{"sample_rate":22050}}"#)
            .expect("config fixture");
        let manifest = multi_artifact_test_manifest(
            &model_source,
            &sha256_file(&model_source).expect("model sha"),
            &config_source,
            &sha256_file(&config_source).expect("config sha"),
        );
        let installed = InstalledRegistry::new(temp.path().join("manifests"));
        installed.install_model(&manifest).expect("first install");
        let record = installed
            .installed_model_record("piper-lessac")
            .expect("record");
        let corrupt = record
            .artifacts
            .iter()
            .find(|item| item.name == "fixture.onnx.json")
            .unwrap()
            .local_path
            .clone()
            .unwrap();
        std::fs::write(&corrupt, b"corrupt").expect("corrupt blob");
        std::fs::remove_file(&model_source).expect("valid source must not be read");

        installed
            .install_model(&manifest)
            .expect("repair corrupt only");
        assert_eq!(
            sha256_file(&corrupt).expect("repaired checksum"),
            manifest.artifacts.configs[0].sha256
        );
    }

    #[test]
    fn successful_local_artifact_install_writes_model_and_config_records() {
        let temp = tempfile::tempdir().expect("tempdir");
        let model_source = temp.path().join("fixture.onnx");
        let config_source = temp.path().join("fixture.onnx.json");
        std::fs::write(&model_source, b"hello").expect("model fixture");
        std::fs::write(&config_source, br#"{"audio":{"sample_rate":22050}}"#)
            .expect("config fixture");
        let model_sha = sha256_file(&model_source).expect("model sha");
        let config_sha = sha256_file(&config_source).expect("config sha");
        let manifest =
            multi_artifact_test_manifest(&model_source, &model_sha, &config_source, &config_sha);
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        installed
            .install_model_with_options(&manifest, InstallModelOptions::default())
            .expect("install model");
        let record = installed
            .installed_model_record("piper-lessac")
            .expect("installed record");

        assert_eq!(record.status, InstalledPackageStatus::Ready);
        assert_eq!(record.artifacts.len(), 2);
        assert!(record.artifacts.iter().all(|artifact| artifact.downloaded));
        assert!(record
            .artifacts
            .iter()
            .any(|artifact| artifact.name == "fixture.onnx"
                && artifact.role == ArtifactRole::Model
                && artifact.local_path.as_ref().is_some_and(
                    |path| path.ends_with(Path::new("blobs").join("sha256").join(&model_sha))
                )));
        assert!(record
            .artifacts
            .iter()
            .any(|artifact| artifact.name == "fixture.onnx.json"
                && artifact.role == ArtifactRole::Config
                && artifact.local_path.as_ref().is_some_and(
                    |path| path.ends_with(Path::new("blobs").join("sha256").join(&config_sha))
                )));
    }

    #[test]
    fn bundled_piper_lessac_manifest_has_verified_artifact_fields() {
        let manifest = PackageRegistry::bundled()
            .model("piper-lessac")
            .expect("piper manifest");

        assert!(!manifest.artifacts.metadata_only);
        assert_eq!(manifest.artifacts.weights.len(), 1);
        assert_eq!(manifest.artifacts.configs.len(), 1);
        assert_eq!(
            manifest.artifacts.weights[0].url.as_deref(),
            Some("https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx")
        );
        assert_eq!(manifest.artifacts.weights[0].bytes, Some(63_201_294));
        assert_eq!(
            manifest.artifacts.weights[0].sha256,
            "5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f"
        );
        assert_eq!(
            manifest.artifacts.configs[0].url.as_deref(),
            Some("https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json")
        );
        assert_eq!(manifest.artifacts.configs[0].bytes, Some(4_885));
        assert_eq!(
            manifest.artifacts.configs[0].sha256,
            "efe19c417bed055f2d69908248c6ba650fa135bc868b0e6abb3da181dab690a0"
        );
    }

    #[test]
    fn bundled_metadata_only_models_install_without_artifact_downloads() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = PackageRegistry::bundled();
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        for model_id in ["chatterbox", "gpt-sovits"] {
            let manifest = registry.model(model_id).expect("model manifest");
            installed.install_model(&manifest).expect("install model");
            let record = installed
                .installed_model_record(model_id)
                .expect("installed record");

            assert_eq!(record.status, InstalledPackageStatus::MetadataOnly);
            assert!(record.artifacts.iter().all(|artifact| !artifact.downloaded));
        }
    }

    #[test]
    fn bundled_whisper_base_manifest_has_verified_artifact_metadata() {
        let registry = PackageRegistry::bundled();
        let manifest = registry.model("whisper-base").expect("whisper manifest");

        assert!(!manifest.artifacts.metadata_only);
        assert_eq!(manifest.family, "whisper");
        assert_eq!(manifest.artifacts.weights.len(), 1);
        assert_eq!(manifest.artifacts.weights[0].name, "ggml-base.bin");
        assert_eq!(manifest.artifacts.weights[0].bytes, Some(147_951_465));
        assert_eq!(
            manifest.artifacts.weights[0].sha256,
            "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe"
        );
    }

    #[test]
    fn bundled_qwen_manifest_pins_complete_local_runtime_artifacts() {
        let registry = PackageRegistry::bundled();
        let manifest = registry.model("qwen3-tts").expect("qwen manifest");

        assert!(!manifest.artifacts.metadata_only);
        assert_eq!(manifest.license, "apache-2.0");
        assert_eq!(manifest.artifacts.weights.len(), 2);
        assert!(manifest
            .artifacts
            .all()
            .any(|artifact| artifact.name == "speech_tokenizer/model.safetensors"));
        assert!(manifest
            .artifacts
            .all()
            .all(|artifact| !artifact.sha256.trim().is_empty() && artifact.bytes.is_some()));
    }

    #[test]
    fn metadata_only_model_install_still_works_with_artifact_placeholders() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("fixture.onnx");
        std::fs::write(&source, b"hello").expect("fixture");
        let manifest = artifact_test_manifest(&source, "");
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        installed
            .install_model_with_options(
                &manifest,
                InstallModelOptions {
                    metadata_only: true,
                },
            )
            .expect("metadata install");
        assert!(installed.model_manifest_path("piper-lessac").exists());
        assert!(installed.model_record_path("piper-lessac").exists());
        assert!(installed.is_model_installed("piper-lessac"));
        let record = installed
            .installed_model_record("piper-lessac")
            .expect("installed record");

        assert_eq!(record.status, InstalledPackageStatus::MetadataOnly);
        assert_eq!(record.artifacts.len(), 1);
        assert!(!record.artifacts[0].downloaded);
        assert!(record.artifacts[0].local_path.is_none());
    }

    #[test]
    fn install_runner_writes_installed_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));
        let manifest = registry.runner("takokit-onnx").expect("runner");

        let report = installed.install_runner(&manifest).expect("install runner");
        let record = installed
            .installed_runner_record("takokit-onnx")
            .expect("installed runner record");

        assert_eq!(report.id, "takokit-onnx");
        assert_eq!(record.id, "takokit-onnx");
        assert_eq!(record.version, "0.1.0");
        assert_eq!(record.kind, "onnx");
        assert_eq!(record.status, RunnerLifecycleState::ContractInstalled);
        assert!(record.manifest_path.ends_with("runners/takokit-onnx.toml"));
    }

    #[test]
    fn install_runner_runtime_updates_runner_record_state_and_note() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));
        let manifest = registry.runner("takokit-onnx").expect("runner");
        installed.install_runner(&manifest).expect("install runner");

        let report = installed
            .install_runner_runtime(
                &manifest,
                RunnerLifecycleState::RuntimeInstalled,
                "ONNX runtime dependency path initialized.",
            )
            .expect("install runner runtime");
        let record = installed
            .installed_runner_record("takokit-onnx")
            .expect("installed runner record");

        assert_eq!(report.id, "takokit-onnx");
        assert_eq!(record.status, RunnerLifecycleState::RuntimeInstalled);
        assert!(record.note.contains("ONNX runtime dependency path"));
    }

    #[test]
    fn pulling_runner_contract_does_not_downgrade_ready_runtime() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));
        let manifest = registry.runner("takokit-onnx").expect("runner");
        installed.install_runner(&manifest).expect("contract");
        installed
            .install_runner_runtime(&manifest, RunnerLifecycleState::Ready, "ready runtime")
            .expect("ready runtime");

        installed
            .install_runner(&manifest)
            .expect("refresh contract");

        let record = installed
            .installed_runner_record("takokit-onnx")
            .expect("runner record");
        assert_eq!(record.status, RunnerLifecycleState::Ready);
        assert_eq!(record.note, "ready runtime");
    }

    #[test]
    fn package_registry_exposes_registry_root_for_health_checks() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = PackageRegistry::new(temp.path());

        assert_eq!(registry.root(), temp.path());
    }

    #[test]
    fn installed_registry_lists_installed_model_and_runner_records() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));
        let model = registry.model("kokoro").expect("model");
        let runner = registry.runner("takokit-onnx").expect("runner");
        installed.install_model(&model).expect("install model");
        installed.install_runner(&runner).expect("install runner");

        let models = installed.installed_model_records().expect("model records");
        let runners = installed
            .installed_runner_records()
            .expect("runner records");

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "kokoro");
        assert_eq!(runners.len(), 1);
        assert_eq!(runners[0].id, "takokit-onnx");
    }

    #[test]
    fn exposes_current_platform_identifier() {
        let platform = current_platform_id();

        assert!(platform.contains('-'));
        assert!(!platform.starts_with('-'));
        assert!(!platform.ends_with('-'));
    }

    #[test]
    fn resolver_reports_model_not_installed_before_runner_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));

        let error = resolve_execution_plan(
            &registry,
            &installed,
            "kokoro",
            CapabilityKind::TextToSpeech,
        )
        .expect_err("missing model");

        assert!(matches!(error, PackageError::ModelNotInstalled(id) if id == "kokoro"));
    }

    #[test]
    fn resolver_reports_runner_missing_after_model_is_installed() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let registry = PackageRegistry::new(temp.path());
        let installed = InstalledRegistry::new(temp.path().join("installed"));
        let manifest = registry.model("kokoro").expect("model");
        installed.install_model(&manifest).expect("install model");

        let error = resolve_execution_plan(
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
    fn resolver_returns_execution_plan_after_model_and_runner_are_installed() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_test_registry(temp.path());
        let installed_root = temp.path().join("installed");
        let installed = InstalledRegistry::new(&installed_root);
        let registry = PackageRegistry::new(temp.path());
        let model = registry.model("kokoro").expect("model");
        let runner = registry.runner("takokit-onnx").expect("runner");
        installed.install_model(&model).expect("install model");
        installed.install_runner(&runner).expect("install runner");

        let plan = resolve_execution_plan(
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
        assert_eq!(plan.status, ExecutionStatus::Planned);
        assert_eq!(
            plan.installed_model
                .as_ref()
                .map(|record| record.id.as_str()),
            Some("kokoro")
        );
    }

    #[test]
    fn bundled_library_model_manifests_parse_with_allowed_enums() {
        let registry = PackageRegistry::bundled();
        let models = registry.library_models().expect("library models");

        assert!(models.iter().any(|model| model.id == "piper-lessac"));
        assert!(models.iter().any(|model| model.id == "whisper"));
        assert!(models.iter().any(|model| model.id == "qwen3-tts"));
        assert!(models.iter().any(|model| model.id == "voxtral"));
        assert!(models
            .iter()
            .all(|model| !model.tasks.is_empty() && !model.languages.is_empty()));
        assert!(models
            .iter()
            .filter(|model| model.runtime_status == LibraryRuntimeStatus::Supported)
            .all(|model| model.id == "piper-lessac"));
    }

    #[test]
    fn bundled_library_runner_manifests_parse_with_allowed_enums() {
        let registry = PackageRegistry::bundled();
        let runners = registry.library_runners().expect("library runners");

        assert!(runners.iter().any(|runner| runner.id == "takokit-onnx"));
        assert!(runners
            .iter()
            .any(|runner| runner.id == "takokit-transformers-audio"));
        assert!(runners
            .iter()
            .any(|runner| runner.id == "takokit-python-managed"));
        assert!(runners
            .iter()
            .all(|runner| !runner.notes.is_empty() && !runner.supported_platforms.is_empty()));
    }

    #[test]
    fn bundled_runtime_runner_manifests_cover_shared_runner_families() {
        let registry = PackageRegistry::bundled();
        let runners = registry.runners().expect("runtime runners");
        let ids: Vec<_> = runners.iter().map(|runner| runner.id.as_str()).collect();

        for required in [
            "takokit-onnx",
            "takokit-whispercpp",
            "takokit-python-managed",
            "takokit-transformers-audio",
            "takokit-nemo",
        ] {
            assert!(ids.contains(&required), "missing runtime runner {required}");
        }

        let python = registry
            .runner("takokit-python-managed")
            .expect("python-managed runner");
        assert!(python
            .supported_model_families
            .iter()
            .any(|family| family == "Qwen3-TTS"));
        assert!(python
            .supported_tasks
            .contains(&CapabilityKind::TextToSpeech));
        assert_eq!(
            python.dependency_strategy,
            RunnerDependencyStrategy::Managed
        );
        assert!(python.notes.contains("Python"));
    }

    #[test]
    fn bundled_runtime_model_manifests_cover_launch_families() {
        let registry = PackageRegistry::bundled();
        let models = registry.models().expect("runtime models");
        let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();

        for required in [
            "piper-lessac",
            "kokoro",
            "whisper-base",
            "qwen3-tts",
            "cosyvoice2",
            "f5-tts",
            "fish-speech",
            "dia",
            "chatterbox",
            "gpt-sovits",
            "openvoice",
            "rvc",
            "qwen3-omni",
            "qwen2-5-omni",
            "voxtral",
            "sensevoice",
            "parakeet",
            "canary",
        ] {
            assert!(ids.contains(&required), "missing runtime model {required}");
        }

        let runners: std::collections::HashSet<_> = registry
            .runners()
            .expect("runtime runners")
            .into_iter()
            .map(|runner| runner.id)
            .collect();
        for model in models {
            assert!(
                runners.contains(&model.runner),
                "{} references unknown runner {}",
                model.id,
                model.runner
            );
            assert!(
                !model.capabilities.to_model_capabilities().is_empty(),
                "{} has no capabilities",
                model.id
            );
        }
    }

    #[test]
    fn lifecycle_enum_values_parse_from_manifest_strings() {
        assert_eq!(
            toml::from_str::<ModelLifecycleFixture>(r#"state = "metadata-only""#)
                .expect("metadata-only")
                .state,
            ModelLifecycleState::MetadataOnly
        );
        assert_eq!(
            toml::from_str::<RunnerLifecycleFixture>(r#"state = "contract-installed""#)
                .expect("contract-installed")
                .state,
            RunnerLifecycleState::ContractInstalled
        );
    }

    #[test]
    fn model_plan_is_honest_for_piper_whisper_qwen_and_missing_model() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = PackageRegistry::bundled();
        let installed = InstalledRegistry::new(temp.path().join("manifests"));
        let piper = registry.model("piper-lessac").expect("piper manifest");
        let piper_model_path = temp.path().join("en_US-lessac-medium.onnx");
        let piper_config_path = temp.path().join("en_US-lessac-medium.onnx.json");
        std::fs::write(&piper_model_path, b"model").expect("piper model fixture");
        std::fs::write(&piper_config_path, b"config").expect("piper config fixture");
        std::fs::create_dir_all(temp.path().join("manifests").join("installed-models"))
            .expect("installed model records dir");
        let piper_record = InstalledModelRecord {
            id: piper.id.clone(),
            version: piper.version.clone(),
            source: "test".to_string(),
            manifest_path: PathBuf::from("piper-lessac.toml"),
            runner: piper.runner.clone(),
            installed_at: "0".to_string(),
            artifacts: vec![
                InstalledArtifactRecord {
                    name: "en_US-lessac-medium.onnx".to_string(),
                    sha256: "test".to_string(),
                    bytes: None,
                    url: None,
                    role: ArtifactRole::Model,
                    local_path: Some(piper_model_path),
                    downloaded: true,
                },
                InstalledArtifactRecord {
                    name: "en_US-lessac-medium.onnx.json".to_string(),
                    sha256: "test".to_string(),
                    bytes: None,
                    url: None,
                    role: ArtifactRole::Config,
                    local_path: Some(piper_config_path),
                    downloaded: true,
                },
            ],
            status: InstalledPackageStatus::Ready,
            note: "test".to_string(),
        };
        std::fs::write(
            temp.path()
                .join("manifests")
                .join("installed-models")
                .join("piper-lessac.toml"),
            toml::to_string_pretty(&piper_record).expect("record toml"),
        )
        .expect("write piper record");
        installed
            .install_runner(&registry.runner("takokit-onnx").expect("onnx runner"))
            .expect("install onnx runner contract");

        let piper_plan = plan_model(&registry, &installed, "piper-lessac").expect("piper plan");
        assert_eq!(piper_plan.model_id, "piper-lessac");
        assert_eq!(piper_plan.family, "piper");
        assert_eq!(
            piper_plan.artifact_state,
            ModelLifecycleState::ArtifactsReady
        );
        assert_eq!(
            piper_plan.runner_contract_state,
            RunnerLifecycleState::ContractInstalled
        );
        assert_eq!(
            piper_plan.runner_runtime_state,
            RunnerLifecycleState::ContractInstalled
        );
        assert!(!piper_plan.executable);
        assert!(piper_plan
            .missing
            .contains(&"Piper text frontend (phonemizer/token preparation)".to_string()));

        let whisper_plan = plan_model(&registry, &installed, "whisper-base").expect("whisper plan");
        assert_eq!(whisper_plan.required_runner, "takokit-whispercpp");
        assert_eq!(whisper_plan.family, "whisper");
        assert_eq!(
            whisper_plan.artifact_state,
            ModelLifecycleState::MetadataOnly
        );
        assert_eq!(
            whisper_plan.runner_runtime_state,
            RunnerLifecycleState::RuntimeMissing
        );
        assert!(!whisper_plan.executable);

        let qwen_plan = plan_model(&registry, &installed, "qwen3-tts").expect("qwen plan");
        assert_eq!(qwen_plan.required_runner, "takokit-python-managed");
        assert_eq!(qwen_plan.family, "qwen");
        assert_eq!(qwen_plan.artifact_state, ModelLifecycleState::MetadataOnly);
        assert!(qwen_plan
            .missing
            .iter()
            .any(|item| item.contains("qwen3_tts managed adapter")));

        let missing = plan_model(&registry, &installed, "does-not-exist")
            .expect_err("missing model should not plan");
        assert!(matches!(missing, PackageError::ModelNotFound(id) if id == "does-not-exist"));
    }

    #[test]
    fn model_info_is_derived_from_canonical_lifecycle_plan() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = PackageRegistry::bundled();
        let installed = InstalledRegistry::new(temp.path().join("manifests"));
        let whisper = registry.model("whisper-base").expect("whisper manifest");
        let runner = registry
            .runner("takokit-whispercpp")
            .expect("whisper runner");

        installed
            .install_model_with_options(
                &whisper,
                InstallModelOptions {
                    metadata_only: true,
                },
            )
            .expect("metadata-only whisper");
        installed
            .install_runner_runtime(&runner, RunnerLifecycleState::Ready, "test runner ready")
            .expect("runner ready");

        let plan = plan_model(&registry, &installed, "whisper-base").expect("plan");
        let info = registry
            .model("whisper-base")
            .expect("manifest")
            .to_model_info_from_plan(&plan, true, true);

        assert_eq!(info.family, "whisper");
        assert_eq!(
            info.lifecycle_state,
            ModelLifecycleState::MetadataOnly.to_string()
        );
        assert_eq!(
            info.runner_runtime_state,
            RunnerLifecycleState::Ready.to_string()
        );
        assert!(!info.executable);
        assert!(info.execution_status.contains("metadata-only"));
        assert_eq!(
            info.next_command,
            "takokit runner doctor takokit-whispercpp"
        );
        assert!(info.missing.iter().any(|item| item == "verified artifacts"));
    }

    #[test]
    fn python_managed_runner_layout_resolves_under_takokit_root() {
        let root = PathBuf::from("/tmp/takokit-test-root");
        let layout = python_managed_runner_layout(&root);

        assert_eq!(layout.root, root.join("runners").join("python-managed"));
        assert_eq!(layout.runtime, layout.root.join("runtime"));
        assert_eq!(layout.env, layout.root.join("env"));
        assert_eq!(layout.packages, layout.root.join("packages"));
        assert_eq!(layout.wheels, layout.root.join("wheels"));
        assert_eq!(layout.logs, layout.root.join("logs"));
        assert_eq!(layout.manifests, layout.root.join("manifests"));
        assert_eq!(layout.cache, layout.root.join("cache"));
        assert_eq!(layout.adapters, layout.root.join("adapters"));
    }

    #[test]
    fn finds_managed_uv_before_path_lookup() {
        let temp = tempfile::tempdir().expect("tempdir");
        let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
        let uv = temp.path().join("tools").join("uv").join(executable);
        std::fs::create_dir_all(uv.parent().expect("parent")).expect("tools dir");
        std::fs::write(&uv, b"fixture").expect("uv fixture");

        assert_eq!(find_uv(temp.path()), Some(uv));
    }

    #[test]
    fn initializing_python_managed_runner_writes_adapter_slots() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = PackageRegistry::bundled();
        let manifest = registry
            .runner("takokit-python-managed")
            .expect("python runner");
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        initialize_runner_runtime(temp.path(), &installed, &manifest).expect("runtime init");

        let adapters = temp
            .path()
            .join("runners")
            .join("python-managed")
            .join("adapters");
        for adapter in ["qwen3_tts", "chatterbox", "f5_tts", "rvc"] {
            assert!(
                adapters.join(adapter).join("adapter.toml").is_file(),
                "missing {adapter} adapter manifest"
            );
        }
    }

    #[test]
    fn bundled_runtime_models_are_not_marked_executable_without_ready_runners() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = PackageRegistry::bundled();
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        for model in registry.models().expect("runtime models") {
            let plan = plan_model(&registry, &installed, &model.id).expect("model plan");

            assert!(!plan.executable, "{} should not be executable", model.id);
            assert_ne!(
                plan.artifact_state,
                ModelLifecycleState::Executable,
                "{} should not claim executable artifact state",
                model.id
            );
        }
    }

    #[derive(Debug, Deserialize)]
    struct ModelLifecycleFixture {
        state: ModelLifecycleState,
    }

    #[derive(Debug, Deserialize)]
    struct RunnerLifecycleFixture {
        state: RunnerLifecycleState,
    }

    fn write_test_registry(root: &Path) {
        let models = root.join("models");
        let runners = root.join("runners");
        std::fs::create_dir_all(&models).expect("models dir");
        std::fs::create_dir_all(&runners).expect("runners dir");
        std::fs::write(models.join("kokoro.toml"), MODEL_TOML).expect("model toml");
        std::fs::write(runners.join("takokit-onnx.toml"), RUNNER_TOML).expect("runner toml");
    }

    fn artifact_test_manifest(source: &Path, sha256: &str) -> ModelManifest {
        let source = source.to_string_lossy().replace('\\', "/");
        let toml = format!(
            r#"
id = "piper-lessac"
name = "Piper Lessac"
family = "piper"
version = "0.1.0"
kind = "tts"
backend = "onnx"
runner = "takokit-onnx"
license = "mit"
description = "Piper Lessac test manifest."

[capabilities]
tts = true
stt = false
voice_cloning = false
live_transcription = false
live_audio = true

[hardware]
cpu = true
gpu = false
min_ram = "2gb"

[artifacts]

[[artifacts.weights]]
name = "fixture.onnx"
url = "{source}"
sha256 = "{sha256}"
bytes = 5
role = "model"
"#
        );

        toml::from_str(&toml).expect("artifact manifest")
    }

    fn multi_artifact_test_manifest(
        model_source: &Path,
        model_sha256: &str,
        config_source: &Path,
        config_sha256: &str,
    ) -> ModelManifest {
        let model_source = model_source.to_string_lossy().replace('\\', "/");
        let config_source = config_source.to_string_lossy().replace('\\', "/");
        let toml = format!(
            r#"
id = "piper-lessac"
name = "Piper Lessac"
family = "piper"
version = "0.1.0"
kind = "tts"
backend = "onnx"
runner = "takokit-onnx"
license = "mit"
description = "Piper Lessac multi-artifact test manifest."

[capabilities]
tts = true
stt = false
voice_cloning = false
live_transcription = false
live_audio = true

[hardware]
cpu = true
gpu = false
min_ram = "2gb"

[artifacts]

[[artifacts.weights]]
name = "fixture.onnx"
url = "{model_source}"
sha256 = "{model_sha256}"
bytes = 5
role = "model"

[[artifacts.configs]]
name = "fixture.onnx.json"
url = "{config_source}"
sha256 = "{config_sha256}"
bytes = 31
role = "config"
"#
        );

        toml::from_str(&toml).expect("multi artifact manifest")
    }
}
