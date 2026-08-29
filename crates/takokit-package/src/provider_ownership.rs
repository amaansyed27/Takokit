//! Durable ownership for provider-managed checkpoint caches.
//!
//! Provider caches remain acceleration data. Any cache byte required by an installed
//! managed-Python model is mirrored into a Takokit-owned content-addressed blob and
//! recorded in a per-model ownership ledger. The cache can therefore be reconstructed
//! without redownloading the model.

use crate::{artifact_io::sha256_file, PackageError, PackageResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

mod support;
use support::*;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationJournal {
    schema_version: u32,
    state: String,
    discovered_models: Vec<String>,
    completed_models: Vec<String>,
    provider_files: u64,
    provider_bytes: u64,
    last_model: Option<String>,
    updated_at_unix: u64,
}

pub fn snapshot_provider_cache(root: &Path) -> PackageResult<ProviderCacheSnapshot> {
    let mut files = BTreeMap::new();
    for provider in PROVIDERS {
        let base = root.join("cache").join(provider);
        scan_cache_files(&base, &base, provider, &mut files)?;
    }
    Ok(ProviderCacheSnapshot { files })
}

pub fn capture_provider_ownership(
    root: &Path,
    model_id: &str,
    before: &ProviderCacheSnapshot,
) -> PackageResult<ModelProviderOwnership> {
    let after = snapshot_provider_cache(root)?;
    let existing = read_model_provider_ownership(root, model_id)?;
    let mut selected = BTreeSet::new();
    for (path, signature) in &after.files {
        if before.files.get(path) != Some(signature) {
            selected.insert(path.clone());
        }
    }

    let legacy_shared = selected.is_empty() && existing.is_none() && !after.files.is_empty();
    if legacy_shared {
        selected.extend(after.files.keys().cloned());
    }

    let mut by_path = BTreeMap::<PathBuf, ProviderOwnedArtifact>::new();
    if let Some(existing) = existing {
        for artifact in existing.artifacts {
            by_path.insert(artifact.relative_cache_path.clone(), artifact);
        }
    }
    for relative in selected {
        let artifact = materialize_owned_artifact(root, &relative)?;
        by_path.insert(relative, artifact);
    }

    let ownership = ModelProviderOwnership {
        schema_version: PROVIDER_OWNERSHIP_SCHEMA,
        model_id: model_id.to_string(),
        legacy_shared,
        artifacts: by_path.into_values().collect(),
        updated_at_unix: now(),
    };
    write_model_provider_ownership(root, &ownership)?;
    Ok(ownership)
}

pub fn ensure_provider_cache_from_ownership(root: &Path, model_id: &str) -> PackageResult<u64> {
    let Some(ownership) = read_model_provider_ownership(root, model_id)? else {
        return Ok(0);
    };
    let cache_root = root.join("cache");
    let mut restored = 0_u64;
    for artifact in ownership.artifacts {
        validate_relative_cache_path(&artifact.relative_cache_path)?;
        let destination = cache_root.join(&artifact.relative_cache_path);
        let valid = fs::metadata(&destination)
            .ok()
            .is_some_and(|metadata| metadata.is_file() && metadata.len() == artifact.bytes);
        if valid {
            continue;
        }
        if !artifact.blob_path.is_file() {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: model_id.to_string(),
                reason: format!(
                    "durable provider blob is missing: {}",
                    artifact.blob_path.display()
                ),
            });
        }
        if sha256_file(&artifact.blob_path)? != artifact.sha256 {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: model_id.to_string(),
                reason: format!(
                    "durable provider blob failed SHA-256 verification: {}",
                    artifact.blob_path.display()
                ),
            });
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        remove_path_if_present(&destination)?;
        link_or_copy(&artifact.blob_path, &destination)?;
        restored = restored.saturating_add(artifact.bytes);
    }
    Ok(restored)
}

pub fn read_model_provider_ownership(
    root: &Path,
    model_id: &str,
) -> PackageResult<Option<ModelProviderOwnership>> {
    let path = ownership_path(root, model_id);
    if !path.is_file() {
        return Ok(None);
    }
    let ownership: ModelProviderOwnership = serde_json::from_slice(&fs::read(path)?)?;
    if ownership.schema_version != PROVIDER_OWNERSHIP_SCHEMA || ownership.model_id != model_id {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: model_id.to_string(),
            reason: "provider ownership ledger schema/model identity mismatch".to_string(),
        });
    }
    Ok(Some(ownership))
}

