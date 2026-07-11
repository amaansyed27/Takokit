//! Prior-artifact record verification used by repeat pulls.

use crate::{sha256_file, ArtifactEntry, InstalledArtifactRecord};

/// Blob-path existence is deliberately insufficient: every field from the
/// current manifest and the actual local bytes must still match.
pub(crate) fn is_verified(record: &InstalledArtifactRecord, artifact: &ArtifactEntry) -> bool {
    if record.name != artifact.name
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
