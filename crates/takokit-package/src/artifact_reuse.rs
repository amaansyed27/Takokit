//! Verification and classification of artifacts from previous model pulls.

use crate::{
    artifact_io::sha256_file, ArtifactEntry, InstalledArtifactRecord, InstalledModelRecord,
    InstalledPackageStatus, ModelManifest,
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

/// A ready record is trustworthy only when it describes exactly the current
/// manifest and every recorded local file still verifies.
pub(crate) fn all_verified(record: &InstalledModelRecord, manifest: &ModelManifest) -> bool {
    if record.status != InstalledPackageStatus::Ready || manifest.artifacts.metadata_only {
        return false;
    }

    let expected = manifest.artifacts.all().collect::<Vec<_>>();
    if expected.is_empty() || record.artifacts.len() != expected.len() {
        return false;
    }

    expected.into_iter().all(|artifact| {
        record
            .artifacts
            .iter()
            .find(|candidate| {
                candidate.name == artifact.name && candidate.role == artifact.role
            })
            .is_some_and(|candidate| is_verified(candidate, artifact))
    })
}

/// Blob-path existence is deliberately insufficient: every field from the
/// current manifest and the actual local bytes must still match.
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
