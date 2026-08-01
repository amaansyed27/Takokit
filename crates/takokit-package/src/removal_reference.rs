//! Resolve user-facing model references to exact installed removal identities.

use crate::{InstalledRegistry, PackageError, PackageRegistry, PackageResult};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RemovalJournalIdentity {
    model_id: String,
}

pub(crate) fn resolve_model_removal_id(
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
        .filter(|candidate| model_id_matches(package_registry, candidate, &target, &canonical))
        .collect::<Vec<_>>();

    let journal_dir = installed_registry
        .storage_root()
        .join("runtime")
        .join("removals");
    if journal_dir.is_dir() {
        let mut entries =
            std::fs::read_dir(&journal_dir)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            let Ok(journal) = serde_json::from_slice::<RemovalJournalIdentity>(&bytes) else {
                continue;
            };
            if model_id_matches(package_registry, &journal.model_id, &target, &canonical) {
                matches.push(journal.model_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InstallModelOptions;
    use std::path::PathBuf;

    fn bundled_registry() -> PackageRegistry {
        PackageRegistry::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../registry"))
    }

    #[test]
    fn resolves_a_legacy_installed_alias_from_target_or_canonical_reference() {
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
            let model_id = resolve_model_removal_id(&registry, &installed, reference)
                .expect("alias-aware resolution");
            assert_eq!(model_id, "xtts");
        }
    }

    #[test]
    fn resolves_an_alias_named_interrupted_journal() {
        let root = tempfile::tempdir().expect("tempdir");
        let registry = bundled_registry();
        let installed = InstalledRegistry::new(root.path().join("manifests"));
        let journal = root.path().join("runtime/removals/xtts.json");
        std::fs::create_dir_all(journal.parent().expect("journal parent"))
            .expect("journal directory");
        std::fs::write(&journal, b"{\"model_id\":\"xtts\"}").expect("journal write");

        let model_id = resolve_model_removal_id(&registry, &installed, "xtts-v2")
            .expect("journal resolution");
        assert_eq!(model_id, "xtts");
    }
}
