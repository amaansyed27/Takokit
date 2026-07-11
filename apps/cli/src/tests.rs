use super::*;

#[test]
fn cli_accepts_bare_command_for_interactive_launcher() {
    let cli = Cli::try_parse_from(["takokit"]).expect("bare cli parse");

    assert!(cli.command.is_none());
}

#[test]
fn cli_parses_doctor_command() {
    let cli = Cli::try_parse_from(["takokit", "doctor"]).expect("doctor cli parse");

    assert!(matches!(
        cli.command,
        Some(Command::Doctor(DoctorArgs { json: false }))
    ));
}

#[test]
fn cli_parses_json_plan_doctor_runner_doctor_and_test_file_options() {
    let doctor = Cli::try_parse_from(["takokit", "doctor", "--json"]).expect("doctor json");
    let plan =
        Cli::try_parse_from(["takokit", "plan", "whisper-base", "--json"]).expect("plan json");
    let runner = Cli::try_parse_from([
        "takokit",
        "runner",
        "doctor",
        "takokit-whispercpp",
        "--json",
    ])
    .expect("runner doctor json");
    let test = Cli::try_parse_from([
        "takokit",
        "test",
        "whisper-base",
        "--file",
        "sample.wav",
        "--json",
    ])
    .expect("test file json");

    assert!(matches!(
        doctor.command,
        Some(Command::Doctor(DoctorArgs { json: true }))
    ));
    assert!(matches!(
        plan.command,
        Some(Command::Plan(PlanArgs { model, json: true })) if model == "whisper-base"
    ));
    assert!(matches!(
        runner.command,
        Some(Command::Runner {
            command: RunnerCommand::Doctor { runner, json: true }
        }) if runner == "takokit-whispercpp"
    ));
    assert!(matches!(
        test.command,
        Some(Command::Test(TestArgs {
            model: Some(model),
            suite: None,
            json: true,
            file: Some(file),
            run: false,
            ..
        })) if model == "whisper-base" && file == PathBuf::from("sample.wav")
    ));
}

#[test]
fn cli_parses_version_command() {
    let cli = Cli::try_parse_from(["takokit", "version"]).expect("version cli parse");

    assert!(matches!(cli.command, Some(Command::Version)));
}

#[test]
fn tako_alias_parses_doctor_and_uses_takokit_storage_root() {
    let cli = Cli::try_parse_from(["tako", "doctor"]).expect("tako doctor cli parse");
    let storage_root = cli_storage_root();

    assert!(matches!(cli.command, Some(Command::Doctor(_))));
    assert_eq!(storage_root, LocalStore::default_root());
    assert_eq!(
        storage_root.file_name().and_then(|name| name.to_str()),
        Some(".takokit")
    );
}

#[test]
fn cli_parses_model_and_runner_aliases() {
    let models = Cli::try_parse_from(["takokit", "models"]).expect("models alias");
    let runners = Cli::try_parse_from(["takokit", "runners"]).expect("runners alias");

    assert!(matches!(models.command, Some(Command::Models)));
    assert!(matches!(runners.command, Some(Command::Runners)));
}

#[test]
fn cli_parses_direct_list_run_and_ps() {
    let direct = Cli::try_parse_from(["takokit", "--direct", "list"]).expect("direct list");
    let run =
        Cli::try_parse_from(["takokit", "run", "kokoro", "hello", "--voice", "Ryan"]).expect("run");
    let ps = Cli::try_parse_from(["takokit", "ps"]).expect("ps");
    assert!(direct.direct);
    assert!(matches!(
        direct.command,
        Some(Command::List { target: None })
    ));
    assert!(
        matches!(run.command, Some(Command::Run(RunArgs { model, text: Some(text), voice: Some(voice), file: None })) if model == "kokoro" && text == "hello" && voice == "Ryan")
    );
    assert!(matches!(ps.command, Some(Command::Ps)));
}

#[test]
fn run_argument_validation_accepts_tts_or_stt_and_rejects_ambiguous_input() {
    let tts = RunArgs {
        model: "kokoro".into(),
        text: Some("hello".into()),
        voice: None,
        file: None,
    };
    let stt = RunArgs {
        model: "whisper-tiny".into(),
        text: None,
        voice: None,
        file: Some(PathBuf::from("sample.wav")),
    };
    let missing = RunArgs {
        model: "kokoro".into(),
        text: None,
        voice: None,
        file: None,
    };
    let both = RunArgs {
        model: "kokoro".into(),
        text: Some("hello".into()),
        voice: None,
        file: Some(PathBuf::from("sample.wav")),
    };
    assert!(validate_run_args(&tts).is_ok());
    assert!(validate_run_args(&stt).is_ok());
    assert!(validate_run_args(&missing).is_err());
    assert!(validate_run_args(&both).is_err());
}

