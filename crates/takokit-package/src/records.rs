//! Installed package records, execution plans, and runtime layout value types.

use crate::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use takokit_core::CapabilityKind;

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
    #[serde(default)]
    pub snapshot: Option<InstalledSnapshotRecord>,
    pub status: InstalledPackageStatus,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledModelsResponse {
    pub kind: String,
    pub data: Vec<InstalledModelSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledModelSummary {
    pub name: String,
    pub id: String,
    pub size_bytes: u64,
    pub modified_at: u64,
    pub version: String,
    pub runner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledSnapshotRecord {
    pub provider: ModelSourceProvider,
    pub repository: String,
    pub revision: String,
    pub local_path: PathBuf,
    pub ready_marker: PathBuf,
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
    pub storage_root: PathBuf,
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
