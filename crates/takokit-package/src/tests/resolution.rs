use super::*;

#[test]
fn exposes_current_platform_identifier() {
    let platform = current_platform_id();

    assert!(platform.contains('-'));
    assert!(!platform.starts_with('-'));
    assert!(!platform.ends_with('-'));
}

#[test]
fn resolver_reports_model_not_installed_before_runner_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_test_registry(temp.path());
    let registry = PackageRegistry::new(temp.path());
    let installed = InstalledRegistry::new(temp.path().join("installed"));

    let error = resolve_execution_plan(
        &registry,
        &installed,
        "kokoro",
        CapabilityKind::TextToSpeech,
    )
    .expect_err("missing model");

    assert!(matches!(error, PackageError::ModelNotInstalled(id) if id == "kokoro"));
}

#[test]
fn resolver_reports_runner_missing_after_model_is_installed() {
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
        CapabilityKind::TextToSpeech,
    )
    .expect_err("missing runner");

    assert!(matches!(
        error,
        PackageError::RunnerNotInstalled { model, runner, capability, .. }
            if model == "kokoro" && runner == "takokit-onnx" && capability == CapabilityKind::TextToSpeech
    ));
}

#[test]
fn resolver_returns_execution_plan_after_model_and_runner_are_installed() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_test_registry(temp.path());
    let installed_root = temp.path().join("installed");
    let installed = InstalledRegistry::new(&installed_root);
    let registry = PackageRegistry::new(temp.path());
    let model = registry.model("kokoro").expect("model");
    let runner = registry.runner("takokit-onnx").expect("runner");
    installed.install_model(&model).expect("install model");
    installed.install_runner(&runner).expect("install runner");

    let plan = resolve_execution_plan(
        &registry,
        &installed,
        "kokoro",
        CapabilityKind::TextToSpeech,
    )
    .expect("execution plan");

    assert_eq!(plan.model.id, "kokoro");
    assert_eq!(plan.runner.id, "takokit-onnx");
    assert_eq!(plan.capability, CapabilityKind::TextToSpeech);
    assert!(plan.runner_installed);
    assert_eq!(plan.status, ExecutionStatus::Planned);
    assert_eq!(
        plan.installed_model
            .as_ref()
            .map(|record| record.id.as_str()),
        Some("kokoro")
    );
}

