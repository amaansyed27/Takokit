//! Read-only model and runner discovery catalog types.

use crate::RunnerKind;
use serde::{Deserialize, Serialize};

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
