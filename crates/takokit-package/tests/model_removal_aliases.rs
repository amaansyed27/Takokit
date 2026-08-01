use takokit_package::{
    manifest_digest, remove_model_complete, InstalledModelRecord, InstalledPackageStatus,
    InstalledRegistry, ModelManifest, PackageRegistry, RegistryHardware, RegistryIndex,
    RegistryModel, RegistrySource, RegistryTag, RemoveModelOptions, REGISTRY_SCHEMA_VERSION,
};

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

#[test]
fn removal_alias_resolves_the_existing_legacy_install_record() {
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
    let record = InstalledModelRecord {
        id: installed_manifest.id,
        version: installed_manifest.version,
        source: "fixture".into(),
        manifest_path: installed_manifest_path,
        runner: installed_manifest.runner,
        installed_at: "0".into(),
        artifacts: Vec::new(),
        snapshot: None,
        status: InstalledPackageStatus::MetadataOnly,
        note: "fixture".into(),
    };
    std::fs::write(
        manifests_root.join("installed-models/xtts.toml"),
        toml::to_string_pretty(&record).expect("installed record toml"),
    )
    .expect("installed record");

    let package_registry = PackageRegistry::new(registry_root);
    let installed_registry = InstalledRegistry::new(manifests_root);
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
