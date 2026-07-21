use super::*;

#[test]

fn bundled_piper_lessac_manifest_has_verified_artifact_fields() {
    let manifest = PackageRegistry::bundled()
        .model("piper-lessac")
        .expect("piper manifest");

    assert!(!manifest.artifacts.metadata_only);

    assert_eq!(manifest.artifacts.weights.len(), 1);

    assert_eq!(manifest.artifacts.configs.len(), 1);

    assert_eq!(

        manifest.artifacts.weights[0].url.as_deref(),

        Some("https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx")

    );

    assert_eq!(manifest.artifacts.weights[0].bytes, Some(63_201_294));

    assert_eq!(
        manifest.artifacts.weights[0].sha256,
        "5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f"
    );

    assert_eq!(

        manifest.artifacts.configs[0].url.as_deref(),

        Some("https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json")

    );

    assert_eq!(manifest.artifacts.configs[0].bytes, Some(4_885));

    assert_eq!(
        manifest.artifacts.configs[0].sha256,
        "efe19c417bed055f2d69908248c6ba650fa135bc868b0e6abb3da181dab690a0"
    );
}

#[test]

fn bundled_metadata_only_models_install_without_artifact_downloads() {
    let temp = tempfile::tempdir().expect("tempdir");

    let registry = PackageRegistry::bundled();

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    for model_id in ["chatterbox", "gpt-sovits"] {
        let manifest = registry.model(model_id).expect("model manifest");

        installed
            .install_model_with_options(
                &manifest,
                InstallModelOptions {
                    metadata_only: true,
                },
            )
            .expect("install metadata-only model");

        let record = installed
            .installed_model_record(model_id)
            .expect("installed record");

        assert_eq!(record.status, InstalledPackageStatus::MetadataOnly);

        assert!(record.artifacts.iter().all(|artifact| !artifact.downloaded));
    }
}

#[test]

fn bundled_whisper_base_manifest_has_verified_artifact_metadata() {
    let registry = PackageRegistry::bundled();

    let manifest = registry.model("whisper-base").expect("whisper manifest");

    assert!(!manifest.artifacts.metadata_only);

    assert_eq!(manifest.family, "whisper");

    assert_eq!(manifest.artifacts.weights.len(), 1);

    assert_eq!(manifest.artifacts.weights[0].name, "ggml-base.bin");

    assert_eq!(manifest.artifacts.weights[0].bytes, Some(147_951_465));

    assert_eq!(
        manifest.artifacts.weights[0].sha256,
        "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe"
    );
}

#[test]

fn bundled_qwen_manifest_pins_complete_local_runtime_artifacts() {
    let registry = PackageRegistry::bundled();

    let manifest = registry.model("qwen3-tts").expect("qwen manifest");

    assert!(!manifest.artifacts.metadata_only);

    assert_eq!(manifest.license, "apache-2.0");

    assert_eq!(manifest.artifacts.weights.len(), 2);

    assert!(manifest
        .artifacts
        .all()
        .any(|artifact| artifact.name == "speech_tokenizer/model.safetensors"));

    assert!(manifest
        .artifacts
        .all()
        .all(|artifact| !artifact.sha256.trim().is_empty() && artifact.bytes.is_some()));
}

#[test]

fn metadata_only_model_install_still_works_with_artifact_placeholders() {
    let temp = tempfile::tempdir().expect("tempdir");

    let source = temp.path().join("fixture.onnx");

    std::fs::write(&source, b"hello").expect("fixture");

    let manifest = artifact_test_manifest(&source, "");

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    installed
        .install_model_with_options(
            &manifest,
            InstallModelOptions {
                metadata_only: true,
            },
        )
        .expect("metadata install");

    assert!(installed.model_manifest_path("piper-lessac").exists());

    assert!(installed.model_record_path("piper-lessac").exists());

    assert!(installed.is_model_installed("piper-lessac"));

    let record = installed
        .installed_model_record("piper-lessac")
        .expect("installed record");

    assert_eq!(record.status, InstalledPackageStatus::MetadataOnly);

    assert_eq!(record.artifacts.len(), 1);

    assert!(!record.artifacts[0].downloaded);

    assert!(record.artifacts[0].local_path.is_none());
}

#[test]

fn install_runner_writes_installed_record() {
    let temp = tempfile::tempdir().expect("tempdir");

    write_test_registry(temp.path());

    let registry = PackageRegistry::new(temp.path());

    let installed = InstalledRegistry::new(temp.path().join("installed"));

    let manifest = registry.runner("takokit-onnx").expect("runner");

    let report = installed.install_runner(&manifest).expect("install runner");

    let record = installed
        .installed_runner_record("takokit-onnx")
        .expect("installed runner record");

    assert_eq!(report.id, "takokit-onnx");

    assert_eq!(record.id, "takokit-onnx");

    assert_eq!(record.version, "0.1.0");

    assert_eq!(record.kind, "onnx");

    assert_eq!(record.status, RunnerLifecycleState::ContractInstalled);

    assert!(record.manifest_path.ends_with("runners/takokit-onnx.toml"));
}

