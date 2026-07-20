use super::*;

#[test]

fn registry_finds_model_manifest_by_id() {
    let temp = tempfile::tempdir().expect("tempdir");

    let models = temp.path().join("models");

    std::fs::create_dir_all(&models).expect("models dir");

    std::fs::write(models.join("kokoro.toml"), MODEL_TOML).expect("model toml");

    let registry = PackageRegistry::new(temp.path());

    let manifest = registry.model("kokoro").expect("model lookup");

    assert_eq!(manifest.name, "Kokoro");
}

#[test]

fn installed_registry_reports_not_installed() {
    let temp = tempfile::tempdir().expect("tempdir");

    let registry = InstalledRegistry::new(temp.path());

    let error = registry
        .installed_model("kokoro")
        .expect_err("not installed");

    assert!(matches!(error, PackageError::ModelNotInstalled(id) if id == "kokoro"));
}

#[test]

fn resolver_rejects_unsupported_capability() {
    let temp = tempfile::tempdir().expect("tempdir");

    write_test_registry(temp.path());

    let registry = PackageRegistry::new(temp.path());

    let installed = InstalledRegistry::new(temp.path().join("installed"));

    let manifest = registry.model("kokoro").expect("model");

    installed.install_model(&manifest).expect("install model");

    let error = resolve_execution_plan(
        &registry,
        &installed,
        "kokoro",
        CapabilityKind::SpeechToText,
    )
    .expect_err("unsupported capability");

    assert!(matches!(

        error,

        PackageError::CapabilityUnsupported { model, capability, .. }

            if model == "kokoro" && capability == CapabilityKind::SpeechToText

    ));
}

#[test]

fn resolver_reports_model_not_installed_before_unsupported_capability() {
    let temp = tempfile::tempdir().expect("tempdir");

    write_test_registry(temp.path());

    let registry = PackageRegistry::new(temp.path());

    let installed = InstalledRegistry::new(temp.path().join("installed"));

    let error = resolve_execution_plan(
        &registry,
        &installed,
        "kokoro",
        CapabilityKind::SpeechToText,
    )
    .expect_err("model not installed");

    assert!(matches!(error, PackageError::ModelNotInstalled(id) if id == "kokoro"));
}

#[test]

fn install_model_writes_installed_record() {
    let temp = tempfile::tempdir().expect("tempdir");

    write_test_registry(temp.path());

    let registry = PackageRegistry::new(temp.path());

    let installed = InstalledRegistry::new(temp.path().join("installed"));

    let manifest = registry.model("kokoro").expect("model");

    let report = installed.install_model(&manifest).expect("install model");

    let record = installed
        .installed_model_record("kokoro")
        .expect("installed model record");

    assert_eq!(report.id, "kokoro");

    assert_eq!(record.id, "kokoro");

    assert_eq!(record.version, "0.1.0");

    assert_eq!(record.runner, "takokit-onnx");

    assert_eq!(record.source, "takokit-registry");

    assert_eq!(record.status, InstalledPackageStatus::MetadataOnly);

    assert!(record.manifest_path.ends_with("models/kokoro.toml"));
}

#[test]

fn install_model_missing_artifact_checksum_returns_typed_error() {
    let temp = tempfile::tempdir().expect("tempdir");

    let source = temp.path().join("fixture.onnx");

    std::fs::write(&source, b"hello").expect("fixture");

    let manifest = artifact_test_manifest(&source, "");

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    let error = installed
        .install_model_with_options(&manifest, InstallModelOptions::default())
        .expect_err("missing checksum");

    assert!(matches!(

        error,

        PackageError::ArtifactChecksumMissing { model, artifact }

            if model == "piper-lessac" && artifact == "fixture.onnx"

    ));
}

#[test]

fn checksum_mismatch_deletes_temporary_download() {
    let temp = tempfile::tempdir().expect("tempdir");

    let source = temp.path().join("fixture.onnx");

    std::fs::write(&source, b"hello").expect("fixture");

    let manifest = artifact_test_manifest(&source, "0000");

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    let error = installed
        .install_model_with_options(&manifest, InstallModelOptions::default())
        .expect_err("checksum mismatch");

    assert!(matches!(

        error,

        PackageError::ArtifactChecksumMismatch { artifact, .. } if artifact == "fixture.onnx"

    ));

    let downloads = temp.path().join("cache").join("downloads");

    let leftovers = std::fs::read_dir(downloads)
        .map(|entries| entries.count())
        .unwrap_or(0);

    assert_eq!(leftovers, 0);
}

#[test]

fn checksum_mismatch_does_not_leave_installed_model_state() {
    let temp = tempfile::tempdir().expect("tempdir");

    let source = temp.path().join("fixture.onnx");

    std::fs::write(&source, b"hello").expect("fixture");

    let manifest = artifact_test_manifest(&source, "0000");

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    let error = installed
        .install_model_with_options(&manifest, InstallModelOptions::default())
        .expect_err("checksum mismatch");

    assert!(matches!(

        error,

        PackageError::ArtifactChecksumMismatch { artifact, .. } if artifact == "fixture.onnx"

    ));

    assert!(!installed.model_manifest_path("piper-lessac").exists());

    assert!(!installed.model_record_path("piper-lessac").exists());

    assert!(!installed.is_model_installed("piper-lessac"));
}