#[test]
fn cli_parses_library_model_and_runner_commands() {
    let models =
        Cli::try_parse_from(["takokit", "library", "models"]).expect("library models command");
    let runners =
        Cli::try_parse_from(["takokit", "library", "runners"]).expect("library runners command");

    assert!(matches!(
        models.command,
        Some(Command::Library {
            target: LibraryTarget::Models
        })
    ));
    assert!(matches!(
        runners.command,
        Some(Command::Library {
            target: LibraryTarget::Runners
        })
    ));
}

#[test]
fn cli_parses_metadata_only_model_pull() {
    let cli = Cli::try_parse_from(["takokit", "pull", "piper-lessac", "--metadata-only"])
        .expect("metadata-only pull");

    assert!(matches!(
        cli.command,
        Some(Command::Pull(PullArgs { model, metadata_only: true })) if model == "piper-lessac"
    ));
}

#[test]
fn cli_parses_model_plan_command() {
    let cli = Cli::try_parse_from(["takokit", "plan", "qwen3-tts"]).expect("plan command");

    assert!(matches!(
        cli.command,
        Some(Command::Plan(PlanArgs { model, json: false })) if model == "qwen3-tts"
    ));
}

#[test]
fn cli_parses_runner_install_and_doctor_commands() {
    let install = Cli::try_parse_from(["takokit", "runner", "install", "takokit-onnx"])
        .expect("runner install");
    let doctor = Cli::try_parse_from(["takokit", "runner", "doctor", "takokit-onnx"])
        .expect("runner doctor");

    assert!(matches!(
        install.command,
        Some(Command::Runner {
            command: RunnerCommand::Install { runner }
        }) if runner == "takokit-onnx"
    ));
    assert!(matches!(
        doctor.command,
        Some(Command::Runner {
            command: RunnerCommand::Doctor { runner, json: false }
        }) if runner == "takokit-onnx"
    ));
}

#[test]
fn cli_parses_adapter_and_launch_run_commands() {
    let adapter = Cli::try_parse_from(["takokit", "adapter", "install", "qwen3-tts"])
        .expect("adapter install");
    let suite =
        Cli::try_parse_from(["takokit", "test", "--suite", "launch", "--run"]).expect("launch run");

    assert!(matches!(
        adapter.command,
        Some(Command::Adapter {
            command: AdapterCommand::Install { adapter }
        }) if adapter == "qwen3-tts"
    ));
    assert!(matches!(
        suite.command,
        Some(Command::Test(TestArgs { suite: Some(name), run: true, .. })) if name == "launch"
    ));
}

#[test]
fn cli_parses_quickstart_deps_samples_and_fast_suite() {
    let quickstart = Cli::try_parse_from(["takokit", "quickstart", "--full"]).expect("quickstart");
    let deps = Cli::try_parse_from(["takokit", "deps", "bootstrap"]).expect("deps");
    let samples = Cli::try_parse_from(["takokit", "samples", "create"]).expect("samples");
    let fast =
        Cli::try_parse_from(["takokit", "test", "--suite", "fast", "--run"]).expect("fast suite");

    assert!(matches!(
        quickstart.command,
        Some(Command::Quickstart(QuickstartArgs { full: true }))
    ));
    assert!(matches!(
        deps.command,
        Some(Command::Deps {
            command: DepsCommand::Bootstrap
        })
    ));
    assert!(matches!(
        samples.command,
        Some(Command::Samples {
            command: SamplesCommand::Create
        })
    ));
    assert!(
        matches!(fast.command, Some(Command::Test(TestArgs { suite: Some(name), run: true, .. })) if name == "fast")
    );
}

#[test]
fn cli_parses_model_and_launch_suite_test_commands() {
    let model = Cli::try_parse_from(["takokit", "test", "whisper-base"]).expect("model test");
    let suite = Cli::try_parse_from(["takokit", "test", "--suite", "launch"]).expect("suite test");

    assert!(matches!(
        model.command,
        Some(Command::Test(TestArgs { model: Some(model), suite: None, json: false, file: None, run: false, .. })) if model == "whisper-base"
    ));
    assert!(matches!(
        suite.command,
        Some(Command::Test(TestArgs { model: None, suite: Some(suite), json: false, file: None, run: false, .. })) if suite == "launch"
    ));
}

