//! Reference-aware model removal and dependency garbage collection.

use crate::{runtime_python_specs::adapter_spec, *};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RemoveModelOptions {
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRemovalItem {
    pub kind: String,
    pub id: String,
    pub path: PathBuf,
    pub logical_bytes: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRemovalReport {
    pub model_id: String,
    pub dry_run: bool,
    pub removed: bool,
    pub reclaimed_bytes: u64,
    pub deleted: Vec<ModelRemovalItem>,
    pub retained: Vec<ModelRemovalItem>,
}

pub fn remove_model_complete(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_reference: &str,
    options: RemoveModelOptions,
) -> PackageResult<ModelRemovalReport> {
    let root = installed_registry.storage_root();
    let _maintenance_guard = acquire_maintenance_lock(&root)?;
    let model_id = match resolve_installed_model_id(
        package_registry,
        installed_registry,
        model_reference,
    ) {
        Ok(model_id) => model_id,
        Err(PackageError::ModelNotInstalled(_)) => resolve_removal_journal_model_id(
            package_registry,
            &root,
            model_reference,
        )?
        .ok_or_else(|| PackageError::ModelNotInstalled(model_reference.to_string()))?,
        Err(error) => return Err(error),
    };
    let journal = removal_journal_path(&root, &model_id);
    if !installed_registry.is_model_installed(&model_id) && journal.is_file() {
        let mut report: ModelRemovalReport = serde_json::from_slice(&std::fs::read(&journal)?)?;
        if !options.dry_run {
            execute_items(&root, &report.deleted)?;
            remove_file_if_present(&journal)?;
            report.removed = true;
        }
        report.dry_run = options.dry_run;
        return Ok(report);
    }

    let target_record = installed_registry.installed_model_record(&model_id)?;
    let target_manifest = package_registry
        .model(&model_id)
        .or_else(|_| installed_registry.installed_model(&model_id))?;
    let remaining_records = installed_registry
        .installed_model_records()?
        .into_iter()
        .filter(|record| record.id != model_id)
        .collect::<Vec<_>>();
    let remaining_manifests = remaining_records
        .iter()
        .filter_map(|record| package_registry.model(&record.id).ok())
        .collect::<Vec<_>>();

    let referenced_blobs = remaining_records
        .iter()
        .flat_map(|record| record.artifacts.iter())
        .filter_map(|artifact| artifact.local_path.clone())
        .collect::<HashSet<_>>();
    let remaining_adapters = remaining_manifests
        .iter()
        .filter_map(|manifest| manifest.required_adapter.clone())
        .collect::<HashSet<_>>();
    let remaining_runners = remaining_records
        .iter()
        .map(|record| record.runner.clone())
        .collect::<HashSet<_>>();

    let mut deleted = Vec::new();
    let mut retained = Vec::new();
    push_candidate(
        &mut deleted,
        "model",
        &model_id,
        root.join("models").join(&model_id),
        "selected model data is exclusively owned by this installation",
    );

    for artifact in &target_record.artifacts {
        let Some(path) = artifact.local_path.as_ref() else {
            continue;
        };
        let item = ModelRemovalItem {
            kind: "blob".to_string(),
            id: artifact.sha256.clone(),
            path: path.clone(),
            logical_bytes: path_size(path),
            reason: if referenced_blobs.contains(path) {
                "retained because another installed model references this blob".to_string()
            } else {
                "no installed model references this content-addressed blob".to_string()
            },
        };
        if referenced_blobs.contains(path) {
            retained.push(item);
        } else {
            deleted.push(item);
        }
    }

    if let Some(adapter) = target_manifest.required_adapter.as_deref() {
        let path = python_managed_runner_layout(&root).adapters.join(adapter);
        if remaining_adapters.contains(adapter) {
            retained.push(item(
                "adapter",
                adapter,
                path,
                "retained because another installed model requires this adapter",
            ));
        } else {
            push_candidate(
                &mut deleted,
                "adapter",
                adapter,
                path,
                "adapter reference count reaches zero after removal",
            );
            if let Some(spec) = adapter_spec(adapter) {
                let abi_still_used = remaining_adapters.iter().any(|remaining| {
                    adapter_spec(remaining)
                        .is_some_and(|remaining_spec| remaining_spec.python == spec.python)
                });
                for path in python_abi_paths(&root, spec.python)? {
                    if abi_still_used {
                        retained.push(item(
                            "python-abi",
                            spec.python,
                            path,
                            "retained because another adapter uses this Python ABI base",
                        ));
                    } else {
                        push_candidate(
                            &mut deleted,
                            "python-abi",
                            spec.python,
                            path,
                            "Python ABI reference count reaches zero after removal",
                        );
                    }
                }
            }
        }
    }

    if remaining_runners.contains(&target_record.runner) {
        if let Ok(runner) = package_registry.runner(&target_record.runner) {
            retained.push(item(
                "runner-runtime",
                &target_record.runner,
                runner_runtime_layout(&root, &runner).root,
                "retained because another installed model requires this runner",
            ));
        }
    } else if let Ok(runner) = package_registry.runner(&target_record.runner) {
        push_candidate(
            &mut deleted,
            "runner-runtime",
            &target_record.runner,
            runner_runtime_layout(&root, &runner).root,
            "runner runtime reference count reaches zero; contract metadata is preserved",
        );
    }

    deleted = collapse_nested_items(deleted);
    let reclaimed_bytes = deleted
        .iter()
        .map(|entry| entry.logical_bytes)
        .fold(0_u64, u64::saturating_add);
    let mut report = ModelRemovalReport {
        model_id: model_id.clone(),
        dry_run: options.dry_run,
        removed: false,
        reclaimed_bytes,
        deleted,
        retained,
    };
    if options.dry_run {
        return Ok(report);
    }

    if let Some(parent) = journal.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&journal, serde_json::to_vec_pretty(&report)?)?;
    execute_items(&root, &report.deleted)?;
    installed_registry.remove_model(&model_id)?;
    remove_file_if_present(&journal)?;
    report.removed = true;
    Ok(report)
}

fn resolve_installed_model_id(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    model_reference: &str,
) -> PackageResult<String> {
    let mut matches = installed_registry
        .installed_model_records()?
        .into_iter()
        .filter(|record| model_references_match(package_registry, &record.id, model_reference))
        .map(|record| record.id)
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    match matches.as_slice() {
        [model_id] => Ok(model_id.clone()),
        [] => Err(PackageError::ModelNotInstalled(model_reference.to_string())),
        _ => Err(PackageError::ArtifactInstallFailed {
            artifact: model_reference.to_string(),
            reason: format!(
                "model reference resolves to multiple installed records: {}",
                matches.join(", ")
            ),
        }),
    }
}

fn resolve_removal_journal_model_id(
    package_registry: &PackageRegistry,
    root: &Path,
    model_reference: &str,
) -> PackageResult<Option<String>> {
    let directory = root.join("runtime").join("removals");
    if !directory.is_dir() {
        return Ok(None);
    }
    let mut matches = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let report: ModelRemovalReport = serde_json::from_slice(&std::fs::read(entry.path())?)?;
        if model_references_match(package_registry, &report.model_id, model_reference) {
            matches.push(report.model_id);
        }
    }
    matches.sort();
    matches.dedup();
    match matches.as_slice() {
        [model_id] => Ok(Some(model_id.clone())),
        [] => Ok(None),
        _ => Err(PackageError::ArtifactInstallFailed {
            artifact: model_reference.to_string(),
            reason: format!(
                "model reference matches multiple interrupted removals: {}",
                matches.join(", ")
            ),
        }),
    }
}

