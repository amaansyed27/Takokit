//! Verified local model inventory and Ollama-style `tako list` data.

use crate::{
    artifact_reuse, InstalledModelRecord, InstalledModelSummary, InstalledModelsResponse,
    InstalledPackageStatus, InstalledRegistry, ModelKind, ModelManifest, PackageRegistry,
    PackageResult,
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
                model_type: model_type_label(&manifest),
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

fn model_type_label(manifest: &ModelManifest) -> String {
    let mut labels = Vec::new();
    if manifest.capabilities.tts {
        labels.push("TTS");
    }
    if manifest.capabilities.stt {
        labels.push("STT");
    }
    if manifest.capabilities.voice_cloning {
        labels.push("CLONING");
    }
    if manifest.capabilities.voice_conversion {
        labels.push("CONVERSION");
    }
    if manifest.capabilities.voice_training {
        labels.push("TRAINING");
    }
    if !labels.is_empty() {
        return labels.join("/");
    }

    match manifest.kind {
        ModelKind::Tts => "TTS",
        ModelKind::Stt => "STT",
        ModelKind::VoiceClone | ModelKind::VoiceCloning => "CLONING",
        ModelKind::VoiceTrain => "TRAINING",
        ModelKind::VoiceConvert => "CONVERSION",
        ModelKind::OmniAudio => "OMNI",
    }
    .to_string()
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

    total.saturating_add(prefetched_runtime_size(record))
}

fn prefetched_runtime_size(record: &InstalledModelRecord) -> u64 {
    let Some(storage_root) = record.manifest_path.ancestors().nth(3) else {
        return 0;
    };
    let marker = storage_root
        .join("models")
        .join(&record.id)
        .join(".takokit-prefetch.json");
    std::fs::read(marker)
        .ok()
        .and_then(|source| serde_json::from_slice::<serde_json::Value>(&source).ok())
        .and_then(|marker| {
            marker
                .get("size_bytes")
                .and_then(serde_json::Value::as_u64)
        })
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_backed_model_uses_prefetch_marker_size() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let root = temporary.path();
        let manifest_path = root
            .join("manifests")
            .join("models")
            .join("bark-small.toml");
        std::fs::create_dir_all(manifest_path.parent().expect("manifest parent"))
            .expect("manifest directory");
        std::fs::write(&manifest_path, "").expect("manifest fixture");

        let marker = root
            .join("models")
            .join("bark-small")
            .join(".takokit-prefetch.json");
        std::fs::create_dir_all(marker.parent().expect("marker parent"))
            .expect("marker directory");
        std::fs::write(
            marker,
            serde_json::to_vec(&serde_json::json!({
                "model_id": "bark-small",
                "model_version": "0.1.0",
                "adapter": "hf_audio",
                "adapter_script_sha256": "fixture",
                "size_bytes": 1_234_567_u64
            }))
            .expect("marker JSON"),
        )
        .expect("marker fixture");

        let record = InstalledModelRecord {
            id: "bark-small".to_string(),
            version: "0.1.0".to_string(),
            source: "bundled".to_string(),
            manifest_path,
            runner: "takokit-python-managed".to_string(),
            installed_at: "0".to_string(),
            artifacts: Vec::new(),
            snapshot: None,
            status: InstalledPackageStatus::Ready,
            note: "ready".to_string(),
        };

        assert_eq!(installed_size(&record), 1_234_567);
    }
}