#[test]
fn model_show_output_uses_canonical_planner_status() {
    let info = takokit_core::ModelInfo {
        id: "whisper-base".to_string(),
        name: "Whisper Base".to_string(),
        family: "whisper".to_string(),
        version: "0.1.0".to_string(),
        summary: "Local STT".to_string(),
        license: "mit".to_string(),
        license_warning: None,
        runtime: takokit_core::ModelRuntime::WhisperCpp,
        backend: "whispercpp".to_string(),
        runner: "takokit-whispercpp".to_string(),
        hardware_notes: "CPU".to_string(),
        artifact_count: 1,
        capabilities: vec![CapabilityKind::SpeechToText],
        installed: true,
        runner_installed: true,
        runner_runtime_state: "ready".to_string(),
        lifecycle_state: "executable".to_string(),
        executable: true,
        missing: Vec::new(),
        next_command: "takokit test whisper-base".to_string(),
        execution_status: "executable".to_string(),
    };

    let output = format_model_show(&info, None);

    assert!(output.contains("lifecycle: executable"));
    assert!(output.contains("status: executable"));
    assert!(output.contains("runner runtime: ready"));
    assert!(!output.contains("real inference is not implemented"));
}

#[test]
fn runner_show_output_uses_persisted_runtime_state() {
    let registry = PackageRegistry::bundled();
    let manifest = registry
        .runner("takokit-whispercpp")
        .expect("whisper runner");
    let output = format_runner_show(
        &manifest,
        true,
        Some(takokit_package::RunnerLifecycleState::Ready),
        Some("whisper.cpp runtime installed".to_string()),
        PathBuf::from("C:/takokit/runners/whispercpp"),
    );

    assert!(output.contains("runtime state: ready"));
    assert!(output.contains("status: ready"));
    assert!(!output.contains("runner contract installed only"));
}

#[test]
fn launch_suite_default_is_human_readable_and_json_flag_is_json() {
    let rows = vec![LaunchSuiteRow {
        model: "whisper-base".to_string(),
        task: Some("STT / Live Transcription API".to_string()),
        runner: Some("takokit-whispercpp".to_string()),
        lifecycle: Some("executable".to_string()),
        artifacts: Some("artifacts-ready".to_string()),
        runner_runtime: Some("ready".to_string()),
        executable: Some(true),
        missing: Vec::new(),
        next_command: Some("takokit test whisper-base".to_string()),
        run_result: None,
        error: None,
    }];

    let human = format_launch_suite(&rows, false).expect("human output");
    let json = format_launch_suite(&rows, true).expect("json output");

    assert!(human.contains("Launch test suite"));
    assert!(human.contains("whisper-base"));
    assert!(!human.trim_start().starts_with('['));
    assert!(json.trim_start().starts_with('['));
}

#[test]
fn launcher_menu_is_available_without_running_it() {
    let labels: Vec<_> = tui::launcher_actions()
        .iter()
        .map(|action| action.label())
        .collect();

    assert!(labels.contains(&"Generate speech with mock-tts"));
    assert!(labels.contains(&"Pull model metadata"));
    assert!(labels.contains(&"Pull runner contract"));
    assert!(labels.contains(&"Doctor"));
    assert!(labels.contains(&"Quit"));
}

#[test]
fn doctor_reports_storage_layout_and_registry_health() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = LocalStore::new(temp.path().join("takokit"));
    store.ensure_layout().expect("layout");
    let config = RuntimeConfig::local(store.root().to_path_buf());
    let package_registry = PackageRegistry::bundled();
    let installed_registry = InstalledRegistry::new(store.manifests_dir());

    let report = doctor::run_doctor(&config, &store, &package_registry, &installed_registry);

    assert!(!report.has_failures());
    assert!(report
        .checks()
        .iter()
        .any(|check| check.label().contains("model manifests found") && check.is_ok()));
    assert!(report
        .checks()
        .iter()
        .any(|check| check.label().contains("installed model records parse") && check.is_ok()));
    assert!(report
        .checks()
        .iter()
        .any(|check| check.label().contains("python-managed/runtime") && check.is_ok()));
    assert!(report.checks().iter().any(|check| check
        .label()
        .contains("python-managed runtime not initialized")));
}

#[test]
fn runtime_resolution_errors_include_code_prefix() {
    let error = runtime_error(TakokitError::Resolution {
        code: takokit_core::ErrorCode::InferenceNotImplemented,
        message: "ONNX runner contract resolved, but real ONNX execution is not implemented yet."
            .to_string(),
    });

    assert_eq!(
        error.to_string(),
        "inference_not_implemented: ONNX runner contract resolved, but real ONNX execution is not implemented yet."
    );
}
