//! Verified local model inventory and Ollama-style `tako list` data.

use crate::{
    artifact_reuse, InstalledModelRecord, InstalledModelSummary, InstalledModelsResponse,
    InstalledPackageStatus, InstalledRegistry, ModelManifest, PackageRegistry, PackageResult,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

impl InstalledRegistry {
    pub fn installed_model_inventory(
        &self,
        package_registry: &PackageRegistry,
    ) -> PackageResult<InstalledModelsResponse> {
        let mut data = Vec::new();

        for record in self.installed_model_records()? {
            if record.status != InstalledPackageStatus::Ready {
                continue;
            }
            let Ok(manifest) = package_registry.model(&record.id) else {
                continue;
            };
            if !artifact_reuse::all_verified(&record, &manifest) {
                continue;
            }

            data.push(InstalledModelSummary {
                name: record.id.clone(),
                id: inventory_digest(&manifest, &record),
                size_bytes: installed_size(&record),
                modified_at: record.installed_at.parse::<u64>().unwrap_or_default(),
                version: record.version.clone(),
                runner: record.runner.clone(),
            });
        }

        data.sort_by(|left, right| {
            right
                .modified_at
                .cmp(&left.modified_at)
                .then_with(|| left.name.cmp(&right.name))
        });

        Ok(InstalledModelsResponse {
            kind: "installed-models".to_string(),
            data,
        })
    }
}

fn inventory_digest(manifest: &ModelManifest, record: &InstalledModelRecord) -> String {
    let mut hasher = Sha256::new();
    hasher.update(manifest.id.as_bytes());
    hasher.update([0]);
    hasher.update(manifest.version.as_bytes());
    hasher.update([0]);
    hasher.update(record.source.as_bytes());

    for artifact in &record.artifacts {
        hasher.update([0]);
        hasher.update(artifact.name.as_bytes());
        hasher.update([0]);
        hasher.update(artifact.sha256.as_bytes());
    }
    if let Some(snapshot) = &record.snapshot {
        hasher.update([0]);
        hasher.update(snapshot.repository.as_bytes());
        hasher.update([0]);
        hasher.update(snapshot.revision.as_bytes());
    }

    let digest = format!("{:x}", hasher.finalize());
    digest[..12].to_string()
}

fn installed_size(record: &InstalledModelRecord) -> u64 {
    let snapshot_root = record
        .snapshot
        .as_ref()
        .map(|snapshot| &snapshot.local_path);
    let mut seen = HashSet::<PathBuf>::new();
    let mut total = 0_u64;

    if let Some(root) = snapshot_root {
        seen.insert(root.clone());
        total = total.saturating_add(path_size(root));
    }

    for artifact in &record.artifacts {
        let Some(path) = artifact.local_path.as_ref() else {
            continue;
        };
        if snapshot_root.is_some_and(|root| path.starts_with(root)) || !seen.insert(path.clone()) {
            continue;
        }
        total = total.saturating_add(path_size(path));
    }

    total
}

fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    if !metadata.is_dir() {
        return 0;
    }

    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| path_size(&entry.path()))
        .fold(0_u64, u64::saturating_add)
}