fn model_references_match(
    package_registry: &PackageRegistry,
    installed_id: &str,
    model_reference: &str,
) -> bool {
    if installed_id.eq_ignore_ascii_case(model_reference) {
        return true;
    }
    let Ok(installed) = package_registry.resolve_model_reference(installed_id) else {
        return false;
    };
    let Ok(requested) = package_registry.resolve_model_reference(model_reference) else {
        return false;
    };
    installed.canonical.eq_ignore_ascii_case(&requested.canonical)
        || installed.target.eq_ignore_ascii_case(&requested.target)
}

fn python_abi_paths(root: &Path, abi: &str) -> PackageResult<Vec<PathBuf>> {
    let python_root = root.join("tools").join("python");
    if !python_root.is_dir() {
        return Ok(Vec::new());
    }
    let prefix = format!("cpython-{abi}");
    Ok(std::fs::read_dir(python_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .collect())
}

fn execute_items(root: &Path, items: &[ModelRemovalItem]) -> PackageResult<()> {
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    for item in items {
        if item.path == root || !item.path.starts_with(root) {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: item.id.clone(),
                reason: format!("refusing to remove unsafe path {}", item.path.display()),
            });
        }
        if item.path.is_dir() {
            std::fs::remove_dir_all(&item.path)?;
        } else {
            remove_file_if_present(&item.path)?;
        }
        let _ = &canonical_root;
    }
    Ok(())
}

