//! Snapshot and rollback support around final model-plan verification.

use crate::{
    install_support::{remove_file_if_exists, write_model_install_files},
    InstalledRegistry, PackageResult,
};
use std::{io::ErrorKind, path::Path};

#[derive(Debug, Clone)]
pub(crate) struct ModelInstallSnapshot {
    manifest: Option<String>,
    record: Option<String>,
}

impl ModelInstallSnapshot {
    pub(crate) fn capture(registry: &InstalledRegistry, model_id: &str) -> PackageResult<Self> {
        Ok(Self {
            manifest: read_optional(&registry.model_manifest_path(model_id))?,
            record: read_optional(&registry.model_record_path(model_id))?,
        })
    }

    pub(crate) fn restore(
        self,
        registry: &InstalledRegistry,
        model_id: &str,
    ) -> PackageResult<()> {
        let manifest_path = registry.model_manifest_path(model_id);
        let record_path = registry.model_record_path(model_id);

        match (self.manifest, self.record) {
            (Some(manifest), Some(record)) => {
                write_model_install_files(&manifest_path, &record_path, &manifest, &record)
            }
            (manifest, record) => {
                remove_file_if_exists(manifest_path.clone())?;
                remove_file_if_exists(record_path.clone())?;
                if let Some(manifest) = manifest {
                    ensure_parent(&manifest_path)?;
                    std::fs::write(&manifest_path, manifest)?;
                }
                if let Some(record) = record {
                    ensure_parent(&record_path)?;
                    std::fs::write(&record_path, record)?;
                }
                Ok(())
            }
        }
    }
}

fn read_optional(path: &Path) -> PackageResult<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn ensure_parent(path: &Path) -> PackageResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_previous_manifest_and_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = InstalledRegistry::new(temp.path().join("manifests"));
        let manifest_path = registry.model_manifest_path("fixture");
        let record_path = registry.model_record_path("fixture");
        ensure_parent(&manifest_path).unwrap();
        ensure_parent(&record_path).unwrap();
        std::fs::write(&manifest_path, "old manifest").unwrap();
        std::fs::write(&record_path, "old record").unwrap();

        let snapshot = ModelInstallSnapshot::capture(&registry, "fixture").unwrap();
        std::fs::write(&manifest_path, "new manifest").unwrap();
        std::fs::write(&record_path, "new record").unwrap();
        snapshot.restore(&registry, "fixture").unwrap();

        assert_eq!(std::fs::read_to_string(manifest_path).unwrap(), "old manifest");
        assert_eq!(std::fs::read_to_string(record_path).unwrap(), "old record");
    }

    #[test]
    fn removes_new_install_when_snapshot_was_empty() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = InstalledRegistry::new(temp.path().join("manifests"));
        let snapshot = ModelInstallSnapshot::capture(&registry, "fixture").unwrap();
        let manifest_path = registry.model_manifest_path("fixture");
        let record_path = registry.model_record_path("fixture");
        ensure_parent(&manifest_path).unwrap();
        ensure_parent(&record_path).unwrap();
        std::fs::write(&manifest_path, "new manifest").unwrap();
        std::fs::write(&record_path, "new record").unwrap();

        snapshot.restore(&registry, "fixture").unwrap();

        assert!(!manifest_path.exists());
        assert!(!record_path.exists());
    }
}
