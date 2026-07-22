//! Verification and classification of artifacts and snapshots from previous pulls.

use crate::{
    artifact_io::sha256_file, runtime_model_source::snapshot_is_ready,
    runtime_python_specs::model_prefetch_required, ArtifactEntry, InstalledArtifactRecord,
    InstalledModelRecord, InstalledPackageStatus, ModelManifest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactReuseState {
    Missing,
    Verified,
    RepairRequired,
}

pub(crate) fn classify(
    record: Option<&InstalledModelRecord>,
    manifest: &ModelManifest,
) -> ArtifactReuseState {
    let Some(record) = record else {
        return ArtifactReuseState::Missing;
    };
    if record.status != InstalledPackageStatus::Ready {
        return ArtifactReuseState::Missing;
    }
    if all_verified(record, manifest) {
        ArtifactReuseState::Verified
    } else {
        ArtifactReuseState::RepairRequired
    }
}

/// A ready record is trustworthy only when it describes the current manifest
/// and every declared local artifact or pinned snapshot still verifies.
pub(crate) fn all_verified(record: &InstalledModelRecord, manifest: &ModelManifest) -> bool {
    if record.status != InstalledPackageStatus::Ready || manifest.artifacts.metadata_only {
        return false;
    }

    let source_ready = match manifest.source.as_ref() {
        Some(source) => record
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot_is_ready(snapshot, source)),
        None => true,
    };
    if !source_ready {
        return false;
    }

    let expected = manifest.artifacts.all().collect::<Vec<_>>();
    if expected.is_empty() {
        if manifest.source.is_some() {
            return true;
        }
        return runtime_prefetch_is_ready(record, manifest);
    }
    if record.artifacts.len() != expected.len() {
        return false;
    }
    expected.into_iter().all(|artifact| {
        record
            .artifacts
            .iter()
            .find(|candidate| candidate.name == artifact.name && candidate.role == artifact.role)
            .is_some_and(|candidate| is_verified(candidate, artifact))
    })
}

fn runtime_prefetch_is_ready(record: &InstalledModelRecord, manifest: &ModelManifest) -> bool {
    if !model_prefetch_required(&manifest.id) {
        return false;
    }
    let Some(storage_root) = record.manifest_path.ancestors().nth(3) else {
        return false;
    };
    let marker = storage_root
        .join("models")
        .join(&manifest.id)
        .join(".takokit-prefetch.json");
    let Ok(source) = std::fs::read(marker) else {
        return false;
    };
    let Ok(marker) = serde_json::from_slice::<serde_json::Value>(&source) else {
        return false;
    };
    marker.get("model_id").and_then(|value| value.as_str()) == Some(manifest.id.as_str())
        && marker.get("model_version").and_then(|value| value.as_str())
            == Some(manifest.version.as_str())
        && marker.get("adapter").and_then(|value| value.as_str())
            == manifest.required_adapter.as_deref()
        && manifest.required_adapter.as_deref().is_some_and(|adapter| {
            let script = storage_root
                .join("runners")
                .join("python-managed")
                .join("adapters")
                .join(adapter)
                .join(format!("{adapter}.py"));
            marker
                .get("adapter_script_sha256")
                .and_then(|value| value.as_str())
                .is_some_and(|expected| {
                    sha256_file(&script)
                        .is_ok_and(|actual| actual.eq_ignore_ascii_case(expected))
                })
        })
}

/// Blob-path existence is deliberately insufficient: every manifest field and
/// the actual local bytes must still match.
pub(crate) fn is_verified(record: &InstalledArtifactRecord, artifact: &ArtifactEntry) -> bool {
    if record.name != artifact.name
        || record.role != artifact.role
        || !record
            .sha256
            .trim()
            .eq_ignore_ascii_case(artifact.sha256.trim())
        || record.bytes != artifact.bytes
        || !record.downloaded
    {
        return false;
    }
    let Some(path) = record.local_path.as_ref() else {
        return false;
    };
    if !path.is_file() {
        return false;
    }
    if let Some(expected) = artifact.bytes {
        if std::fs::metadata(path).map(|metadata| metadata.len()).ok() != Some(expected) {
            return false;
        }
    }
    sha256_file(path)
        .map(|actual| actual.eq_ignore_ascii_case(artifact.sha256.trim()))
        .unwrap_or(false)
}
