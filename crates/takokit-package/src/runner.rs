//! Runner contracts and lifecycle state.

use crate::*;
use serde::{Deserialize, Serialize};
use takokit_core::CapabilityKind;

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
    pub(crate) fn as_str(&self) -> &'static str {
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