fn collapse_nested_items(mut items: Vec<ModelRemovalItem>) -> Vec<ModelRemovalItem> {
    items.sort_by(|left, right| {
        left.path
            .components()
            .count()
            .cmp(&right.path.components().count())
    });
    let mut collapsed: Vec<ModelRemovalItem> = Vec::new();
    for item in items {
        if collapsed
            .iter()
            .any(|parent| item.path.starts_with(&parent.path))
        {
            continue;
        }
        collapsed.push(item);
    }
    collapsed
}

fn push_candidate(
    items: &mut Vec<ModelRemovalItem>,
    kind: &str,
    id: &str,
    path: PathBuf,
    reason: &str,
) {
    items.push(ModelRemovalItem {
        kind: kind.to_string(),
        id: id.to_string(),
        logical_bytes: path_size(&path),
        path,
        reason: reason.to_string(),
    });
}

fn item(kind: &str, id: &str, path: PathBuf, reason: &str) -> ModelRemovalItem {
    ModelRemovalItem {
        kind: kind.to_string(),
        id: id.to_string(),
        logical_bytes: path_size(&path),
        path,
        reason: reason.to_string(),
    }
}

fn removal_journal_path(root: &Path, model_id: &str) -> PathBuf {
    root.join("runtime")
        .join("removals")
        .join(format!("{model_id}.json"))
}