#[test]

fn install_runner_runtime_updates_runner_record_state_and_note() {
    let temp = tempfile::tempdir().expect("tempdir");

    write_test_registry(temp.path());

    let registry = PackageRegistry::new(temp.path());

    let installed = InstalledRegistry::new(temp.path().join("installed"));

    let manifest = registry.runner("takokit-onnx").expect("runner");

    installed.install_runner(&manifest).expect("install runner");

    let report = installed
        .install_runner_runtime(
            &manifest,
            RunnerLifecycleState::RuntimeInstalled,
            "ONNX runtime dependency path initialized.",
        )
        .expect("install runner runtime");

    let record = installed
        .installed_runner_record("takokit-onnx")
        .expect("installed runner record");

    assert_eq!(report.id, "takokit-onnx");

    assert_eq!(record.status, RunnerLifecycleState::RuntimeInstalled);

    assert!(record.note.contains("ONNX runtime dependency path"));
}

#[test]

fn pulling_runner_contract_does_not_downgrade_ready_runtime() {
    let temp = tempfile::tempdir().expect("tempdir");

    write_test_registry(temp.path());

    let registry = PackageRegistry::new(temp.path());

    let installed = InstalledRegistry::new(temp.path().join("installed"));

    let manifest = registry.runner("takokit-onnx").expect("runner");

    installed.install_runner(&manifest).expect("contract");

    installed
        .install_runner_runtime(&manifest, RunnerLifecycleState::Ready, "ready runtime")
        .expect("ready runtime");

    installed
        .install_runner(&manifest)
        .expect("refresh contract");

    let record = installed
        .installed_runner_record("takokit-onnx")
        .expect("runner record");

    assert_eq!(record.status, RunnerLifecycleState::Ready);

    assert_eq!(record.note, "ready runtime");
}

#[test]

fn package_registry_exposes_registry_root_for_health_checks() {
    let temp = tempfile::tempdir().expect("tempdir");

    let registry = PackageRegistry::new(temp.path());

    assert_eq!(registry.root(), temp.path());
}

#[test]

fn installed_registry_lists_installed_model_and_runner_records() {
    let temp = tempfile::tempdir().expect("tempdir");

    write_test_registry(temp.path());

    let registry = PackageRegistry::new(temp.path());

    let installed = InstalledRegistry::new(temp.path().join("installed"));

    let model = registry.model("kokoro").expect("model");

    let runner = registry.runner("takokit-onnx").expect("runner");

    installed.install_model(&model).expect("install model");

    installed.install_runner(&runner).expect("install runner");

    let models = installed.installed_model_records().expect("model records");

    let runners = installed
        .installed_runner_records()
        .expect("runner records");

    assert_eq!(models.len(), 1);

    assert_eq!(models[0].id, "kokoro");

    assert_eq!(runners.len(), 1);

    assert_eq!(runners[0].id, "takokit-onnx");
}

#[test]
fn installed_inventory_contains_only_verified_ready_models() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("fixture.onnx");
    std::fs::write(&source, b"hello").expect("fixture");
    let manifest = artifact_test_manifest(&source, HELLO_SHA256);

    let registry_root = temp.path().join("registry");
    std::fs::create_dir_all(registry_root.join("models")).expect("models dir");
    std::fs::write(
        registry_root.join("models").join("piper-lessac.toml"),
        toml::to_string_pretty(&manifest).expect("manifest toml"),
    )
    .expect("write manifest");
    let registry = PackageRegistry::new(&registry_root);
    let installed = InstalledRegistry::new(temp.path().join("home").join("manifests"));

    installed
        .install_model_with_options(
            &manifest,
            InstallModelOptions {
                metadata_only: true,
            },
        )
        .expect("metadata install");
    assert!(installed
        .installed_model_inventory(&registry)
        .expect("inventory")
        .data
        .is_empty());

    installed.install_model(&manifest).expect("full install");
    let inventory = installed
        .installed_model_inventory(&registry)
        .expect("inventory");
    assert_eq!(inventory.kind, "installed-models");
    assert_eq!(inventory.data.len(), 1);
    assert_eq!(inventory.data[0].name, "piper-lessac");
    assert_eq!(inventory.data[0].size_bytes, 5);
    assert_eq!(inventory.data[0].id.len(), 12);

    let record = installed
        .installed_model_record("piper-lessac")
        .expect("record");
    std::fs::remove_file(
        record.artifacts[0]
            .local_path
            .as_ref()
            .expect("artifact path"),
    )
    .expect("remove artifact");
    assert!(installed
        .installed_model_inventory(&registry)
        .expect("inventory")
        .data
        .is_empty());
}
