use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModelLifecycleState {
    MetadataOnly,
    ArtifactsReady,
    RunnerReady,
    Executable,
    Failed,
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
        let artifact_set = self.install_artifacts(manifest, options)?;
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
        let record = installed_runner_record(manifest, path.clone());
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

    fn storage_root(&self) -> PathBuf {
        self.root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.root.clone())
    }

    fn install_artifacts(
        &self,
        manifest: &ModelManifest,
        options: InstallModelOptions,
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
            let local_path = install_artifact(manifest, artifact, &downloads_dir, &blob_dir)?;
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
            artifact_count: self.artifacts.all().count(),
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
            supported_model_families: self.supported_model_families.clone(),
            supported_tasks: self.supported_tasks.clone(),
            dependency_strategy: self.dependency_strategy,
            install_state: if installed {
                RunnerLifecycleState::ContractInstalled
            } else {
                self.install_state
            },
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
    let executable = artifact_state == ModelLifecycleState::Executable
        && runner_runtime_state == RunnerLifecycleState::Ready;
    let mut missing = Vec::new();

    if matches!(artifact_state, ModelLifecycleState::MetadataOnly) {
        missing.push("verified artifacts".to_string());
    }
    if runner_contract_state == RunnerLifecycleState::RuntimeMissing {
        missing.push(format!("runner contract: {}", runner.id));
    }
    if runner_runtime_state != RunnerLifecycleState::Ready {
        missing.push(runner_missing_component(&runner));
    }
    if executable {
        missing.clear();
    }

    Ok(ModelPlan {
        model_id: model.id.clone(),
        model_name: model.name.clone(),
        family: model.name.clone(),
        task: model_task_label(&model),
        required_runner: runner.id.clone(),
        artifact_state,
        runner_contract_state,
        runner_runtime_state,
        executable,
        missing,
        next_command: next_plan_command(
            &model,
            installed_model.is_some(),
            installed_runner.is_some(),
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
        root,
    }
}

fn model_artifact_state(
    model: &ModelManifest,
    installed_model: Option<&InstalledModelRecord>,
) -> ModelLifecycleState {
    let Some(record) = installed_model else {
        return ModelLifecycleState::MetadataOnly;
    };

    if record.status == InstalledPackageStatus::Ready
        && !model.artifacts.metadata_only
        && model.artifacts.all().next().is_some()
        && record.artifacts.iter().all(|artifact| artifact.downloaded)
    {
        ModelLifecycleState::ArtifactsReady
    } else {
        ModelLifecycleState::MetadataOnly
    }
}

fn model_task_label(model: &ModelManifest) -> String {
    capability_labels(&model.capabilities.to_model_capabilities())
}

fn runner_missing_component(runner: &RunnerManifest) -> String {
    match runner.kind {
        RunnerKind::Onnx => "ONNX inference implementation".to_string(),
        RunnerKind::Whispercpp => "whisper.cpp transcription implementation".to_string(),
        RunnerKind::PythonManaged => "verified artifacts and managed runtime adapter".to_string(),
        RunnerKind::TransformersAudio => "Transformers audio runtime adapter".to_string(),
        RunnerKind::Nemo => "NeMo runtime adapter".to_string(),
        RunnerKind::External => "external runner adapter".to_string(),
        RunnerKind::Native => "native runner implementation".to_string(),
    }
}

fn next_plan_command(
    model: &ModelManifest,
    model_installed: bool,
    runner_installed: bool,
) -> String {
    if !model_installed {
        format!("takokit pull {}", model.id)
    } else if !runner_installed {
        format!("takokit runner pull {}", model.runner)
    } else {
        "wait for runner execution implementation".to_string()
    }
}

fn capability_labels(capabilities: &[CapabilityKind]) -> String {
    if capabilities.is_empty() {
        return "none".to_string();
    }

    capabilities
        .iter()
        .map(|capability| capability.label())
        .collect::<Vec<_>>()
        .join(" / ")
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
        note: "Installed runner contract from local mock registry. Execution binary is not implemented."
            .to_string(),
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

fn install_artifact(
    manifest: &ModelManifest,
    artifact: &ArtifactEntry,
    downloads_dir: &Path,
    blob_dir: &Path,
) -> PackageResult<PathBuf> {
    let url = artifact
        .url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| PackageError::ArtifactUrlMissing {
            model: manifest.id.clone(),
            artifact: artifact.name.clone(),
        })?;
    let expected = artifact.sha256.trim().to_ascii_lowercase();
    if expected.is_empty() || expected == "todo" {
        return Err(PackageError::ArtifactChecksumMissing {
            model: manifest.id.clone(),
            artifact: artifact.name.clone(),
        });
    }

    let temp_path = downloads_dir.join(format!(
        "{}.{}.part",
        sanitize_file_name(&artifact.name),
        timestamp_now()
    ));
    download_to_temp(url, &artifact.name, &temp_path)?;

    if let Some(expected_bytes) = artifact.bytes {
        let actual_bytes = std::fs::metadata(&temp_path)
            .map(|metadata| metadata.len())
            .map_err(|error| {
                let _ = std::fs::remove_file(&temp_path);
                PackageError::ArtifactInstallFailed {
                    artifact: artifact.name.clone(),
                    reason: error.to_string(),
                }
            })?;
        if actual_bytes != expected_bytes {
            let _ = std::fs::remove_file(&temp_path);
            return Err(PackageError::ArtifactInstallFailed {
                artifact: artifact.name.clone(),
                reason: format!("expected {expected_bytes} bytes, got {actual_bytes}"),
            });
        }
    }

    let actual = sha256_file(&temp_path).map_err(|error| PackageError::ArtifactInstallFailed {
        artifact: artifact.name.clone(),
        reason: error.to_string(),
    })?;
    if actual != expected {
        let _ = std::fs::remove_file(&temp_path);
        return Err(PackageError::ArtifactChecksumMismatch {
            artifact: artifact.name.clone(),
            expected,
            actual,
        });
    }

    let final_path = blob_dir.join(&expected);
    if final_path.exists() {
        let _ = std::fs::remove_file(&temp_path);
    } else {
        std::fs::rename(&temp_path, &final_path).map_err(|error| {
            let _ = std::fs::remove_file(&temp_path);
            PackageError::ArtifactInstallFailed {
                artifact: artifact.name.clone(),
                reason: error.to_string(),
            }
        })?;
    }

    Ok(final_path)
}

fn download_to_temp(url: &str, artifact: &str, temp_path: &Path) -> PackageResult<()> {
    if url.starts_with("http://") || url.starts_with("https://") {
        let response =
            ureq::get(url)
                .call()
                .map_err(|error| PackageError::ArtifactDownloadFailed {
                    artifact: artifact.to_string(),
                    reason: error.to_string(),
                })?;
        let mut reader = response.into_reader();
        let mut file = File::create(temp_path)?;
        std::io::copy(&mut reader, &mut file).map_err(|error| {
            let _ = std::fs::remove_file(temp_path);
            PackageError::ArtifactDownloadFailed {
                artifact: artifact.to_string(),
                reason: error.to_string(),
            }
        })?;
        return Ok(());
    }

    let local_path = if let Some(path) = url.strip_prefix("file://") {
        PathBuf::from(path)
    } else {
        PathBuf::from(url)
    };

    let mut input =
        File::open(&local_path).map_err(|error| PackageError::ArtifactDownloadFailed {
            artifact: artifact.to_string(),
            reason: error.to_string(),
        })?;
    let mut output = File::create(temp_path)?;
    std::io::copy(&mut input, &mut output).map_err(|error| {
        let _ = std::fs::remove_file(temp_path);
        PackageError::ArtifactDownloadFailed {
            artifact: artifact.to_string(),
            reason: error.to_string(),
        }
    })?;
    Ok(())
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sanitize_file_name(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
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
    fn bundled_placeholder_models_still_install_metadata_only() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = PackageRegistry::bundled();
        let installed = InstalledRegistry::new(temp.path().join("manifests"));

        for model_id in ["kokoro", "whisper-base", "chatterbox", "gpt-sovits"] {
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
            .contains(&"ONNX inference implementation".to_string()));

        let whisper_plan = plan_model(&registry, &installed, "whisper-base").expect("whisper plan");
        assert_eq!(whisper_plan.required_runner, "takokit-whispercpp");
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
        assert_eq!(qwen_plan.artifact_state, ModelLifecycleState::MetadataOnly);
        assert!(qwen_plan
            .missing
            .iter()
            .any(|item| item.contains("managed runtime adapter")));

        let missing = plan_model(&registry, &installed, "does-not-exist")
            .expect_err("missing model should not plan");
        assert!(matches!(missing, PackageError::ModelNotFound(id) if id == "does-not-exist"));
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