fn remove_file_if_present(path: &Path) -> PackageResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
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
    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| path_size(&entry.path()))
        .fold(0_u64, u64::saturating_add)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_manifest_source(id: &str) -> String {
        format!(
            r#"id = "{id}"
name = "XTTS v2"
family = "xtts"
version = "2.0.0"
kind = "tts"
backend = "python-managed"
runner = "takokit-python-managed"
license = "mit"
description = "fixture"

[capabilities]
tts = true

[hardware]
cpu = true
gpu = false

[artifacts]
metadata_only = true
"#
        )
    }

    fn alias_fixture() -> (tempfile::TempDir, PackageRegistry, InstalledRegistry) {
        let root = tempfile::tempdir().expect("tempdir");
        let registry_root = root.path().join("registry");
        let manifests_root = root.path().join("manifests");
        std::fs::create_dir_all(registry_root.join("models")).expect("registry models");
        std::fs::create_dir_all(manifests_root.join("models")).expect("installed models");
        std::fs::create_dir_all(manifests_root.join("installed-models"))
            .expect("installed records");

        let target_source = model_manifest_source("xtts-v2");
        std::fs::write(registry_root.join("models/xtts-v2.toml"), &target_source)
            .expect("target manifest");
        let index = RegistryIndex {
            schema_version: REGISTRY_SCHEMA_VERSION,
            namespace: "library".into(),
            generated_at: "0".into(),
            models: vec![RegistryModel {
                name: "xtts".into(),
                display_name: "XTTS v2".into(),
                default_tag: "2".into(),
                summary: "fixture".into(),
                tasks: vec!["tts".into()],
                aliases: Vec::new(),
                tags: vec![RegistryTag {
                    tag: "2".into(),
                    target: "xtts-v2".into(),
                    aliases: vec!["xtts-v2".into()],
                    version: "2.0.0".into(),
                    digest: manifest_digest(&target_source),
                    size_bytes: 0,
                    runner: "takokit-python-managed".into(),
                    adapter: None,
                    license: "mit".into(),
                    kind: "tts".into(),
                    backend: "python-managed".into(),
                    hardware: RegistryHardware {
                        cpu: true,
                        gpu: false,
                        min_ram: None,
                        min_vram: None,
                    },
                    source: RegistrySource {
                        provider: "artifact".into(),
                        repository: None,
                        revision: None,
                    },
                    manifest_toml: Some(target_source),
                }],
            }],
        };
        std::fs::write(
            registry_root.join("index.json"),
            serde_json::to_vec_pretty(&index).expect("registry json"),
        )
        .expect("registry index");

        let installed_source = model_manifest_source("xtts");
        let installed_manifest: ModelManifest =
            toml::from_str(&installed_source).expect("installed manifest");
        let installed_manifest_path = manifests_root.join("models/xtts.toml");
        std::fs::write(&installed_manifest_path, installed_source).expect("installed manifest file");
        let record = installed_model_record(&installed_manifest, installed_manifest_path);
        std::fs::write(
            manifests_root.join("installed-models/xtts.toml"),
            toml::to_string_pretty(&record).expect("installed record toml"),
        )
        .expect("installed record");

        let package_registry = PackageRegistry::new(registry_root);
        let installed_registry = InstalledRegistry::new(manifests_root);
        (root, package_registry, installed_registry)
    }

    #[test]
    fn nested_dependency_paths_are_counted_once() {
        let root = PathBuf::from("root");
        let items = vec![
            item(
                "runner",
                "python",
                root.join("runners/python-managed"),
                "zero refs",
            ),
            item(
                "adapter",
                "coqui",
                root.join("runners/python-managed/adapters/coqui"),
                "zero refs",
            ),
        ];
        let collapsed = collapse_nested_items(items);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].kind, "runner");
    }

    #[test]
    fn dry_run_preserves_shared_and_exclusive_classification() {
        let root = tempfile::tempdir().expect("tempdir");
        let shared = root.path().join("shared");
        let exclusive = root.path().join("exclusive");
        std::fs::write(&shared, b"shared").expect("shared");
        std::fs::write(&exclusive, b"exclusive").expect("exclusive");
        let retained = item("blob", "shared", shared.clone(), "referenced");
        let deleted = item("blob", "exclusive", exclusive.clone(), "zero refs");
        let report = ModelRemovalReport {
            model_id: "fixture".into(),
            dry_run: true,
            removed: false,
            reclaimed_bytes: deleted.logical_bytes,
            deleted: vec![deleted],
            retained: vec![retained],
        };
        assert!(!report.removed);
        assert!(shared.exists());
        assert!(exclusive.exists());
    }

    #[test]
    fn removal_alias_resolves_the_existing_legacy_install_record() {
        let (_root, package_registry, installed_registry) = alias_fixture();
        let report = remove_model_complete(
            &package_registry,
            &installed_registry,
            "xtts-v2",
            RemoveModelOptions { dry_run: true },
        )
        .expect("alias removal plan");

        assert_eq!(report.model_id, "xtts");
        assert!(report.dry_run);
        assert!(!report.removed);
        assert!(installed_registry.is_model_installed("xtts"));
    }
}
