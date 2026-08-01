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
    model_id: &str,
    options: RemoveModelOptions,
) -> PackageResult<ModelRemovalReport> {
    let root = installed_registry.storage_root();
    let _maintenance_guard = acquire_maintenance_lock(&root)?;
    let resolved_model_id =
        resolve_model_removal_id(package_registry, installed_registry, model_id)?;
    let model_id = resolved_model_id.as_str();
    let journal = removal_journal_path(&root, model_id);
    if !installed_registry.is_model_installed(model_id) && journal.is_file() {
        let mut report: ModelRemovalReport = serde_json::from_slice(&std::fs::read(&journal)?)?;
        if !options.dry_run {
            execute_items(&root, &report.deleted)?;
            remove_file_if_present(&journal)?;
            report.removed = true;
        }
        report.dry_run = options.dry_run;
        return Ok(report);
    }

    let target_record = installed_registry.installed_model_record(model_id)?;
    let target_manifest = package_registry
        .model(model_id)
        .or_else(|_| installed_registry.installed_model(model_id))?;
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
        model_id,
        root.join("models").join(model_id),
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
        model_id: model_id.to_string(),
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
    installed_registry.remove_model(model_id)?;
    remove_file_if_present(&journal)?;
    report.removed = true;
    Ok(report)
}

fn resolve_model_removal_id(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    reference: &str,
) -> PackageResult<String> {
    if installed_registry.is_model_installed(reference) {
        return Ok(reference.to_string());
    }

    let resolved = package_registry.resolve_model_reference(reference).ok();
    let target = resolved
        .as_ref()
        .map(|resolved| resolved.target.clone())
        .unwrap_or_else(|| reference.to_string());
    let canonical = resolved
        .as_ref()
        .map(|resolved| resolved.canonical.clone())
        .unwrap_or_else(|| package_registry.canonical_reference_for_id(&target));
    let mut matches = installed_registry
        .installed_model_records()?
        .into_iter()
        .map(|record| record.id)
        .filter(|candidate| {
            model_id_matches(package_registry, candidate, &target, &canonical)
        })
        .collect::<Vec<_>>();

    let journal_dir = installed_registry
        .storage_root()
        .join("runtime")
        .join("removals");
    if journal_dir.is_dir() {
        let mut entries = std::fs::read_dir(&journal_dir)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            let Ok(report) = serde_json::from_slice::<ModelRemovalReport>(&bytes) else {
                continue;
            };
            if model_id_matches(package_registry, &report.model_id, &target, &canonical) {
                matches.push(report.model_id);
            }
        }
    }

    matches.sort();
    matches.dedup();
    match matches.as_slice() {
        [model_id] => Ok(model_id.clone()),
        [] => Err(PackageError::ModelNotInstalled(target)),
        _ => Err(PackageError::ArtifactInstallFailed {
            artifact: target,
            reason: format!(
                "multiple installed records match this model reference: {}; remove one exact install ID at a time",
                matches.join(", ")
            ),
        }),
    }
}

fn model_id_matches(
    package_registry: &PackageRegistry,
    candidate: &str,
    target: &str,
    canonical: &str,
) -> bool {
    candidate.eq_ignore_ascii_case(target)
        || package_registry
            .canonical_reference_for_id(candidate)
            .eq_ignore_ascii_case(canonical)
        || package_registry
            .resolve_model_reference(candidate)
            .is_ok_and(|resolved| resolved.target.eq_ignore_ascii_case(target))
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

    fn bundled_registry() -> PackageRegistry {
        PackageRegistry::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../registry"))
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
    fn removal_resolves_a_legacy_installed_alias_from_target_or_canonical_reference() {
        let root = tempfile::tempdir().expect("tempdir");
        let registry = bundled_registry();
        let installed = InstalledRegistry::new(root.path().join("manifests"));
        let mut legacy_manifest = registry.model("xtts-v2").expect("XTTS manifest");
        legacy_manifest.id = "xtts".to_string();
        installed
            .install_model_with_options(
                &legacy_manifest,
                InstallModelOptions {
                    metadata_only: true,
                    ..InstallModelOptions::default()
                },
            )
            .expect("legacy install record");

        for reference in ["xtts-v2", "xtts:2"] {
            let report = remove_model_complete(
                &registry,
                &installed,
                reference,
                RemoveModelOptions { dry_run: true },
            )
            .expect("alias-aware dry run");
            assert_eq!(report.model_id, "xtts");
            assert!(!report.removed);
        }
    }

    #[test]
    fn removal_recovers_an_alias_named_interrupted_journal() {
        let root = tempfile::tempdir().expect("tempdir");
        let registry = bundled_registry();
        let installed = InstalledRegistry::new(root.path().join("manifests"));
        let journal = removal_journal_path(root.path(), "xtts");
        std::fs::create_dir_all(journal.parent().expect("journal parent"))
            .expect("journal directory");
        let report = ModelRemovalReport {
            model_id: "xtts".to_string(),
            dry_run: false,
            removed: false,
            reclaimed_bytes: 0,
            deleted: Vec::new(),
            retained: Vec::new(),
        };
        std::fs::write(&journal, serde_json::to_vec_pretty(&report).expect("journal JSON"))
            .expect("journal write");

        let recovered = remove_model_complete(
            &registry,
            &installed,
            "xtts-v2",
            RemoveModelOptions { dry_run: true },
        )
        .expect("journal recovery");
        assert_eq!(recovered.model_id, "xtts");
        assert!(journal.is_file());
    }
}