#[test]

fn successful_local_artifact_install_writes_downloaded_record() {
    let temp = tempfile::tempdir().expect("tempdir");

    let source = temp.path().join("fixture.onnx");

    std::fs::write(&source, b"hello").expect("fixture");

    let manifest = artifact_test_manifest(&source, HELLO_SHA256);

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    installed
        .install_model_with_options(&manifest, InstallModelOptions::default())
        .expect("install model");

    assert!(installed.model_manifest_path("piper-lessac").exists());

    assert!(installed.model_record_path("piper-lessac").exists());

    assert!(installed.is_model_installed("piper-lessac"));

    let record = installed
        .installed_model_record("piper-lessac")
        .expect("installed record");

    assert_eq!(record.status, InstalledPackageStatus::Ready);

    assert_eq!(record.artifacts.len(), 1);

    assert_eq!(record.artifacts[0].role, ArtifactRole::Model);

    assert!(record.artifacts[0].downloaded);

    let local_path = record.artifacts[0].local_path.as_ref().expect("local path");

    assert!(local_path.ends_with(Path::new("blobs").join("sha256").join(HELLO_SHA256)));

    assert_eq!(std::fs::read(local_path).expect("blob"), b"hello");
}

#[test]

fn second_local_pull_reuses_verified_artifact_without_reading_source() {
    let temp = tempfile::tempdir().expect("tempdir");

    let source = temp.path().join("fixture.onnx");

    std::fs::write(&source, b"hello").expect("fixture");

    let manifest = artifact_test_manifest(&source, HELLO_SHA256);

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    installed.install_model(&manifest).expect("first install");

    std::fs::remove_file(&source).expect("make fixture unavailable");

    let report = installed.install_model(&manifest).expect("verified reuse");

    assert!(report.installed);

    assert_eq!(
        installed
            .installed_model_record("piper-lessac")
            .expect("record")
            .status,
        InstalledPackageStatus::Ready
    );
}

#[test]

fn corrupt_artifact_repairs_only_corrupt_entry() {
    let temp = tempfile::tempdir().expect("tempdir");

    let model_source = temp.path().join("fixture.onnx");

    let config_source = temp.path().join("fixture.onnx.json");

    std::fs::write(&model_source, b"hello").expect("model fixture");

    std::fs::write(&config_source, br#"{"audio":{"sample_rate":22050}}"#).expect("config fixture");

    let manifest = multi_artifact_test_manifest(
        &model_source,
        &sha256_file(&model_source).expect("model sha"),
        &config_source,
        &sha256_file(&config_source).expect("config sha"),
    );

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    installed.install_model(&manifest).expect("first install");

    let record = installed
        .installed_model_record("piper-lessac")
        .expect("record");

    let corrupt = record
        .artifacts
        .iter()
        .find(|item| item.name == "fixture.onnx.json")
        .unwrap()
        .local_path
        .clone()
        .unwrap();

    std::fs::write(&corrupt, b"corrupt").expect("corrupt blob");

    std::fs::remove_file(&model_source).expect("valid source must not be read");

    installed
        .install_model(&manifest)
        .expect("repair corrupt only");

    assert_eq!(
        sha256_file(&corrupt).expect("repaired checksum"),
        manifest.artifacts.configs[0].sha256
    );
}

#[test]

fn successful_local_artifact_install_writes_model_and_config_records() {
    let temp = tempfile::tempdir().expect("tempdir");

    let model_source = temp.path().join("fixture.onnx");

    let config_source = temp.path().join("fixture.onnx.json");

    std::fs::write(&model_source, b"hello").expect("model fixture");

    std::fs::write(&config_source, br#"{"audio":{"sample_rate":22050}}"#).expect("config fixture");

    let model_sha = sha256_file(&model_source).expect("model sha");

    let config_sha = sha256_file(&config_source).expect("config sha");

    let manifest =
        multi_artifact_test_manifest(&model_source, &model_sha, &config_source, &config_sha);

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    installed
        .install_model_with_options(&manifest, InstallModelOptions::default())
        .expect("install model");

    let record = installed
        .installed_model_record("piper-lessac")
        .expect("installed record");

    assert_eq!(record.status, InstalledPackageStatus::Ready);

    assert_eq!(record.artifacts.len(), 2);

    assert!(record.artifacts.iter().all(|artifact| artifact.downloaded));

    assert!(record
        .artifacts
        .iter()
        .any(|artifact| artifact.name == "fixture.onnx"
            && artifact.role == ArtifactRole::Model
            && artifact.local_path.as_ref().is_some_and(
                |path| path.ends_with(Path::new("blobs").join("sha256").join(&model_sha))
            )));

    assert!(record
        .artifacts
        .iter()
        .any(|artifact| artifact.name == "fixture.onnx.json"
            && artifact.role == ArtifactRole::Config
            && artifact.local_path.as_ref().is_some_and(
                |path| path.ends_with(Path::new("blobs").join("sha256").join(&config_sha))
            )));
}
