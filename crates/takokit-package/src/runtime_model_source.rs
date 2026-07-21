//! Transactional installation of repository-backed model snapshots.

use crate::{
    runtime_command::{configure_managed_command, run_logged_command, PathOrArg},
    runtime_uv::bootstrap_uv,
    *,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const READY_MARKER: &str = ".takokit-source.json";
const PARTIAL_MARKER: &str = ".takokit-partial-source.json";

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct SourceMarker {
    provider: ModelSourceProvider,
    repository: String,
    revision: String,
}

pub(crate) fn install_model_source(
    takokit_root: &Path,
    manifest: &ModelManifest,
    previous: Option<&InstalledModelRecord>,
) -> PackageResult<Option<InstalledSnapshotRecord>> {
    let Some(source) = manifest.source.as_ref() else {
        return Ok(None);
    };
    match source.provider {
        ModelSourceProvider::HuggingFace => {
            install_hugging_face_snapshot(takokit_root, manifest, source, previous).map(Some)
        }
    }
}

pub(crate) fn estimate_model_source_bytes(
    takokit_root: &Path,
    manifest: &ModelManifest,
) -> Option<u64> {
    let source = manifest.source.as_ref()?;
    match source.provider {
        ModelSourceProvider::HuggingFace => {
            estimate_hugging_face_snapshot_bytes(takokit_root, source)
        }
    }
}

pub(crate) fn model_source_staging_path(takokit_root: &Path, model_id: &str) -> PathBuf {
    takokit_root
        .join("models")
        .join(format!(".{model_id}.download"))
}

pub(crate) fn snapshot_is_ready(
    record: &InstalledSnapshotRecord,
    source: &ModelSourceManifest,
) -> bool {
    if record.provider != source.provider
        || record.repository != source.repository
        || record.revision != source.revision
        || !record.local_path.is_dir()
        || !record.ready_marker.is_file()
    {
        return false;
    }
    read_marker(&record.ready_marker).is_some_and(|marker| marker_matches(&marker, source))
}

fn estimate_hugging_face_snapshot_bytes(
    takokit_root: &Path,
    source: &ModelSourceManifest,
) -> Option<u64> {
    let uv = bootstrap_uv(takokit_root).ok()?;
    let cache_root = takokit_root.join("cache").join("huggingface");
    let mut arguments: Vec<PathOrArg> = vec![
        "tool".into(),
        "run".into(),
        "--from".into(),
        "huggingface_hub".into(),
        "hf".into(),
        "download".into(),
        source.repository.clone().into(),
        "--revision".into(),
        source.revision.clone().into(),
        "--cache-dir".into(),
        cache_root.into(),
        "--dry-run".into(),
    ];
    append_source_filters(&mut arguments, source);

    let mut command = Command::new(&uv);
    for argument in &arguments {
        command.arg(argument.as_os_str());
    }
    configure_managed_command(&mut command);
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_dry_run_total(&text)
}

fn install_hugging_face_snapshot(
    takokit_root: &Path,
    manifest: &ModelManifest,
    source: &ModelSourceManifest,
    previous: Option<&InstalledModelRecord>,
) -> PackageResult<InstalledSnapshotRecord> {
    if let Some(record) = previous.and_then(|record| record.snapshot.as_ref()) {
        if snapshot_is_ready(record, source) {
            return Ok(record.clone());
        }
    }

    let models_root = takokit_root.join("models");
    let cache_root = takokit_root.join("cache").join("huggingface");
    let logs_root = takokit_root.join("logs");
    std::fs::create_dir_all(&models_root)?;
    std::fs::create_dir_all(&cache_root)?;
    std::fs::create_dir_all(&logs_root)?;

    let destination = models_root.join(&manifest.id);
    let temporary = model_source_staging_path(takokit_root, &manifest.id);
    let backup = models_root.join(format!(".{}.backup", manifest.id));
    prepare_staging_dir(&temporary, source)?;
    remove_dir_if_exists(&backup)?;

    let uv = bootstrap_uv(takokit_root)?;
    let log = logs_root.join(format!("model-{}-download.log", manifest.id));
    let mut arguments: Vec<PathOrArg> = vec![
        "tool".into(),
        "run".into(),
        "--from".into(),
        "huggingface_hub".into(),
        "hf".into(),
        "download".into(),
        source.repository.clone().into(),
        "--revision".into(),
        source.revision.clone().into(),
        "--local-dir".into(),
        temporary.clone().into(),
        "--cache-dir".into(),
        cache_root.into(),
    ];
    append_source_filters(&mut arguments, source);

    if let Err(error) = run_logged_command(&log, &uv, &arguments) {
        return Err(PackageError::ArtifactDownloadFailed {
            artifact: manifest.id.clone(),
            reason: format!(
                "Hugging Face snapshot download failed for {}@{}: {error}; partial data was retained at {} for resume; see {}",
                source.repository,
                source.revision,
                temporary.display(),
                log.display()
            ),
        });
    }

    let marker = temporary.join(READY_MARKER);
    std::fs::write(&marker, serde_json::to_vec_pretty(&source_marker(source))?)?;

    if destination.exists() {
        std::fs::rename(&destination, &backup)?;
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        if backup.exists() {
            let _ = std::fs::rename(&backup, &destination);
        }
        return Err(PackageError::Io(error));
    }
    let _ = std::fs::remove_file(destination.join(PARTIAL_MARKER));
    remove_dir_if_exists(&backup)?;

    Ok(InstalledSnapshotRecord {
        provider: source.provider,
        repository: source.repository.clone(),
        revision: source.revision.clone(),
        local_path: destination.clone(),
        ready_marker: destination.join(READY_MARKER),
    })
}

fn prepare_staging_dir(path: &Path, source: &ModelSourceManifest) -> PackageResult<()> {
    if path.exists() {
        let matches = read_marker(&path.join(PARTIAL_MARKER))
            .is_some_and(|marker| marker_matches(&marker, source));
        if !matches {
            remove_dir_if_exists(path)?;
        }
    }
    std::fs::create_dir_all(path)?;
    std::fs::write(
        path.join(PARTIAL_MARKER),
        serde_json::to_vec_pretty(&source_marker(source))?,
    )?;
    Ok(())
}

fn append_source_filters(arguments: &mut Vec<PathOrArg>, source: &ModelSourceManifest) {
    for pattern in &source.allow_patterns {
        arguments.push("--include".into());
        arguments.push(pattern.clone().into());
    }
    for pattern in &source.ignore_patterns {
        arguments.push("--exclude".into());
        arguments.push(pattern.clone().into());
    }
}

fn source_marker(source: &ModelSourceManifest) -> SourceMarker {
    SourceMarker {
        provider: source.provider,
        repository: source.repository.clone(),
        revision: source.revision.clone(),
    }
}

fn marker_matches(marker: &SourceMarker, source: &ModelSourceManifest) -> bool {
    marker.provider == source.provider
        && marker.repository == source.repository
        && marker.revision == source.revision
}

fn parse_dry_run_total(text: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let tail = line
            .split_once("totalling ")
            .or_else(|| line.split_once("totaling "))?
            .1;
        let token = tail
            .split_whitespace()
            .next()?
            .trim_end_matches(|character: char| character == '.' || character == ',');
        parse_human_bytes(token)
    })
}

