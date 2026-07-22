use super::*;

#[test]
fn planner_rechecks_local_artifact_bytes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry_root = temp.path().join("registry");
    let models = registry_root.join("models");
    let runners = registry_root.join("runners");
    std::fs::create_dir_all(&models).unwrap();
    std::fs::create_dir_all(&runners).unwrap();

    let source = temp.path().join("fixture.onnx");
    std::fs::write(&source, b"hello").unwrap();
    let manifest = artifact_test_manifest(&source, HELLO_SHA256);
    std::fs::write(
        models.join("piper-lessac.toml"),
        toml::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(runners.join("takokit-onnx.toml"), RUNNER_TOML).unwrap();

    let registry = PackageRegistry::new(&registry_root);
    let installed = InstalledRegistry::new(temp.path().join("manifests"));
    installed.install_model(&manifest).unwrap();
    let blob = installed
        .installed_model_record("piper-lessac")
        .unwrap()
        .artifacts[0]
        .local_path
        .clone()
        .unwrap();
    std::fs::remove_file(blob).unwrap();

    let plan = plan_model(&registry, &installed, "piper-lessac").unwrap();
    assert_eq!(plan.artifact_state, ModelLifecycleState::MetadataOnly);
    assert!(!plan.executable);
    assert!(plan.missing.iter().any(|item| item == "verified artifacts"));
}

#[test]
fn runtime_managed_model_requires_verified_prefetch_marker() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry = PackageRegistry::bundled();
    let manifest = registry.model("bark-small").expect("runtime-managed model");
    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    installed.install_model(&manifest).expect("install model");
    let record = installed
        .installed_model_record("bark-small")
        .expect("installed record");

    assert_eq!(record.status, InstalledPackageStatus::MetadataOnly);
    assert!(record.artifacts.is_empty());
    assert!(!crate::artifact_reuse::all_verified(&record, &manifest));

    let model_dir = temp.path().join("models").join("bark-small");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(
        model_dir.join(".takokit-prefetch.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "model_id": manifest.id,
            "model_version": manifest.version,
            "adapter": manifest.runner.required_adapter,
        }))
        .unwrap(),
    )
    .unwrap();
    installed
        .mark_runtime_model_ready("bark-small", "verified checkpoint")
        .unwrap();

    let record = installed
        .installed_model_record("bark-small")
        .expect("ready record");
    assert_eq!(record.status, InstalledPackageStatus::Ready);
    assert!(crate::artifact_reuse::all_verified(&record, &manifest));
}

#[test]
fn metadata_only_pull_preserves_verified_ready_install() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("fixture.onnx");
    std::fs::write(&source, b"hello").unwrap();
    let manifest = artifact_test_manifest(&source, HELLO_SHA256);
    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    installed.install_model(&manifest).unwrap();
    let before = installed.installed_model_record("piper-lessac").unwrap();
    let report = installed
        .install_model_with_options(
            &manifest,
            InstallModelOptions {
                metadata_only: true,
            },
        )
        .unwrap();
    let after = installed.installed_model_record("piper-lessac").unwrap();

    assert!(!report.installed);
    assert_eq!(after, before);
    assert_eq!(after.status, InstalledPackageStatus::Ready);
}

#[test]
fn repeated_metadata_only_pull_is_idempotent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("fixture.onnx");
    std::fs::write(&source, b"hello").unwrap();
    let manifest = artifact_test_manifest(&source, HELLO_SHA256);
    let installed = InstalledRegistry::new(temp.path().join("manifests"));
    let options = InstallModelOptions {
        metadata_only: true,
    };

    let first = installed
        .install_model_with_options(&manifest, options)
        .unwrap();
    let second = installed
        .install_model_with_options(&manifest, options)
        .unwrap();

    assert!(first.installed);
    assert!(!second.installed);
    assert_eq!(
        installed
            .installed_model_record("piper-lessac")
            .unwrap()
            .status,
        InstalledPackageStatus::MetadataOnly
    );
}

#[test]
fn corrupt_ready_artifact_is_classified_for_repair() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("fixture.onnx");
    std::fs::write(&source, b"hello").unwrap();
    let manifest = artifact_test_manifest(&source, HELLO_SHA256);
    let installed = InstalledRegistry::new(temp.path().join("manifests"));
    installed.install_model(&manifest).unwrap();

    let record = installed.installed_model_record("piper-lessac").unwrap();
    std::fs::write(record.artifacts[0].local_path.as_ref().unwrap(), b"broken").unwrap();
    let record = installed.installed_model_record("piper-lessac").unwrap();

    assert_eq!(
        crate::artifact_reuse::classify(Some(&record), &manifest),
        crate::artifact_reuse::ArtifactReuseState::RepairRequired
    );
}

#[test]
fn unsupported_executor_fails_before_reading_artifact_source() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry_root = temp.path().join("registry");
    let models = registry_root.join("models");
    let runners = registry_root.join("runners");
    std::fs::create_dir_all(&models).unwrap();
    std::fs::create_dir_all(&runners).unwrap();

    let missing_source = temp.path().join("does-not-exist.onnx");
    let manifest = artifact_test_manifest(&missing_source, HELLO_SHA256);
    std::fs::write(
        models.join("piper-lessac.toml"),
        toml::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(runners.join("takokit-onnx.toml"), RUNNER_TOML).unwrap();

    let registry = PackageRegistry::new(&registry_root);
    let installed = InstalledRegistry::new(temp.path().join("manifests"));
    let runner = registry.runner("takokit-onnx").unwrap();
    installed.install_runner(&runner).unwrap();
    installed
        .install_runner_runtime(&runner, RunnerLifecycleState::Ready, "fixture runtime")
        .unwrap();

    let error = install_model_complete(
        &registry,
        &installed,
        temp.path(),
        "piper-lessac",
        InstallModelOptions::default(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        PackageError::InstallStage {
            stage: InstallFailureStage::FinalVerification,
            ..
        }
    ));
    assert!(!installed.is_model_installed("piper-lessac"));
}
