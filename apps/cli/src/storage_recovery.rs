//! Crash-safe journaling for mutating provider-aware storage cleanup.

use serde::Serialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const PROVIDER_CLEANUP_JOURNAL_SCHEMA: u32 = 1;
const PROVIDER_CLEANUP_JOURNAL_FILE: &str = "storage-cleanup-provider.json";

#[derive(Debug, Serialize)]
struct ProviderCleanupJournal<'a> {
    schema_version: u32,
    state: &'a str,
    scope: &'a str,
    recovery: &'a str,
}

/// Record a mutating provider cleanup before any candidate is deleted.
///
/// A surviving journal means the previous cleanup did not reach its successful
/// completion boundary. Recovery deliberately recomputes the cleanup plan under
/// the maintenance lock rather than replaying stale paths from the old process.
pub(crate) fn begin_provider_cleanup(root: &Path, scope: &str) -> anyhow::Result<bool> {
    let path = provider_cleanup_journal_path(root);
    if path.is_file() {
        // Never replace a surviving journal before recovery finishes. Keeping
        // the old file closes the crash window where delete-then-rename could
        // otherwise temporarily erase the only interruption marker on Windows.
        return Ok(true);
    }

    let journal = ProviderCleanupJournal {
        schema_version: PROVIDER_CLEANUP_JOURNAL_SCHEMA,
        state: "running",
        scope,
        recovery: "recompute-under-maintenance-lock",
    };
    write_new_json_atomic(&path, &journal)?;
    Ok(false)
}

/// Clear the provider cleanup journal only after cleanup completed successfully.
pub(crate) fn finish_provider_cleanup(root: &Path) -> anyhow::Result<()> {
    let path = provider_cleanup_journal_path(root);
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn provider_cleanup_journal_path(root: &Path) -> PathBuf {
    root.join("runtime").join(PROVIDER_CLEANUP_JOURNAL_FILE)
}

fn write_new_json_atomic(path: &Path, value: &impl Serialize) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use takokit_package::{
        capture_provider_ownership, migrate_legacy_provider_cache, read_model_provider_ownership,
        remove_model_provider_ownership, snapshot_provider_cache,
    };

    fn write_prefetch_marker(root: &Path, model_id: &str) {
        let marker = root
            .join("models")
            .join(model_id)
            .join(".takokit-prefetch.json");
        fs::create_dir_all(marker.parent().expect("marker parent")).expect("marker directory");
        fs::write(marker, b"{}").expect("prefetch marker");
    }

    #[test]
    fn cleanup_journal_marks_interruption_and_clears_after_success() {
        let root = tempfile::tempdir().expect("tempdir");
        assert!(!begin_provider_cleanup(root.path(), "all-safe").expect("first journal"));
        let journal_path = provider_cleanup_journal_path(root.path());
        assert!(journal_path.is_file());
        let first_journal = fs::read_to_string(&journal_path).expect("first journal contents");

        assert!(begin_provider_cleanup(root.path(), "unused").expect("recovery journal"));
        let recovered_journal = fs::read_to_string(&journal_path).expect("recovery journal contents");
        assert_eq!(recovered_journal, first_journal);
        assert!(recovered_journal.contains("recompute-under-maintenance-lock"));

        finish_provider_cleanup(root.path()).expect("finish cleanup");
        assert!(!journal_path.exists());
    }

    #[test]
    fn interrupted_legacy_migration_resumes_from_completed_model_ledger() {
        let root = tempfile::tempdir().expect("tempdir");
        write_prefetch_marker(root.path(), "model-a");
        write_prefetch_marker(root.path(), "model-b");

        let before = snapshot_provider_cache(root.path()).expect("empty snapshot");
        let checkpoint = root
            .path()
            .join("cache")
            .join("huggingface")
            .join("hub")
            .join("weights.bin");
        fs::create_dir_all(checkpoint.parent().expect("checkpoint parent"))
            .expect("provider directory");
        fs::write(&checkpoint, vec![7_u8; 4096]).expect("provider checkpoint");
        capture_provider_ownership(root.path(), "model-a", &before).expect("first model ownership");

        let journal = root
            .path()
            .join("runtime")
            .join("storage-migration-provider-ownership.json");
        fs::create_dir_all(journal.parent().expect("journal parent")).expect("runtime directory");
        fs::write(
            &journal,
            br#"{"schema_version":1,"state":"running","completed_models":["model-a"]}"#,
        )
        .expect("interrupted migration journal");

        let report = migrate_legacy_provider_cache(root.path()).expect("resumed migration");
        assert!(report.completed);
        assert!(report.already_owned_models.iter().any(|id| id == "model-a"));
        assert!(report.migrated_models.iter().any(|id| id == "model-b"));
        assert!(read_model_provider_ownership(root.path(), "model-b")
            .expect("model-b ledger read")
            .is_some());
        assert!(fs::read_to_string(report.journal)
            .expect("completed migration journal")
            .contains("completed"));
    }

    #[test]
    fn shared_provider_blob_survives_until_last_model_owner_is_removed() {
        let root = tempfile::tempdir().expect("tempdir");
        let before = snapshot_provider_cache(root.path()).expect("empty snapshot");
        let checkpoint = root
            .path()
            .join("cache")
            .join("huggingface")
            .join("hub")
            .join("shared.bin");
        fs::create_dir_all(checkpoint.parent().expect("checkpoint parent"))
            .expect("provider directory");
        fs::write(&checkpoint, vec![11_u8; 4096]).expect("provider checkpoint");

        capture_provider_ownership(root.path(), "model-a", &before).expect("model-a ownership");
        let current = snapshot_provider_cache(root.path()).expect("current snapshot");
        capture_provider_ownership(root.path(), "model-b", &current)
            .expect("model-b shared ownership");

        let ledger = read_model_provider_ownership(root.path(), "model-a")
            .expect("model-a ledger read")
            .expect("model-a ledger");
        let blob = ledger.artifacts[0].blob_path.clone();
        assert!(blob.is_file());

        remove_model_provider_ownership(root.path(), "model-a", false).expect("remove first owner");
        assert!(blob.is_file());
        assert!(read_model_provider_ownership(root.path(), "model-b")
            .expect("model-b ledger read")
            .is_some());

        remove_model_provider_ownership(root.path(), "model-b", false).expect("remove final owner");
        assert!(!blob.exists());
    }
}