fn parse_human_bytes(value: &str) -> Option<u64> {
    let split = value
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(value.len());
    let number = value[..split].parse::<f64>().ok()?;
    let unit = value[split..].trim().to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "" | "B" => 1_f64,
        "K" | "KB" => 1_000_f64,
        "KIB" => 1_024_f64,
        "M" | "MB" => 1_000_000_f64,
        "MIB" => 1_048_576_f64,
        "G" | "GB" => 1_000_000_000_f64,
        "GIB" => 1_073_741_824_f64,
        "T" | "TB" => 1_000_000_000_000_f64,
        "TIB" => 1_099_511_627_776_f64,
        _ => return None,
    };
    Some((number * multiplier).round() as u64)
}

fn read_marker(path: &Path) -> Option<SourceMarker> {
    let source = std::fs::read(path).ok()?;
    serde_json::from_slice(&source).ok()
}

fn remove_dir_if_exists(path: &Path) -> PackageResult<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PackageError::Io(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_readiness_requires_matching_marker() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join(READY_MARKER);
        let source = ModelSourceManifest {
            provider: ModelSourceProvider::HuggingFace,
            repository: "owner/model".into(),
            revision: "0123456789abcdef".into(),
            allow_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
        };
        std::fs::write(
            &marker,
            serde_json::to_vec(&source_marker(&source)).expect("marker"),
        )
        .expect("write marker");
        let record = InstalledSnapshotRecord {
            provider: source.provider,
            repository: source.repository.clone(),
            revision: source.revision.clone(),
            local_path: temp.path().to_path_buf(),
            ready_marker: marker,
        };
        assert!(snapshot_is_ready(&record, &source));
    }

    #[test]
    fn dry_run_total_parser_supports_hugging_face_units() {
        assert_eq!(
            parse_dry_run_total("Will download 4 files totalling 2.5G."),
            Some(2_500_000_000)
        );
        assert_eq!(
            parse_dry_run_total("Will download 4 files totaling 148.2M."),
            Some(148_200_000)
        );
    }

    #[test]
    fn matching_partial_snapshot_is_kept_for_resume() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join(".model.download");
        let source = ModelSourceManifest {
            provider: ModelSourceProvider::HuggingFace,
            repository: "owner/model".into(),
            revision: "revision-a".into(),
            allow_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
        };
        prepare_staging_dir(&path, &source).expect("prepare");
        std::fs::write(path.join("partial.bin"), b"partial").expect("partial");
        prepare_staging_dir(&path, &source).expect("resume");
        assert!(path.join("partial.bin").is_file());
    }
}
