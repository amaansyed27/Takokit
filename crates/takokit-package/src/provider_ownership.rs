//! Durable ownership for provider-managed checkpoint caches.
//!
//! Provider caches remain acceleration data. Any cache byte required by an installed
//! managed-Python model is mirrored into a Takokit-owned content-addressed blob and
//! recorded in a per-model ownership ledger. The cache can therefore be reconstructed
//! without redownloading the model.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

mod operations;
mod storage;
mod support;
#[cfg(test)]
mod tests;

pub use operations::*;

pub const PROVIDER_OWNERSHIP_SCHEMA: u32 = 1;
const PROVIDERS: &[&str] = &["huggingface", "torch", "coqui", "modelscope", "openvoice"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderOwnedArtifact {
    pub provider: String,
    pub relative_cache_path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
    pub blob_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelProviderOwnership {
    pub schema_version: u32,
    pub model_id: String,
    pub legacy_shared: bool,
    pub artifacts: Vec<ProviderOwnedArtifact>,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderCacheSnapshot {
    files: BTreeMap<PathBuf, FileSignature>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileSignature {
    bytes: u64,
    modified_nanos: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderOwnershipStatus {
    pub schema_version: u32,
    pub provider_cache_files: u64,
    pub provider_cache_bytes: u64,
    pub durable_blob_files: u64,
    pub durable_blob_bytes: u64,
    pub model_ledgers: u64,
    pub legacy_models_pending_migration: Vec<String>,
    pub provider_cache_fully_owned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMigrationReport {
    pub journal: PathBuf,
    pub discovered_models: Vec<String>,
    pub migrated_models: Vec<String>,
    pub already_owned_models: Vec<String>,
    pub provider_files: u64,
    pub provider_bytes: u64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCleanupItem {
    pub category: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCleanupReport {
    pub scope: String,
    pub dry_run: bool,
    pub removed: Vec<ProviderCleanupItem>,
    pub retained: Vec<ProviderCleanupItem>,
    pub reclaimed_bytes: u64,
}