pub fn migrate_legacy_provider_cache(root: &Path) -> PackageResult<ProviderMigrationReport> {
    let models = discover_prefetched_models(root)?;
    let snapshot = snapshot_provider_cache(root)?;
    let (provider_files, provider_bytes) = snapshot_totals(&snapshot);
    let journal_path = migration_journal_path(root);
    let mut journal = MigrationJournal {
        schema_version: PROVIDER_OWNERSHIP_SCHEMA,
        state: "running".to_string(),
        discovered_models: models.clone(),
        completed_models: Vec::new(),
        provider_files,
        provider_bytes,
        last_model: None,
        updated_at_unix: now(),
    };
    write_json_atomic(&journal_path, &journal)?;

    let empty = ProviderCacheSnapshot::default();
    let mut migrated = Vec::new();
    let mut already = Vec::new();
    for model in &models {
        if read_model_provider_ownership(root, model)?.is_some() {
            already.push(model.clone());
        } else {
            capture_provider_ownership(root, model, &empty)?;
            migrated.push(model.clone());
        }
        journal.completed_models.push(model.clone());
        journal.last_model = Some(model.clone());
        journal.updated_at_unix = now();
        write_json_atomic(&journal_path, &journal)?;
    }
    journal.state = "completed".to_string();
    journal.updated_at_unix = now();
    write_json_atomic(&journal_path, &journal)?;

    Ok(ProviderMigrationReport {
        journal: journal_path,
        discovered_models: models,
        migrated_models: migrated,
        already_owned_models: already,
        provider_files,
        provider_bytes,
        completed: true,
    })
}

pub fn provider_ownership_status(root: &Path) -> PackageResult<ProviderOwnershipStatus> {
    let snapshot = snapshot_provider_cache(root)?;
    let (provider_cache_files, provider_cache_bytes) = snapshot_totals(&snapshot);
    let ledgers = read_all_ledgers(root)?;
    let legacy_models = discover_prefetched_models(root)?;
    let owned_models = ledgers
        .iter()
        .map(|ledger| ledger.model_id.as_str())
        .collect::<HashSet<_>>();
    let pending = legacy_models
        .into_iter()
        .filter(|model| !owned_models.contains(model.as_str()))
        .collect::<Vec<_>>();
    let (durable_blob_files, durable_blob_bytes) =
        scan_regular_file_totals(&provider_blob_root(root))?;
    let fully_owned = pending.is_empty()
        && ledgers
            .iter()
            .all(|ledger| verify_ledger_blobs(ledger).is_ok());
    Ok(ProviderOwnershipStatus {
        schema_version: PROVIDER_OWNERSHIP_SCHEMA,
        provider_cache_files,
        provider_cache_bytes,
        durable_blob_files,
        durable_blob_bytes,
        model_ledgers: ledgers.len() as u64,
        legacy_models_pending_migration: pending,
        provider_cache_fully_owned: fully_owned,
    })
}

pub fn clean_provider_storage(
    root: &Path,
    scope: &str,
    dry_run: bool,
) -> PackageResult<ProviderCleanupReport> {
    let mut removed = Vec::new();
    let mut retained = Vec::new();
    match scope {
        "downloads" => collect_tree(
            root.join("cache").join("downloads"),
            "downloads",
            "reconstructible download staging",
            &mut removed,
        )?,
        "unused" => collect_unused_blobs(root, &mut removed, &mut retained)?,
        "all-safe" => {
            collect_tree(
                root.join("cache").join("uv"),
                "uv",
                "reconstructible package cache",
                &mut removed,
            )?;
            collect_tree(
                root.join("cache").join("downloads"),
                "downloads",
                "reconstructible download staging",
                &mut removed,
            )?;
            collect_unused_blobs(root, &mut removed, &mut retained)?;
            let status = provider_ownership_status(root)?;
            if status.provider_cache_fully_owned {
                for provider in PROVIDERS {
                    collect_tree(
                        root.join("cache").join(provider),
                        provider,
                        "fully durable provider cache; reconstructible from Takokit-owned blobs",
                        &mut removed,
                    )?;
                }
            } else {
                for provider in PROVIDERS {
                    let path = root.join("cache").join(provider);
                    if path.exists() {
                        retained.push(ProviderCleanupItem {
                            category: (*provider).to_string(),
                            bytes: path_size(&path),
                            path,
                            reason: "protected because one or more installed managed models have not completed durable ownership migration".to_string(),
                        });
                    }
                }
            }
        }
        other => {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "storage-cleanup".to_string(),
                reason: format!("unsupported provider cleanup scope {other}"),
            })
        }
    }

    collapse_nested(&mut removed);
    let reclaimed_bytes = removed
        .iter()
        .map(|item| item.bytes)
        .fold(0_u64, u64::saturating_add);
    if !dry_run {
        for item in &removed {
            remove_path_if_present(&item.path)?;
            if matches!(item.category.as_str(), "uv" | "downloads")
                || PROVIDERS.contains(&item.category.as_str())
            {
                fs::create_dir_all(&item.path)?;
            }
        }
    }
    Ok(ProviderCleanupReport {
        scope: scope.to_string(),
        dry_run,
        removed,
        retained,
        reclaimed_bytes,
    })
}

