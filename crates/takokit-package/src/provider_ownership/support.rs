use super::*;
use fs2::FileExt;
use std::{fs::OpenOptions, io::Write};

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
        let temporary = blob.with_extension(format!(
            "tmp-{}-{}",
            std::process::id(),
            now_nanos()
        ));
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

pub(super) fn provider_blob_path(root: &Path, sha256: &str) -> PackageResult<PathBuf> {
    validate_sha256(sha256)?;
    Ok(provider_blob_root(root)
        .join(&sha256[0..2])
        .join(sha256))
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

pub(super) fn scan_cache_files(
    base: &Path,
    current: &Path,
    provider: &str,
    files: &mut BTreeMap<PathBuf, FileSignature>,
) -> PackageResult<()> {
    if !current.exists() {
        return Ok(());
    }

    let link_metadata = fs::symlink_metadata(current)?;
    if link_metadata.file_type().is_symlink() {
        let canonical_base = fs::canonicalize(base)?;
        let canonical_target = fs::canonicalize(current)?;
        if !canonical_target.starts_with(&canonical_base) {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: current.display().to_string(),
                reason: "provider cache symlink escaped its provider root".to_string(),
            });
        }
        let metadata = fs::metadata(&canonical_target)?;
        if metadata.is_dir() {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: current.display().to_string(),
                reason: "provider cache directory symlinks are not supported".to_string(),
            });
        }
        if metadata.is_file() {
            record_cache_file(base, current, provider, metadata, files)?;
        }
        return Ok(());
    }

    if link_metadata.is_file() {
        record_cache_file(base, current, provider, link_metadata, files)?;
        return Ok(());
    }
    if link_metadata.is_dir() {
        let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            scan_cache_files(base, &entry.path(), provider, files)?;
        }
    }
    Ok(())
}

fn record_cache_file(
    base: &Path,
    current: &Path,
    provider: &str,
    metadata: fs::Metadata,
    files: &mut BTreeMap<PathBuf, FileSignature>,
) -> PackageResult<()> {
    let relative = current
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

fn canonical_provider_cache_file(root: &Path, relative: &Path) -> PackageResult<PathBuf> {
    validate_relative_cache_path(relative)?;
    let provider = relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .unwrap_or_default();
    let provider_root = root.join("cache").join(provider);
    let canonical_root = fs::canonicalize(&provider_root).map_err(|error| {
        PackageError::ArtifactInstallFailed {
            artifact: relative.display().to_string(),
            reason: format!("could not resolve provider cache root: {error}"),
        }
    })?;
    let source = root.join("cache").join(relative);
    let canonical_source = fs::canonicalize(&source).map_err(|error| {
        PackageError::ArtifactInstallFailed {
            artifact: relative.display().to_string(),
            reason: format!("could not resolve provider cache artifact: {error}"),
        }
    })?;
    if !canonical_source.starts_with(&canonical_root) {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: relative.display().to_string(),
            reason: "provider cache artifact escaped its provider root".to_string(),
        });
    }
    Ok(canonical_source)
}

fn validate_sha256(value: &str) -> PackageResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: value.to_string(),
            reason: "provider blob SHA-256 is invalid".to_string(),
        });
    }
    Ok(())
}

pub(super) fn validate_ledger(
    root: &Path,
    ledger: &ModelProviderOwnership,
) -> PackageResult<()> {
    if ledger.schema_version != PROVIDER_OWNERSHIP_SCHEMA {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: ledger.model_id.clone(),
            reason: "provider ownership ledger schema mismatch".to_string(),
        });
    }
    for artifact in &ledger.artifacts {
        validate_relative_cache_path(&artifact.relative_cache_path)?;
        validate_sha256(&artifact.sha256)?;
        let expected_blob = provider_blob_path(root, &artifact.sha256)?;
        // blob_path is retained in schema v1 for backwards compatibility, but it
        // is never trusted for reads/deletes. New ledgers always write the derived
        // Takokit-owned location.
        if artifact.blob_path.as_os_str().is_empty() {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: ledger.model_id.clone(),
                reason: "provider ownership ledger contains an empty blob path".to_string(),
            });
        }
        let _ = expected_blob;
    }
    Ok(())
}

pub(super) fn write_json_atomic(path: &Path, value: &impl Serialize) -> PackageResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let lock_path = path.with_extension("lock");
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.lock_exclusive()?;

    recover_atomic_json(path)?;
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        now_nanos()
    ));
    let source = serde_json::to_vec_pretty(value)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(&source)?;
    file.sync_all()?;
    drop(file);

    replace_atomic_json(path, &temporary)?;
    sync_parent(path);
    Ok(())
}

pub(super) fn recover_atomic_json(path: &Path) -> PackageResult<()> {
    #[cfg(windows)]
    {
        let backup = atomic_backup_path(path);
        if !path.exists() && backup.is_file() {
            fs::rename(&backup, path)?;
        } else if path.exists() && backup.exists() {
            remove_path_if_present(&backup)?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_atomic_json(path: &Path, temporary: &Path) -> PackageResult<()> {
    fs::rename(temporary, path).map_err(Into::into)
}

#[cfg(windows)]
fn replace_atomic_json(path: &Path, temporary: &Path) -> PackageResult<()> {
    if !path.exists() {
        return fs::rename(temporary, path).map_err(Into::into);
    }
    let backup = atomic_backup_path(path);
    remove_path_if_present(&backup)?;
    fs::rename(path, &backup)?;
    match fs::rename(temporary, path) {
        Ok(()) => {
            remove_path_if_present(&backup)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup, path);
            let _ = remove_path_if_present(temporary);
            Err(error.into())
        }
    }
}

#[cfg(windows)]
fn atomic_backup_path(path: &Path) -> PathBuf {
    path.with_extension("bak")
}

fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
    }
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

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