#[test]
fn lifecycle_enum_values_parse_from_manifest_strings() {
    assert_eq!(
        toml::from_str::<ModelLifecycleFixture>(r#"state = "metadata-only""#)
            .expect("metadata-only")
            .state,
        ModelLifecycleState::MetadataOnly
    );
    assert_eq!(
        toml::from_str::<RunnerLifecycleFixture>(r#"state = "contract-installed""#)
            .expect("contract-installed")
            .state,
        RunnerLifecycleState::ContractInstalled
    );
}

#[test]
fn model_plan_is_honest_for_piper_whisper_qwen_and_missing_model() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry = PackageRegistry::bundled();
    let installed = InstalledRegistry::new(temp.path().join("manifests"));
    let piper = registry.model("piper-lessac").expect("piper manifest");
    let piper_model_path = temp.path().join("en_US-lessac-medium.onnx");
    let piper_config_path = temp.path().join("en_US-lessac-medium.onnx.json");
    std::fs::write(&piper_model_path, b"model").expect("piper model fixture");
    std::fs::write(&piper_config_path, b"config").expect("piper config fixture");
    std::fs::create_dir_all(temp.path().join("manifests").join("installed-models"))
        .expect("installed model records dir");
    let piper_record = InstalledModelRecord {
        id: piper.id.clone(),
        version: piper.version.clone(),
        source: "test".to_string(),
        manifest_path: PathBuf::from("piper-lessac.toml"),
        runner: piper.runner.clone(),
        installed_at: "0".to_string(),
        artifacts: vec![
            InstalledArtifactRecord {
                name: "en_US-lessac-medium.onnx".to_string(),
                sha256: "test".to_string(),
                bytes: None,
                url: None,
                role: ArtifactRole::Model,
                local_path: Some(piper_model_path),
                downloaded: true,
            },
            InstalledArtifactRecord {
                name: "en_US-lessac-medium.onnx.json".to_string(),
                sha256: "test".to_string(),
                bytes: None,
                url: None,
                role: ArtifactRole::Config,
                local_path: Some(piper_config_path),
                downloaded: true,
            },
        ],
        snapshot: None,
        status: InstalledPackageStatus::Ready,
        note: "test".to_string(),
    };
    std::fs::write(
        temp.path()
            .join("manifests")
            .join("installed-models")
            .join("piper-lessac.toml"),
        toml::to_string_pretty(&piper_record).expect("record toml"),
    )
    .expect("write piper record");
    installed
        .install_runner(
            &registry
                .runner("takokit-python-managed")
                .expect("managed Python runner"),
        )
        .expect("install managed Python runner contract");

    let piper_plan = plan_model(&registry, &installed, "piper-lessac").expect("piper plan");
    assert_eq!(piper_plan.model_id, "piper-lessac");
    assert_eq!(piper_plan.family, "piper-lessac");
    assert_eq!(piper_plan.required_runner, "takokit-python-managed");
    assert_eq!(piper_plan.artifact_state, ModelLifecycleState::MetadataOnly);
    assert_eq!(
        piper_plan.runner_contract_state,
        RunnerLifecycleState::ContractInstalled
    );
    assert_eq!(
        piper_plan.runner_runtime_state,
        RunnerLifecycleState::ContractInstalled
    );
    assert!(!piper_plan.executable);
    assert!(piper_plan
        .missing
        .iter()
        .any(|item| item == "verified artifacts"));
    assert!(piper_plan
        .missing
        .iter()
        .any(|item| item.contains("piper managed adapter")));

    let whisper_plan = plan_model(&registry, &installed, "whisper-base").expect("whisper plan");
    assert_eq!(whisper_plan.required_runner, "takokit-whispercpp");
    assert_eq!(whisper_plan.family, "whisper");
    assert_eq!(
        whisper_plan.artifact_state,
        ModelLifecycleState::MetadataOnly
    );
    assert_eq!(
        whisper_plan.runner_runtime_state,
        RunnerLifecycleState::RuntimeMissing
    );
    assert!(!whisper_plan.executable);

    let qwen_plan = plan_model(&registry, &installed, "qwen3-tts").expect("qwen plan");
    assert_eq!(qwen_plan.required_runner, "takokit-python-managed");
    assert_eq!(qwen_plan.family, "qwen3-tts");
    assert_eq!(qwen_plan.artifact_state, ModelLifecycleState::MetadataOnly);
    assert!(qwen_plan
        .missing
        .iter()
        .any(|item| item.contains("qwen3_tts managed adapter")));

    let missing = plan_model(&registry, &installed, "does-not-exist")
        .expect_err("missing model should not plan");
    assert!(matches!(missing, PackageError::ModelNotFound(id) if id == "does-not-exist"));
}

#[test]
fn model_info_is_derived_from_canonical_lifecycle_plan() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry = PackageRegistry::bundled();
    let installed = InstalledRegistry::new(temp.path().join("manifests"));
    let whisper = registry.model("whisper-base").expect("whisper manifest");
    let runner = registry
        .runner("takokit-whispercpp")
        .expect("whisper runner");

    installed
        .install_model_with_options(
            &whisper,
            InstallModelOptions {
                metadata_only: true,
            },
        )
        .expect("metadata-only whisper");
    installed
        .install_runner_runtime(&runner, RunnerLifecycleState::Ready, "test runner ready")
        .expect("runner ready");

    let plan = plan_model(&registry, &installed, "whisper-base").expect("plan");
    let info = registry
        .model("whisper-base")
        .expect("manifest")
        .to_model_info_from_plan(&plan, true, true);

    assert_eq!(info.family, "whisper");
    assert_eq!(
        info.lifecycle_state,
        ModelLifecycleState::MetadataOnly.to_string()
    );
    assert_eq!(
        info.runner_runtime_state,
        RunnerLifecycleState::Ready.to_string()
    );
    assert!(!info.executable);
    assert!(info.execution_status.contains("metadata-only"));
    assert_eq!(
        info.next_command,
        "takokit runner doctor takokit-whispercpp"
    );
    assert!(info.missing.iter().any(|item| item == "verified artifacts"));
}

#[test]
fn bundled_runtime_models_are_not_marked_executable_without_ready_runners() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry = PackageRegistry::bundled();
    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    for model in registry.models().expect("runtime models") {
        let plan = plan_model(&registry, &installed, &model.id).expect("model plan");

        assert!(!plan.executable, "{} should not be executable", model.id);
        assert_ne!(
            plan.artifact_state,
            ModelLifecycleState::Executable,
            "{} should not claim executable artifact state",
            model.id
        );
    }
}
