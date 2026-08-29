use super::*;

pub(super) fn materialize_owned_artifact(
    root: &Path,
    relative: &Path,
) -> PackageResult<ProviderOwnedArtifact> {
    validate_relative_cache_path(relative)?;
    let source = root.join("cache").join(relative);
    let metadata = fs::metadata(&source)?;
    if !metadata.is_file() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: relative.display().to_string(),
            reason: "provider ownership source is not a regular file".to_string(),
        });
    }
    let sha256 = sha256_file(&source)?;
    let blob = provider_blob_root(root).join(&sha256[0..2]).join(&sha256);
    if !blob.is_file() {
        if let Some(parent) = blob.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = blob.with_extension(format!("tmp-{}", std::process::id()));
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

pub(super) fn ownership_path(root: &Path, model_id: &str) -> PathBuf {
    root.join("manifests")
        .join("ownership")
        .join("models")
        .join(format!("{}.json", safe_id(model_id)))
}

pub(super) fn provider_blob_root(root: &Path) -> PathBuf {
    root.join("blobs").join("provider").join("sha256")
}

pub(super) fn migration_journal_path(root: &Path) -> PathBuf {
    root.join("runtime")
        .join("storage-migration-provider-ownership.json")
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
        let ledger: ModelProviderOwnership = serde_json::from_slice(&fs::read(path)?)?;
        if ledger.schema_version == PROVIDER_OWNERSHIP_SCHEMA {
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
    let referenced = ledgers
        .iter()
        .filter(|ledger| !ignored_models.contains(&ledger.model_id))
        .flat_map(|ledger| {
            ledger
                .artifacts
                .iter()
                .map(|artifact| artifact.blob_path.clone())
        })
        .collect::<HashSet<_>>();
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

pub(super) fn verify_ledger_blobs(ledger: &ModelProviderOwnership) -> PackageResult<()> {
    for artifact in &ledger.artifacts {
        if !artifact.blob_path.is_file()
            || fs::metadata(&artifact.blob_path)?.len() != artifact.bytes
            || sha256_file(&artifact.blob_path)? != artifact.sha256
        {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: ledger.model_id.clone(),
                reason: format!(
                    "durable provider blob verification failed: {}",
                    artifact.blob_path.display()
                ),
            });
        }
    }
    Ok(())
}

pub(super) fn scan_cache_files(
    base: &Path,
    current: &Path,
    provider: &str,
    files: &mut BTreeMap<PathBuf, FileSignature>,
) -> PackageResult<()> {
    if !current.exists() {
        return Ok(());
    }
    let metadata = fs::metadata(current)?;
    if metadata.is_file() {
        let relative =
            current
                .strip_prefix(base)
                .map_err(|_| PackageError::ArtifactInstallFailed {
                    artifact: provider.to_string(),
                    reason: "provider cache path escaped its provider root".to_string(),
                })?;
        let relative = PathBuf::from(provider).join(relative);
        let modified_nanos = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        files.insert(
            relative,
            FileSignature {
                bytes: metadata.len(),
                modified_nanos,
            },
        );
        return Ok(());
    }
    if metadata.is_dir() {
        let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            scan_cache_files(base, &entry.path(), provider, files)?;
        }
    }
    Ok(())
}

pub(super) fn snapshot_totals(snapshot: &ProviderCacheSnapshot) -> (u64, u64) {
    (
        snapshot.files.len() as u64,
        snapshot
            .files
            .values()
            .map(|signature| signature.bytes)
            .fold(0_u64, u64::saturating_add),
    )
}

pub(super) fn collect_tree(
    path: PathBuf,
    category: &str,
    reason: &str,
    output: &mut Vec<ProviderCleanupItem>,
) -> PackageResult<()> {
    if path.exists() {
        output.push(ProviderCleanupItem {
            category: category.to_string(),
            bytes: path_size(&path),
            path,
            reason: reason.to_string(),
        });
    }
    Ok(())
}

fn collect_files(path: &Path, output: &mut Vec<PathBuf>) -> PackageResult<()> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_file() {
        output.push(path.to_path_buf());
    } else if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            collect_files(&entry?.path(), output)?;
        }
    }
    Ok(())
}

pub(super) fn scan_regular_file_totals(path: &Path) -> PackageResult<(u64, u64)> {
    let mut files = Vec::new();
    collect_files(path, &mut files)?;
    let bytes = files
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .fold(0_u64, u64::saturating_add);
    Ok((files.len() as u64, bytes))
}

pub(super) fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| path_size(&entry.path()))
        .fold(0_u64, u64::saturating_add)
}

pub(super) fn collapse_nested(items: &mut Vec<ProviderCleanupItem>) {
    items.sort_by(|left, right| {
        left.path
            .components()
            .count()
            .cmp(&right.path.components().count())
            .then_with(|| left.path.cmp(&right.path))
    });
    let mut collapsed = Vec::<ProviderCleanupItem>::new();
    for item in items.drain(..) {
        if collapsed
            .iter()
            .any(|parent| item.path.starts_with(&parent.path))
        {
            continue;
        }
        collapsed.push(item);
    }
    *items = collapsed;
}

pub(super) fn link_or_copy(source: &Path, destination: &Path) -> PackageResult<()> {
    let source_is_symlink = fs::symlink_metadata(source)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
    if !source_is_symlink && fs::hard_link(source, destination).is_ok() {
        return Ok(());
    }
    fs::copy(source, destination)?;
    Ok(())
}

pub(super) fn remove_path_if_present(path: &Path) -> PackageResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)?
        }
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
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

pub(super) fn write_json_atomic(path: &Path, value: &impl Serialize) -> PackageResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp-{}-{}", std::process::id(), now()));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        PackageError::Io(error)
    })?;
    Ok(())
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
