use super::{
    ModelProviderOwnership, ProviderCacheSnapshot, ProviderCleanupItem, ProviderCleanupReport,
    ProviderMigrationReport, ProviderOwnedArtifact, ProviderOwnershipStatus, PROVIDERS,
    PROVIDER_OWNERSHIP_SCHEMA,
};
use super::{storage::*, support::*};
use crate::{artifact_io::sha256_file, PackageError, PackageResult};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs::{self, File, OpenOptions},
    path::Path,
};

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

struct ProviderOwnershipGuard(File);

impl Drop for ProviderOwnershipGuard {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn acquire_provider_ownership_lock(root: &Path) -> PackageResult<ProviderOwnershipGuard> {
    let path = root.join("runtime").join("provider-ownership.lock");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;
    file.lock_exclusive()?;
    Ok(ProviderOwnershipGuard(file))
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
    // Normal model pulls already hold the global maintenance lock across the
    // before/after interval. This narrower lock additionally serializes the
    // ledger read-modify-write itself for direct callers and migrations.
    let _guard = acquire_provider_ownership_lock(root)?;
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

    let mut by_path = BTreeMap::<_, ProviderOwnedArtifact>::new();
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
    let _guard = acquire_provider_ownership_lock(root)?;
    let Some(ownership) = read_model_provider_ownership(root, model_id)? else {
        return Ok(0);
    };
    let cache_root = root.join("cache");
    let mut restored = 0_u64;
    for artifact in ownership.artifacts {
        validate_relative_cache_path(&artifact.relative_cache_path)?;
        let destination = cache_root.join(&artifact.relative_cache_path);
        let valid = fs::symlink_metadata(&destination)
            .ok()
            .is_some_and(|metadata| {
                !metadata.file_type().is_symlink()
                    && metadata.is_file()
                    && metadata.len() == artifact.bytes
                    && sha256_file(&destination)
                        .map(|actual| actual == artifact.sha256)
                        .unwrap_or(false)
            });
        if valid {
            continue;
        }

        // Schema v1 stores blob_path, but recovery never trusts a persisted path.
        // The only authoritative location is derived from Takokit's root + digest.
        let blob = provider_blob_path(root, &artifact.sha256)?;
        if !blob.is_file() {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: model_id.to_string(),
                reason: format!("durable provider blob is missing: {}", blob.display()),
            });
        }
        if sha256_file(&blob)? != artifact.sha256 {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: model_id.to_string(),
                reason: format!(
                    "durable provider blob failed SHA-256 verification: {}",
                    blob.display()
                ),
            });
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        remove_path_if_present(&destination)?;
        link_or_copy(&blob, &destination)?;
        restored = restored.saturating_add(artifact.bytes);
    }
    Ok(restored)
}

pub fn read_model_provider_ownership(
    root: &Path,
    model_id: &str,
) -> PackageResult<Option<ModelProviderOwnership>> {
    let path = ownership_path(root, model_id);
    recover_atomic_json(&path)?;
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
    validate_ledger(root, &ownership)?;
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
            .all(|ledger| verify_ledger_blobs(root, ledger).is_ok());
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
    let _guard = acquire_provider_ownership_lock(root)?;
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
    let _guard = acquire_provider_ownership_lock(root)?;
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
