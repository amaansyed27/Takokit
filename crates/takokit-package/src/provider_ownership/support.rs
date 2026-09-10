use super::storage::{
    canonical_provider_cache_file, collect_files, link_or_copy, now_nanos, ownership_path,
    provider_blob_path, provider_blob_root, recover_atomic_json, remove_path_if_present,
    validate_ledger, validate_relative_cache_path, write_json_atomic,
};
use super::{
    ModelProviderOwnership, ProviderCleanupItem, ProviderOwnedArtifact, PROVIDER_OWNERSHIP_SCHEMA,
};
use crate::{artifact_io::sha256_file, PackageError, PackageResult};
use std::{collections::HashSet, fs, path::Path};

pub(super) fn materialize_owned_artifact(
    root: &Path,
    relative: &Path,
) -> PackageResult<ProviderOwnedArtifact> {
    validate_relative_cache_path(relative)?;
    let source = canonical_provider_cache_file(root, relative)?;
    let metadata = fs::metadata(&source)?;
    if !metadata.is_file() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: relative.display().to_string(),
            reason: "provider ownership source is not a regular file".to_string(),
        });
    }
    let sha256 = sha256_file(&source)?;
    let blob = provider_blob_path(root, &sha256)?;
    if !blob.is_file() {
        if let Some(parent) = blob.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = blob.with_extension(format!("tmp-{}-{}", std::process::id(), now_nanos()));
        remove_path_if_present(&temporary)?;
        link_or_copy(&source, &temporary)?;
        if sha256_file(&temporary)? != sha256 {
            let _ = remove_path_if_present(&temporary);
            return Err(PackageError::ArtifactInstallFailed {
                artifact: relative.display().to_string(),
                reason: "provider blob changed while it was being materialized".to_string(),
            });
        }
        match fs::rename(&temporary, &blob) {
            Ok(()) => {}
            Err(error) if blob.is_file() => {
                let _ = remove_path_if_present(&temporary);
                let _ = error;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(ProviderOwnedArtifact {
        provider: relative
            .components()
            .next()
            .and_then(|part| part.as_os_str().to_str())
            .unwrap_or("unknown")
            .to_string(),
        relative_cache_path: relative.to_path_buf(),
        sha256,
        bytes: metadata.len(),
        blob_path: blob,
    })
}

pub(super) fn write_model_provider_ownership(
    root: &Path,
    ownership: &ModelProviderOwnership,
) -> PackageResult<()> {
    let path = ownership_path(root, &ownership.model_id);
    write_json_atomic(&path, ownership)
}

pub(super) fn discover_prefetched_models(root: &Path) -> PackageResult<Vec<String>> {
    let models_root = root.join("models");
    if !models_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut models = Vec::new();
    for entry in fs::read_dir(models_root)? {
        let entry = entry?;
        if !entry.path().is_dir() || !entry.path().join(".takokit-prefetch.json").is_file() {
            continue;
        }
        if let Some(id) = entry.file_name().to_str() {
            models.push(id.to_string());
        }
    }
    models.sort();
    Ok(models)
}

pub(super) fn read_all_ledgers(root: &Path) -> PackageResult<Vec<ModelProviderOwnership>> {
    let directory = root.join("manifests").join("ownership").join("models");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut ledgers = Vec::new();
    for path in paths {
        recover_atomic_json(&path)?;
        let ledger: ModelProviderOwnership = serde_json::from_slice(&fs::read(&path)?)?;
        if ledger.schema_version == PROVIDER_OWNERSHIP_SCHEMA {
            validate_ledger(root, &ledger)?;
            ledgers.push(ledger);
        }
    }
    Ok(ledgers)
}

pub(super) fn collect_unused_blobs(
    root: &Path,
    removed: &mut Vec<ProviderCleanupItem>,
    retained: &mut Vec<ProviderCleanupItem>,
) -> PackageResult<()> {
    collect_unused_blobs_ignoring(root, &HashSet::new(), removed, retained)
}

pub(super) fn collect_unused_blobs_ignoring(
    root: &Path,
    ignored_models: &HashSet<String>,
    removed: &mut Vec<ProviderCleanupItem>,
    retained: &mut Vec<ProviderCleanupItem>,
) -> PackageResult<()> {
    let ledgers = read_all_ledgers(root)?;
    let mut referenced = HashSet::new();
    for ledger in ledgers
        .iter()
        .filter(|ledger| !ignored_models.contains(&ledger.model_id))
    {
        for artifact in &ledger.artifacts {
            referenced.insert(provider_blob_path(root, &artifact.sha256)?);
        }
    }
    let blobs = provider_blob_root(root);
    let mut files = Vec::new();
    collect_files(&blobs, &mut files)?;
    files.sort();
    for path in files {
        let item = ProviderCleanupItem {
            category: "provider-blob".to_string(),
            bytes: fs::metadata(&path)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
            path: path.clone(),
            reason: if referenced.contains(&path) {
                "retained because an installed model references this durable blob".to_string()
            } else {
                "no remaining model ownership ledger references this durable blob".to_string()
            },
        };
        if referenced.contains(&path) {
            retained.push(item);
        } else {
            removed.push(item);
        }
    }
    Ok(())
}

pub(super) fn verify_ledger_blobs(
    root: &Path,
    ledger: &ModelProviderOwnership,
) -> PackageResult<()> {
    validate_ledger(root, ledger)?;
    for artifact in &ledger.artifacts {
        let blob = provider_blob_path(root, &artifact.sha256)?;
        if !blob.is_file()
            || fs::metadata(&blob)?.len() != artifact.bytes
            || sha256_file(&blob)? != artifact.sha256
        {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: ledger.model_id.clone(),
                reason: format!(
                    "durable provider blob verification failed: {}",
                    blob.display()
                ),
            });
        }
    }
    Ok(())
}