pub fn remove_model_provider_ownership(
    root: &Path,
    model_id: &str,
    dry_run: bool,
) -> PackageResult<ProviderCleanupReport> {
    let ledger_path = ownership_path(root, model_id);
    let mut ignored = HashSet::new();
    ignored.insert(model_id.to_string());
    let mut removed = Vec::new();
    let mut retained = Vec::new();
    collect_unused_blobs_ignoring(root, &ignored, &mut removed, &mut retained)?;
    if ledger_path.is_file() {
        removed.push(ProviderCleanupItem {
            category: "ownership-ledger".to_string(),
            bytes: fs::metadata(&ledger_path)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
            path: ledger_path,
            reason: "selected model ownership ledger".to_string(),
        });
    }
    let reclaimed_bytes = removed
        .iter()
        .map(|item| item.bytes)
        .fold(0_u64, u64::saturating_add);
    if !dry_run {
        for item in &removed {
            remove_path_if_present(&item.path)?;
        }
    }
    Ok(ProviderCleanupReport {
        scope: format!("model:{model_id}"),
        dry_run,
        removed,
        retained,
        reclaimed_bytes,
    })
}

fn validate_relative_cache_path(path: &Path) -> PackageResult<()> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: path.display().to_string(),
            reason: "unsafe provider cache relative path".to_string(),
        });
    }
    let provider = path
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .unwrap_or_default();
    if !PROVIDERS.contains(&provider) {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: path.display().to_string(),
            reason: "provider cache path uses an unknown provider root".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_hardlinks_cache_bytes_into_durable_blob_and_rehydrates() {
        let root = tempfile::tempdir().unwrap();
        let provider = root.path().join("cache/huggingface/hub");
        fs::create_dir_all(&provider).unwrap();
        let before = snapshot_provider_cache(root.path()).unwrap();
        fs::write(provider.join("weights.bin"), vec![7_u8; 4096]).unwrap();
        let ownership = capture_provider_ownership(root.path(), "fixture-model", &before).unwrap();
        assert_eq!(ownership.artifacts.len(), 1);
        assert!(ownership.artifacts[0].blob_path.is_file());
        fs::remove_dir_all(root.path().join("cache/huggingface")).unwrap();
        let restored = ensure_provider_cache_from_ownership(root.path(), "fixture-model").unwrap();
        assert_eq!(restored, 4096);
        assert!(provider.join("weights.bin").is_file());
    }

    #[test]
    fn legacy_migration_is_idempotent_and_journaled() {
        let root = tempfile::tempdir().unwrap();
        let model = root.path().join("models/legacy");
        fs::create_dir_all(&model).unwrap();
        fs::write(model.join(".takokit-prefetch.json"), b"{}").unwrap();
        fs::create_dir_all(root.path().join("cache/torch")).unwrap();
        fs::write(
            root.path().join("cache/torch/checkpoint.bin"),
            vec![3_u8; 128],
        )
        .unwrap();
        let first = migrate_legacy_provider_cache(root.path()).unwrap();
        assert_eq!(first.migrated_models, vec!["legacy"]);
        let second = migrate_legacy_provider_cache(root.path()).unwrap();
        assert!(second.migrated_models.is_empty());
        assert_eq!(second.already_owned_models, vec!["legacy"]);
        assert!(second.journal.is_file());
    }

    #[test]
    fn all_safe_refuses_provider_cache_without_complete_ownership() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("models/legacy")).unwrap();
        fs::write(
            root.path().join("models/legacy/.takokit-prefetch.json"),
            b"{}",
        )
        .unwrap();
        fs::create_dir_all(root.path().join("cache/huggingface")).unwrap();
        fs::write(root.path().join("cache/huggingface/weights"), b"weights").unwrap();
        let report = clean_provider_storage(root.path(), "all-safe", true).unwrap();
        assert!(report
            .retained
            .iter()
            .any(|item| item.category == "huggingface"));
    }
}
