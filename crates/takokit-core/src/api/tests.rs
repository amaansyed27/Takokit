use super::*;

#[test]
fn legacy_daemon_identity_without_build_id_still_deserializes() {
    let identity: DaemonBuildIdentity = serde_json::from_str(
            r#"{"instance_id":null,"mode":"direct","pid":1,"executable":"takokit","storage_root":".takokit","host":"127.0.0.1","port":5050,"started_at":1,"log_path":null}"#,
        )
        .expect("legacy identity");
    assert!(identity.build_id.is_empty());
    assert_eq!(identity.identity.mode, DaemonMode::Direct);
}

#[test]
fn build_identity_serializes_as_one_flat_object() {
    let identity = DaemonBuildIdentity {
        identity: DaemonIdentity {
            instance_id: None,
            mode: DaemonMode::Direct,
            pid: 1,
            executable: PathBuf::from("takokit"),
            storage_root: PathBuf::from(".takokit"),
            host: "127.0.0.1".into(),
            port: 5050,
            started_at: 1,
            log_path: None,
        },
        build_id: "fixture".into(),
    };
    let value = serde_json::to_value(identity).unwrap();
    assert_eq!(value["mode"], "direct");
    assert_eq!(value["build_id"], "fixture");
    assert!(value.get("identity").is_none());
}

#[test]
fn speech_request_matches_openai_compatible_shape() {
    let request = SpeechRequest {
        model: "kokoro".to_string(),
        input: "Hello from Takokit".to_string(),
        voice: Some("default".to_string()),
        response_format: Some("wav".to_string()),
        language: None,
        instruction: None,
        reference_text: None,
    };
    let json = serde_json::to_value(request).expect("serializes");
    assert_eq!(json["model"], "kokoro");
    assert_eq!(json["input"], "Hello from Takokit");
    assert_eq!(json["voice"], "default");
    assert_eq!(json["response_format"], "wav");
}

#[test]
fn pull_model_request_keeps_metadata_only_optional() {
    let request: PullModelRequest =
        serde_json::from_str(r#"{"model":"piper-lessac"}"#).expect("pull request");
    assert_eq!(request.model, "piper-lessac");
    assert!(!request.metadata_only);
}

#[test]
fn rvc_request_defaults_are_stable_and_validated() {
    let request: VoiceConversionRequest = serde_json::from_str(
        r#"{"model":"rvc","source_path":"source.wav","target_voice":"voice"}"#,
    )
    .expect("RVC request");
    assert_eq!(request.f0_method, RvcF0Method::Rmvpe);
    assert_eq!(request.index_rate, 0.75);
    assert_eq!(request.rms_mix_rate, 0.25);
    assert_eq!(request.protect, 0.33);
    assert_eq!(request.filter_radius, 3);
    request.settings().validate().expect("default settings");

    let mut invalid = request.settings();
    invalid.index_rate = 1.5;
    assert!(invalid.validate().unwrap_err().contains("index rate"));
}

#[test]
fn model_install_report_serializes_typed_steps() {
    let report = ModelInstallReport {
        model_id: "fixture-model".into(),
        required_runner: "fixture-runner".into(),
        required_adapter: None,
        artifacts: InstallStep {
            state: InstallStepState::AlreadyReady,
            newly_installed: false,
            detail: "verified".into(),
        },
        runner_contract: InstallStep {
            state: InstallStepState::NotRequested,
            newly_installed: false,
            detail: "fixture".into(),
        },
        runner_runtime: InstallStep {
            state: InstallStepState::NotRequested,
            newly_installed: false,
            detail: "fixture".into(),
        },
        adapter: None,
        executable: false,
        missing: vec!["runner runtime".into()],
        logs_path: PathBuf::from("logs"),
    };
    let json = serde_json::to_value(report).expect("serialize report");
    assert_eq!(json["artifacts"]["state"], "already-ready");
    for key in [
        "model_id",
        "required_runner",
        "required_adapter",
        "artifacts",
        "runner_contract",
        "runner_runtime",
        "adapter",
        "executable",
        "missing",
        "logs_path",
    ] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
}
