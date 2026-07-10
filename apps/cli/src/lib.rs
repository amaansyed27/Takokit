mod doctor;
mod gui;
mod tui;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use std::{path::PathBuf, time::Instant};
use takokit_audio::{write_silence_wav, WavSpec};
use takokit_core::{CapabilityKind, ModelInfo, RuntimeConfig, SpeechRequest, TakokitError};
use takokit_models::{
    execute_speech, execute_transcription, MockTextToSpeechEngine, ModelRegistry,
    TextToSpeechEngine,
};
use takokit_package::{
    bootstrap_uv, find_uv, initialize_runner_runtime, install_python_adapter, model_info_from_plan,
    plan_model, python_adapter_record, python_adapter_records, resolve_execution_plan,
    runner_runtime_layout, InstallModelOptions, InstalledRegistry, ModelPlan, PackageError,
    PackageRegistry, RunnerManifest,
};
use takokit_server::{run_server, AppState};
use takokit_store::LocalStore;

#[derive(Debug, Parser)]
#[command(name = "takokit", version, about = "Local voice AI runtime")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve,
    Gui,
    Doctor(DoctorArgs),
    Version,
    Status,
    Capabilities,
    Models,
    Runners,
    Library {
        #[command(subcommand)]
        target: LibraryTarget,
    },
    Speak(SpeakArgs),
    Pull(PullArgs),
    Show {
        model: String,
    },
    Plan(PlanArgs),
    Rm {
        model: String,
    },
    List {
        #[command(subcommand)]
        target: ListTarget,
    },
    Runner {
        #[command(subcommand)]
        command: RunnerCommand,
    },
    Adapter {
        #[command(subcommand)]
        command: AdapterCommand,
    },
    Quickstart(QuickstartArgs),
    Deps {
        #[command(subcommand)]
        command: DepsCommand,
    },
    Samples {
        #[command(subcommand)]
        command: SamplesCommand,
    },
    Test(TestArgs),
    Transcribe {
        audio: PathBuf,
        #[arg(long, default_value = "whisper-base")]
        model: String,
    },
    Clone(CloneArgs),
    Train(TrainArgs),
}

#[derive(Debug, Args)]
struct SpeakArgs {
    text: String,
    #[arg(long, default_value = "mock-tts")]
    model: String,
    #[arg(long, default_value = "default")]
    voice: String,
}

#[derive(Debug, Args)]
struct PullArgs {
    model: String,
    #[arg(long)]
    metadata_only: bool,
}

#[derive(Debug, Args)]
struct DoctorArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct PlanArgs {
    model: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct TestArgs {
    model: Option<String>,
    #[arg(long)]
    suite: Option<String>,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    run: bool,
    #[arg(long)]
    category: Option<String>,
    #[arg(long)]
    include_heavy: bool,
}

#[derive(Debug, Args)]
struct QuickstartArgs {
    #[arg(long)]
    full: bool,
}

#[derive(Debug, Subcommand)]
enum DepsCommand {
    Doctor,
    Bootstrap,
}

#[derive(Debug, Subcommand)]
enum SamplesCommand {
    Create,
}

#[derive(Debug, Args)]
struct CloneArgs {
    sample: PathBuf,
    #[arg(long)]
    name: String,
}

#[derive(Debug, Args)]
struct TrainArgs {
    samples: PathBuf,
    #[arg(long)]
    name: String,
}

#[derive(Debug, Subcommand)]
enum ListTarget {
    Models,
    Runners,
    Voices,
}

#[derive(Debug, Subcommand)]
enum LibraryTarget {
    Models,
    Runners,
}

#[derive(Debug, Subcommand)]
enum RunnerCommand {
    Pull {
        runner: String,
    },
    Install {
        runner: String,
    },
    Doctor {
        runner: String,
        #[arg(long)]
        json: bool,
    },
    Show {
        runner: String,
    },
    Rm {
        runner: String,
    },
}

#[derive(Debug, Subcommand)]
enum AdapterCommand {
    List,
    Install {
        adapter: String,
    },
    Doctor {
        adapter: String,
        #[arg(long)]
        json: bool,
    },
}

fn cli_storage_root() -> PathBuf {
    LocalStore::default_root()
}

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let store = LocalStore::new(cli_storage_root());
    store.ensure_layout()?;
    let config = RuntimeConfig::local(store.root().to_path_buf());
    let package_registry = PackageRegistry::bundled();
    let installed_registry = InstalledRegistry::new(store.manifests_dir());

    match cli.command {
        None => tui::run_launcher(&config, &store, &package_registry, &installed_registry).await?,
        Some(Command::Serve) => {
            run_server(AppState::new(config, store)).await?;
        }
        Some(Command::Gui) => gui::open_gui(&config).await?,
        Some(Command::Doctor(args)) => {
            let report =
                doctor::run_doctor(&config, &store, &package_registry, &installed_registry);
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                doctor::print_report(&report);
            }
            if report.has_failures() {
                std::process::exit(1);
            }
        }
        Some(Command::Quickstart(args)) => {
            run_quickstart(&store, &package_registry, &installed_registry, args.full).await?;
        }
        Some(Command::Deps { command }) => match command {
            DepsCommand::Doctor => print_deps_doctor(&store),
            DepsCommand::Bootstrap => {
                let uv = bootstrap_uv(store.root()).map_err(cli_error)?;
                println!("uv ready: {}", uv.display());
                println!(
                    "log: {}",
                    store.logs_dir().join("uv-bootstrap.log").display()
                );
            }
        },
        Some(Command::Samples {
            command: SamplesCommand::Create,
        }) => {
            create_samples(&store, &package_registry, &installed_registry).await?;
        }
        Some(Command::Version) => {
            println!("takokit {}", env!("CARGO_PKG_VERSION"));
            println!("storage: {}", store.root().display());
        }
        Some(Command::Status) => {
            let state = AppState::new(config, store);
            println!("{}", serde_json::to_string_pretty(&state.status())?);
        }
        Some(Command::Capabilities) => {
            for capability in CapabilityKind::ALL {
                println!("{} - {}", capability.label(), capability.explanation());
            }
        }
        Some(Command::Models) => print_models(&package_registry, &installed_registry)?,
        Some(Command::Runners) => print_runners(&package_registry, &installed_registry)?,
        Some(Command::Library { target }) => match target {
            LibraryTarget::Models => print_library_models(&package_registry)?,
            LibraryTarget::Runners => print_library_runners(&package_registry)?,
        },
        Some(Command::Speak(args)) => {
            if args.model != "mock-tts" {
                let plan = resolve_execution_plan(
                    &package_registry,
                    &installed_registry,
                    &args.model,
                    CapabilityKind::TextToSpeech,
                )
                .map_err(cli_error)?;
                let response = execute_speech(
                    &plan,
                    SpeechRequest {
                        model: args.model,
                        input: args.text,
                        voice: Some(args.voice),
                        response_format: Some("wav".to_string()),
                    },
                    &store.outputs_dir(),
                )
                .await
                .map_err(runtime_error)?;
                println!("{}", serde_json::to_string_pretty(&response)?);
                return Ok(());
            }

            let engine = MockTextToSpeechEngine;
            let response = engine
                .synthesize(
                    SpeechRequest {
                        model: args.model,
                        input: args.text,
                        voice: Some(args.voice),
                        response_format: Some("wav".to_string()),
                    },
                    &store.outputs_dir(),
                )
                .await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Some(Command::Pull(args)) => {
            let manifest = package_registry.model(&args.model).map_err(cli_error)?;
            let report = installed_registry
                .install_model_with_options(
                    &manifest,
                    InstallModelOptions {
                        metadata_only: args.metadata_only,
                    },
                )
                .map_err(cli_error)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some(Command::Show { model }) => {
            let manifest = package_registry.model(&model).map_err(cli_error)?;
            let info = model_info_from_plan(&package_registry, &installed_registry, &manifest.id)
                .map_err(cli_error)?;
            let installed_record = installed_registry.installed_model_record(&manifest.id).ok();
            println!("{}", format_model_show(&info, installed_record.as_ref()));
        }
        Some(Command::Plan(args)) => {
            let plan = plan_model(&package_registry, &installed_registry, &args.model)
                .map_err(cli_error)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                print_model_plan(&plan);
            }
        }
        Some(Command::Rm { model }) => {
            let removed = installed_registry.remove_model(&model).map_err(cli_error)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "id": model,
                    "removed": removed
                }))?
            );
        }
        Some(Command::List { target }) => {
            let registry = ModelRegistry::default();
            match target {
                ListTarget::Models => print_models(&package_registry, &installed_registry)?,
                ListTarget::Runners => print_runners(&package_registry, &installed_registry)?,
                ListTarget::Voices => {
                    println!("{}", serde_json::to_string_pretty(registry.voices())?)
                }
            }
        }
        Some(Command::Transcribe { audio, model }) => {
            let plan = resolve_execution_plan(
                &package_registry,
                &installed_registry,
                &model,
                CapabilityKind::SpeechToText,
            )
            .map_err(cli_error)?;
            let response = execute_transcription(
                &plan,
                takokit_core::TranscriptionRequest {
                    file_path: audio,
                    model: Some(model),
                },
            )
            .await
            .map_err(runtime_error)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            return Ok(());
        }
        Some(Command::Clone(args)) => {
            let _ = args;
            return not_implemented(
                "voice cloning",
                "clone adapters require explicit model runner integration",
            );
        }
        Some(Command::Train(args)) => {
            let _ = args;
            return not_implemented(
                "voice training",
                "training jobs and dataset preparation are planned for a later phase",
            );
        }
        Some(Command::Runner { command }) => match command {
            RunnerCommand::Pull { runner } => {
                let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                let report = installed_registry
                    .install_runner(&manifest)
                    .map_err(cli_error)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            RunnerCommand::Install { runner } => {
                let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                let report =
                    initialize_runner_runtime(store.root(), &installed_registry, &manifest)
                        .map_err(cli_error)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            RunnerCommand::Doctor { runner, json } => {
                let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                if json {
                    print_runner_doctor_json(&store, &installed_registry, &manifest)?;
                } else {
                    print_runner_doctor(&store, &installed_registry, &manifest);
                }
            }
            RunnerCommand::Show { runner } => {
                let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                let installed = installed_registry.is_runner_installed(&manifest.id);
                let record = installed_registry
                    .installed_runner_record(&manifest.id)
                    .ok();
                let layout = runner_runtime_layout(store.root(), &manifest);
                println!(
                    "{}",
                    format_runner_show(
                        &manifest,
                        installed,
                        record.as_ref().map(|record| record.status),
                        record.as_ref().map(|record| record.note.clone()),
                        layout.root,
                    )
                );
            }
            RunnerCommand::Rm { runner } => {
                let removed = installed_registry
                    .remove_runner(&runner)
                    .map_err(cli_error)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "id": runner,
                        "removed": removed
                    }))?
                );
            }
        },
        Some(Command::Adapter { command }) => match command {
            AdapterCommand::List => {
                let records = python_adapter_records(store.root()).map_err(cli_error)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            AdapterCommand::Install { adapter } => {
                let adapter = normalize_adapter_id(&adapter);
                let record = install_python_adapter(store.root(), &adapter).map_err(cli_error)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            AdapterCommand::Doctor { adapter, json } => {
                let adapter = normalize_adapter_id(&adapter);
                let record = python_adapter_record(store.root(), &adapter).map_err(cli_error)?;
                print_adapter_doctor(&store, &record, json)?;
            }
        },
        Some(Command::Test(args)) => {
            run_test_command(&store, &package_registry, &installed_registry, args).await?
        }
    }

    Ok(())
}

fn normalize_adapter_id(adapter: &str) -> String {
    adapter.trim().replace('-', "_")
}

fn print_adapter_doctor(
    store: &LocalStore,
    record: &takokit_package::AdapterRecord,
    json: bool,
) -> anyhow::Result<()> {
    let adapter_dir = store.python_managed_adapters_dir().join(&record.id);
    let python = if cfg!(windows) {
        store
            .python_managed_env_dir()
            .join("venv")
            .join("Scripts")
            .join("python.exe")
    } else {
        store
            .python_managed_env_dir()
            .join("venv")
            .join("bin")
            .join("python3")
    };
    let payload = serde_json::json!({
        "id": record.id,
        "model_family": record.model_family,
        "state": record.state,
        "notes": record.notes,
        "adapter_path": adapter_dir,
        "adapter_script": adapter_dir.join("qwen3_tts.py"),
        "python": python,
        "python_present": python.is_file(),
        "log_path": adapter_dir.join("install.log"),
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("Adapter Doctor: {} ({})", record.id, record.model_family);
        println!("state: {}", record.state);
        println!("python: {}", python.display());
        println!("python present: {}", yes_no(python.is_file()));
        println!("adapter path: {}", adapter_dir.display());
        println!("log path: {}", adapter_dir.join("install.log").display());
        println!("note: {}", record.notes);
    }
    Ok(())
}

fn print_deps_doctor(store: &LocalStore) {
    let uv = find_uv(store.root());
    println!("Dependency doctor");
    match uv {
        Some(path) => println!("uv: ready ({})", path.display()),
        None => {
            println!("uv: missing");
            println!("next: takokit deps bootstrap");
            println!(
                "log: {}",
                store.logs_dir().join("uv-bootstrap.log").display()
            );
        }
    }
    println!(
        "managed Python: {}",
        if store.python_managed_env_dir().join("venv").is_dir() {
            "present"
        } else {
            "missing"
        }
    );
    println!("storage: {}", store.root().display());
}

async fn run_quickstart(
    store: &LocalStore,
    registry: &PackageRegistry,
    installed: &InstalledRegistry,
    full: bool,
) -> anyhow::Result<()> {
    println!("Takokit fast local setup");
    let uv = bootstrap_uv(store.root()).map_err(cli_error)?;
    println!("Dependency bootstrap: ready ({})", uv.display());
    for runner in ["takokit-onnx", "takokit-whispercpp"] {
        let manifest = registry.runner(runner).map_err(cli_error)?;
        installed.install_runner(&manifest).map_err(cli_error)?;
        initialize_runner_runtime(store.root(), installed, &manifest).map_err(cli_error)?;
    }
    for model in ["kokoro", "whisper-tiny"] {
        let manifest = registry.model(model).map_err(cli_error)?;
        installed.install_model(&manifest).map_err(cli_error)?;
    }
    if full {
        let manifest = registry
            .runner("takokit-python-managed")
            .map_err(cli_error)?;
        installed.install_runner(&manifest).map_err(cli_error)?;
        initialize_runner_runtime(store.root(), installed, &manifest).map_err(cli_error)?;
        let record = install_python_adapter(store.root(), "qwen3_tts").map_err(cli_error)?;
        println!("Qwen adapter: {}", record.state);
        let manifest = registry.model("qwen3-tts").map_err(cli_error)?;
        installed.install_model(&manifest).map_err(cli_error)?;
    }
    // This is intentionally an execution test, not a lifecycle check.  It
    // creates speech with Kokoro and transcribes that exact WAV with Whisper.
    run_fast_smokes(store, registry, installed, true, false).await?;
    for (label, model) in [
        ("Kokoro TTS", "kokoro"),
        ("Whisper Tiny STT", "whisper-tiny"),
    ] {
        let plan = plan_model(registry, installed, model).map_err(cli_error)?;
        println!(
            "{label}: {}",
            if plan.executable {
                "ready".to_string()
            } else {
                format!("blocked with reason: {}", plan.missing.join("; "))
            }
        );
    }
    println!(
        "GUI: {}",
        if takokit_server::router::gui_dist_path()
            .join("index.html")
            .is_file()
        {
            "available"
        } else {
            "build missing"
        }
    );
    println!("Storage: {}", store.root().display());
    println!("Next:\n  takokit speak \"Hello from Takokit\" --model kokoro\n  takokit samples create\n  takokit transcribe {} --model whisper-tiny\n  takokit gui", store.root().join("samples").join("hello.wav").display());
    Ok(())
}

async fn create_samples(
    store: &LocalStore,
    registry: &PackageRegistry,
    installed: &InstalledRegistry,
) -> anyhow::Result<()> {
    let samples = store.root().join("samples");
    std::fs::create_dir_all(&samples)?;
    let hello = samples.join("hello.wav");
    let silence = samples.join("silence.wav");
    let plan = plan_model(registry, installed, "kokoro").map_err(cli_error)?;
    if plan.executable {
        match resolve_execution_plan(registry, installed, "kokoro", CapabilityKind::TextToSpeech)
            .map_err(cli_error)
        {
            Ok(execution) => match execute_speech(
                &execution,
                SpeechRequest {
                    model: "kokoro".to_string(),
                    input: "Hello from Takokit.".to_string(),
                    voice: Some("af_heart".to_string()),
                    response_format: Some("wav".to_string()),
                },
                &samples,
            )
            .await
            {
                Ok(response) => {
                    std::fs::copy(response.output_path, &hello)?;
                    println!("hello.wav: Kokoro speech ({})", hello.display());
                }
                Err(error) => return Err(runtime_error(error)),
            },
            Err(error) => return Err(error),
        }
    } else {
        return Err(anyhow::anyhow!(
            "Kokoro is not executable; refusing to create hello.wav from silence"
        ));
    }
    write_silence_wav(&silence, 500, WavSpec::default())?;
    println!("silence.wav: {}", silence.display());
    Ok(())
}

async fn run_test_command(
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    args: TestArgs,
) -> anyhow::Result<()> {
    if args.suite.as_deref() == Some("launch") {
        print_launch_suite(package_registry, installed_registry, args.json, args.run).await?;
        return Ok(());
    }
    if args.suite.as_deref() == Some("fast") {
        run_fast_smokes(
            store,
            package_registry,
            installed_registry,
            args.run,
            args.json,
        )
        .await?;
        return Ok(());
    }

    let Some(model) = args.model else {
        return Err(TakokitError::InvalidRequest(
            "provide a model id or --suite launch|fast".to_string(),
        )
        .into());
    };
    let plan = plan_model(package_registry, installed_registry, &model).map_err(cli_error)?;
    if let Some(file) = args.file {
        if !plan.executable {
            print_or_json_plan(&plan, args.json)?;
            return Err(anyhow::anyhow!(
                "model is not executable; missing: {}",
                plan.missing.join("; ")
            ));
        }
        let execution = resolve_execution_plan(
            package_registry,
            installed_registry,
            &model,
            CapabilityKind::SpeechToText,
        )
        .map_err(cli_error)?;
        let response = execute_transcription(
            &execution,
            takokit_core::TranscriptionRequest {
                file_path: file,
                model: Some(model),
            },
        )
        .await
        .map_err(runtime_error)?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    print_or_json_plan(&plan, args.json)?;
    if !args.json {
        println!(
            "Test result: {}",
            if plan.executable {
                "executable; provide --file <audio.wav> for a real STT smoke test when applicable"
            } else {
                "blocked"
            }
        );
    }
    Ok(())
}

async fn run_fast_smokes(
    store: &LocalStore,
    registry: &PackageRegistry,
    installed: &InstalledRegistry,
    run: bool,
    json: bool,
) -> anyhow::Result<()> {
    let mut rows = Vec::new();
    let phrase = "Hello from Takokit.";
    let mut spoken_sample: Option<PathBuf> = None;
    for id in ["kokoro", "whisper-tiny", "whisper-base", "qwen3-tts"] {
        let plan = plan_model(registry, installed, id).map_err(cli_error)?;
        if !plan.executable {
            rows.push(FastSuiteRow::skipped(
                id,
                if plan.missing.is_empty() {
                    "not installed".to_string()
                } else {
                    plan.missing.join("; ")
                },
                store,
            ));
            continue;
        }
        if !run {
            rows.push(FastSuiteRow::ready(id, store));
            continue;
        }
        let started = Instant::now();
        match id {
            "kokoro" | "qwen3-tts" => {
                let plan =
                    resolve_execution_plan(registry, installed, id, CapabilityKind::TextToSpeech)
                        .map_err(cli_error)?;
                match execute_speech(
                    &plan,
                    SpeechRequest {
                        model: id.to_string(),
                        input: phrase.to_string(),
                        voice: None,
                        response_format: Some("wav".to_string()),
                    },
                    &store.outputs_dir(),
                )
                .await
                {
                    Ok(response) => {
                        if id == "kokoro" {
                            let sample_dir = store.root().join("samples");
                            std::fs::create_dir_all(&sample_dir)?;
                            let sample = sample_dir.join("hello.wav");
                            std::fs::copy(&response.output_path, &sample)?;
                            spoken_sample = Some(sample);
                        }
                        rows.push(FastSuiteRow::passed(
                            id,
                            Some(response.output_path),
                            None,
                            started.elapsed().as_millis(),
                            store,
                        ));
                    }
                    Err(error) => rows.push(FastSuiteRow::failed(
                        id,
                        error.to_string(),
                        started.elapsed().as_millis(),
                        store,
                    )),
                }
            }
            _ => {
                let Some(sample) = spoken_sample.clone() else {
                    rows.push(FastSuiteRow::failed(id, "Kokoro did not produce the required spoken hello.wav; refusing to transcribe silence".to_string(), started.elapsed().as_millis(), store));
                    continue;
                };
                let plan =
                    resolve_execution_plan(registry, installed, id, CapabilityKind::SpeechToText)
                        .map_err(cli_error)?;
                match execute_transcription(
                    &plan,
                    takokit_core::TranscriptionRequest {
                        file_path: sample.clone(),
                        model: Some(id.to_string()),
                    },
                )
                .await
                {
                    Ok(response) if response.text.trim().is_empty() => {
                        rows.push(FastSuiteRow::failed(
                            id,
                            "transcript was empty".to_string(),
                            started.elapsed().as_millis(),
                            store,
                        ))
                    }
                    Ok(response) => rows.push(FastSuiteRow::passed(
                        id,
                        Some(sample),
                        Some(response.text),
                        started.elapsed().as_millis(),
                        store,
                    )),
                    Err(error) => rows.push(FastSuiteRow::failed(
                        id,
                        error.to_string(),
                        started.elapsed().as_millis(),
                        store,
                    )),
                }
            }
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!("Fast smoke test");
        for row in &rows {
            println!("{:<13} {:<7} {}", row.model, row.status, row.detail());
        }
    }
    if run && rows.iter().any(|row| row.status == "failed") {
        return Err(anyhow::anyhow!(
            "fast suite has executable model failures; inspect the JSON rows and logs"
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct FastSuiteRow {
    model: String,
    status: String,
    output_path: Option<PathBuf>,
    transcript: Option<String>,
    duration_ms: Option<u128>,
    log_path: PathBuf,
    error: Option<String>,
}
impl FastSuiteRow {
    fn skipped(model: &str, reason: String, store: &LocalStore) -> Self {
        Self {
            model: model.to_string(),
            status: "skipped".to_string(),
            output_path: None,
            transcript: None,
            duration_ms: None,
            log_path: store.logs_dir(),
            error: Some(reason),
        }
    }
    fn ready(model: &str, store: &LocalStore) -> Self {
        Self {
            model: model.to_string(),
            status: "ready".to_string(),
            output_path: None,
            transcript: None,
            duration_ms: None,
            log_path: store.logs_dir(),
            error: None,
        }
    }
    fn passed(
        model: &str,
        output_path: Option<PathBuf>,
        transcript: Option<String>,
        duration_ms: u128,
        store: &LocalStore,
    ) -> Self {
        Self {
            model: model.to_string(),
            status: "passed".to_string(),
            output_path,
            transcript,
            duration_ms: Some(duration_ms),
            log_path: store.logs_dir(),
            error: None,
        }
    }
    fn failed(model: &str, error: String, duration_ms: u128, store: &LocalStore) -> Self {
        Self {
            model: model.to_string(),
            status: "failed".to_string(),
            output_path: None,
            transcript: None,
            duration_ms: Some(duration_ms),
            log_path: store.logs_dir(),
            error: Some(error),
        }
    }
    fn detail(&self) -> String {
        self.error
            .clone()
            .or_else(|| {
                self.output_path
                    .as_ref()
                    .map(|path| format!("output={}", path.display()))
            })
            .or_else(|| {
                self.transcript
                    .as_ref()
                    .map(|text| format!("transcript={text:?}"))
            })
            .unwrap_or_default()
    }
}

async fn print_launch_suite(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    json: bool,
    run: bool,
) -> anyhow::Result<()> {
    let ids = [
        "piper-lessac",
        "kokoro",
        "whisper-base",
        "whisper-tiny",
        "qwen3-tts",
        "chatterbox",
        "f5-tts",
        "sensevoice",
        "parakeet",
        "canary",
        "openvoice",
        "rvc",
    ];
    let mut rows = Vec::new();
    for id in ids {
        match plan_model(package_registry, installed_registry, id) {
            Ok(plan) => rows.push(LaunchSuiteRow {
                model: plan.model_id,
                task: Some(plan.task),
                runner: Some(plan.required_runner),
                lifecycle: Some(plan.lifecycle_state.to_string()),
                artifacts: Some(plan.artifact_state.to_string()),
                runner_runtime: Some(plan.runner_runtime_state.to_string()),
                executable: Some(plan.executable),
                missing: plan.missing,
                next_command: Some(plan.next_command),
                run_result: None,
                error: None,
            }),
            Err(error) => rows.push(LaunchSuiteRow {
                model: id.to_string(),
                task: None,
                runner: None,
                lifecycle: None,
                artifacts: None,
                runner_runtime: None,
                executable: None,
                missing: Vec::new(),
                next_command: None,
                run_result: None,
                error: Some(error.to_string()),
            }),
        }
    }
    if run {
        run_launch_smokes(package_registry, installed_registry, &mut rows).await;
    }
    println!("{}", format_launch_suite(&rows, json)?);
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct LaunchSuiteRow {
    model: String,
    task: Option<String>,
    runner: Option<String>,
    lifecycle: Option<String>,
    artifacts: Option<String>,
    runner_runtime: Option<String>,
    executable: Option<bool>,
    missing: Vec<String>,
    next_command: Option<String>,
    run_result: Option<String>,
    error: Option<String>,
}

async fn run_launch_smokes(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    rows: &mut [LaunchSuiteRow],
) {
    for row in rows {
        if !row.executable.unwrap_or(false) {
            row.run_result =
                Some("skipped: model is blocked by its recorded lifecycle state".to_string());
            continue;
        }
        match row.model.as_str() {
            "kokoro" | "qwen3-tts" => {
                let result = match resolve_execution_plan(
                    package_registry,
                    installed_registry,
                    &row.model,
                    CapabilityKind::TextToSpeech,
                )
                .map_err(cli_error)
                {
                    Ok(plan) => execute_speech(
                        &plan,
                        SpeechRequest {
                            model: row.model.clone(),
                            input: "Takokit launch smoke test.".to_string(),
                            voice: None,
                            response_format: Some("wav".to_string()),
                        },
                        &installed_registry.storage_root().join("outputs"),
                    )
                    .await
                    .map_err(runtime_error),
                    Err(error) => Err(error),
                };
                row.run_result = Some(match result {
                    Ok(response) => format!(
                        "passed: {} ({} bytes, {} Hz)",
                        response.output_path.display(),
                        response.bytes,
                        response
                            .sample_rate
                            .map(|rate| rate.to_string())
                            .unwrap_or_else(|| "unknown sample rate".to_string())
                    ),
                    Err(error) => format!("failed: {error}"),
                });
            }
            "whisper-base" | "whisper-tiny" => {
                let output_dir = installed_registry.storage_root().join("outputs");
                let sample = output_dir.join("launch-whisper-silence.wav");
                let result = write_silence_wav(&sample, 500, WavSpec::default())
                    .map_err(anyhow::Error::from)
                    .and_then(|_| {
                        resolve_execution_plan(
                            package_registry,
                            installed_registry,
                            &row.model,
                            CapabilityKind::SpeechToText,
                        )
                        .map_err(cli_error)
                    });
                let result = match result {
                    Ok(plan) => execute_transcription(
                        &plan,
                        takokit_core::TranscriptionRequest {
                            file_path: sample,
                            model: Some(row.model.clone()),
                        },
                    )
                    .await
                    .map(|response| {
                        format!(
                            "passed: whisper.cpp completed; transcript={:?}",
                            response.text
                        )
                    })
                    .map_err(runtime_error),
                    Err(error) => Err(error),
                };
                row.run_result = Some(match result {
                    Ok(result) => result,
                    Err(error) => format!("failed: {error}"),
                });
            }
            _ => {
                row.run_result = Some(
                    "skipped: no smoke handler is declared for this executable model".to_string(),
                );
            }
        }
    }
}

fn format_launch_suite(rows: &[LaunchSuiteRow], json: bool) -> anyhow::Result<String> {
    if json {
        return Ok(serde_json::to_string_pretty(rows)?);
    }

    let mut output = String::from("Launch test suite\n");
    for row in rows {
        if let Some(error) = &row.error {
            output.push_str(&format!("- {}: error: {error}\n", row.model));
            continue;
        }

        output.push_str(&format!(
            "- {}: lifecycle={}, runner={}, executable={}\n",
            row.model,
            row.lifecycle.as_deref().unwrap_or("unknown"),
            row.runner.as_deref().unwrap_or("unknown"),
            yes_no(row.executable.unwrap_or(false))
        ));
        if !row.missing.is_empty() {
            output.push_str(&format!("  missing: {}\n", row.missing.join("; ")));
        }
        if let Some(next) = &row.next_command {
            output.push_str(&format!("  next: {next}\n"));
        }
        if let Some(result) = &row.run_result {
            output.push_str(&format!("  run: {result}\n"));
        }
    }
    Ok(output.trim_end().to_string())
}

fn print_or_json_plan(plan: &ModelPlan, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(plan)?);
    } else {
        print_model_plan(plan);
    }
    Ok(())
}

fn print_models(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let models: Vec<_> = package_registry
        .models()
        .map_err(cli_error)?
        .into_iter()
        .map(|model| {
            model_info_from_plan(package_registry, installed_registry, &model.id).map_err(cli_error)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    println!("{}", serde_json::to_string_pretty(&models)?);
    Ok(())
}

fn print_runners(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let runners: Vec<_> = package_registry
        .runners()
        .map_err(cli_error)?
        .into_iter()
        .map(|runner| runner.to_runner_info(installed_registry.is_runner_installed(&runner.id)))
        .map(|info| {
            if let Ok(record) = installed_registry.installed_runner_record(&info.id) {
                let manifest = package_registry
                    .runner(&info.id)
                    .expect("runner listed by registry is readable");
                manifest.to_runner_info_with_state(true, record.status)
            } else {
                info
            }
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&runners)?);
    Ok(())
}

fn print_library_models(package_registry: &PackageRegistry) -> anyhow::Result<()> {
    let models = package_registry.library_models().map_err(cli_error)?;
    println!("{}", serde_json::to_string_pretty(&models)?);
    Ok(())
}

fn print_library_runners(package_registry: &PackageRegistry) -> anyhow::Result<()> {
    let runners = package_registry.library_runners().map_err(cli_error)?;
    println!("{}", serde_json::to_string_pretty(&runners)?);
    Ok(())
}

fn print_model_plan(plan: &ModelPlan) {
    println!("Model: {} ({})", plan.model_name, plan.model_id);
    println!("Task: {}", plan.task);
    println!("Runner: {}", plan.required_runner);
    println!("Lifecycle: {:?}", plan.lifecycle_state);
    println!("Artifacts: {:?}", plan.artifact_state);
    println!("Runner contract: {:?}", plan.runner_contract_state);
    println!("Runner runtime: {:?}", plan.runner_runtime_state);
    println!("Executable today: {}", yes_no(plan.executable));
    if plan.missing.is_empty() {
        println!("Missing: none");
    } else {
        println!("Missing: {}", plan.missing.join("; "));
    }
    println!("Next command: {}", plan.next_command);
}

fn print_runner_doctor(
    store: &LocalStore,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) {
    println!("Runner Doctor: {} ({})", manifest.name, manifest.id);
    println!(
        "contract manifest: {}",
        yes_no(installed_registry.is_runner_installed(&manifest.id))
    );
    match installed_registry.installed_runner_record(&manifest.id) {
        Ok(record) => {
            println!("runtime state: {:?}", record.status);
            println!("recorded at: {}", record.installed_at);
            println!("note: {}", record.note);
        }
        Err(_) => println!("runtime state: RuntimeMissing"),
    }
    let layout = runner_runtime_layout(store.root(), manifest);
    println!("runtime path: {}", layout.root.display());
    println!("logs path: {}", layout.logs.display());
    if manifest.id == "takokit-onnx" {
        let ready = installed_registry
            .installed_runner_record(&manifest.id)
            .map(|record| record.status == takokit_package::RunnerLifecycleState::Ready)
            .unwrap_or(false);
        println!(
            "ONNX session capability: {}",
            if ready {
                "kokoro-onnx-ready"
            } else {
                "not-installed"
            }
        );
        println!("Piper frontend status: piper_text_frontend_not_implemented");
        println!(
            "executable models: {}",
            if ready { "kokoro" } else { "none" }
        );
    }
    if manifest.id == "takokit-python-managed" {
        match python_adapter_records(store.root()) {
            Ok(records) if !records.is_empty() => {
                println!("adapters:");
                for record in records {
                    println!("- {}: {}", record.id, record.state);
                }
            }
            _ => println!("adapters: run `takokit runner install takokit-python-managed`"),
        }
    }
}

fn print_runner_doctor_json(
    store: &LocalStore,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) -> anyhow::Result<()> {
    let layout = runner_runtime_layout(store.root(), manifest);
    let record = installed_registry
        .installed_runner_record(&manifest.id)
        .ok();
    let adapters = if manifest.id == "takokit-python-managed" {
        python_adapter_records(store.root()).unwrap_or_default()
    } else {
        Vec::new()
    };
    let executable_models = if manifest.id == "takokit-onnx" {
        vec!["kokoro"]
    } else {
        Vec::new()
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "id": manifest.id,
            "name": manifest.name,
            "contract_installed": installed_registry.is_runner_installed(&manifest.id),
            "runtime_state": record
                .as_ref()
                .map(|record| record.status.to_string())
                .unwrap_or_else(|| "runtime-missing".to_string()),
            "note": record.as_ref().map(|record| record.note.clone()),
            "runtime_path": layout.root,
            "logs_path": layout.logs,
            "adapters": adapters,
            "onnx_session_capability": if manifest.id == "takokit-onnx" && record.as_ref().is_some_and(|item| item.status == takokit_package::RunnerLifecycleState::Ready) { Some("kokoro-onnx-ready") } else { None },
            "piper_frontend_status": if manifest.id == "takokit-onnx" { Some("piper_text_frontend_not_implemented") } else { None },
            "executable_models": executable_models,
        }))?
    );
    Ok(())
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn not_implemented(feature: &'static str, reason: &'static str) -> anyhow::Result<()> {
    Err(TakokitError::NotImplemented { feature, reason }.into())
}

fn cli_error(error: PackageError) -> anyhow::Error {
    runtime_error(TakokitError::from(error))
}

fn runtime_error(error: TakokitError) -> anyhow::Error {
    match error {
        TakokitError::Resolution { code, message } => {
            anyhow::anyhow!("{}: {}", code.as_str(), message)
        }
        error => error.into(),
    }
}

fn capability_labels(capabilities: &[CapabilityKind]) -> String {
    if capabilities.is_empty() {
        return "none".to_string();
    }

    capabilities
        .iter()
        .map(|capability| capability.label())
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_model_show(
    info: &ModelInfo,
    installed_record: Option<&takokit_package::InstalledModelRecord>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("{} ({})", info.name, info.id));
    lines.push(format!("family: {}", info.family));
    lines.push(format!("version: {}", info.version));
    lines.push(format!("backend: {}", info.backend));
    lines.push(format!("runner: {}", info.runner));
    lines.push(format!("installed: {}", info.installed));
    lines.push(format!("runner installed: {}", info.runner_installed));
    lines.push(format!("runner runtime: {}", info.runner_runtime_state));
    lines.push(format!("lifecycle: {}", info.lifecycle_state));
    lines.push(format!("status: {}", info.execution_status));
    lines.push(format!("executable today: {}", yes_no(info.executable)));
    lines.push(format!("license: {}", info.license));
    if let Some(warning) = &info.license_warning {
        lines.push(format!("license warning: {warning}"));
    }
    lines.push(format!(
        "capabilities: {}",
        capability_labels(&info.capabilities)
    ));
    lines.push(format!("hardware: {}", info.hardware_notes));
    lines.push(format!("artifacts: {}", info.artifact_count));
    if let Some(record) = installed_record {
        lines.push(format!("installed status: {:?}", record.status));
        lines.push(format!("installed at: {}", record.installed_at));
        lines.push(format!("source: {}", record.source));
        lines.push(format!("installed artifacts: {}", record.artifacts.len()));
    } else {
        lines.push("installed status: not installed".to_string());
    }
    if info.missing.is_empty() {
        lines.push("missing: none".to_string());
    } else {
        lines.push(format!("missing: {}", info.missing.join("; ")));
    }
    lines.push(format!("next command: {}", info.next_command));
    lines.push(format!("description: {}", info.summary));
    lines.join("\n")
}

fn format_runner_show(
    manifest: &RunnerManifest,
    installed: bool,
    runtime_state: Option<takokit_package::RunnerLifecycleState>,
    note: Option<String>,
    runtime_path: PathBuf,
) -> String {
    let runtime_state_label = runtime_state
        .map(|state| state.to_string())
        .unwrap_or_else(|| "runtime-missing".to_string());
    let mut lines = Vec::new();
    lines.push(format!("{} ({})", manifest.name, manifest.id));
    lines.push(format!("version: {}", manifest.version));
    lines.push(format!("kind: {:?}", manifest.kind));
    lines.push(format!("platforms: {}", manifest.platforms.join(", ")));
    lines.push(format!(
        "model families: {}",
        if manifest.supported_model_families.is_empty() {
            "none declared".to_string()
        } else {
            manifest.supported_model_families.join(", ")
        }
    ));
    lines.push(format!(
        "tasks: {}",
        capability_labels(&manifest.supported_tasks)
    ));
    lines.push(format!(
        "dependency strategy: {:?}",
        manifest.dependency_strategy
    ));
    lines.push(format!("installed: {}", installed));
    lines.push(format!("runtime state: {runtime_state_label}"));
    lines.push(format!("runtime path: {}", runtime_path.display()));
    lines.push(format!(
        "status: {}",
        runner_status_text(manifest, runtime_state)
    ));
    if manifest.id == "takokit-python-managed" {
        lines.push(
            "user setup: Takokit manages Python, packages, wheels, caches, and logs internally."
                .to_string(),
        );
    }
    if let Some(note) = note {
        lines.push(format!("installed note: {note}"));
    }
    if !manifest.notes.is_empty() {
        lines.push(format!("notes: {}", manifest.notes));
    }
    lines.push(format!("description: {}", manifest.description));
    lines.join("\n")
}

fn runner_status_text(
    manifest: &RunnerManifest,
    runtime_state: Option<takokit_package::RunnerLifecycleState>,
) -> String {
    match runtime_state {
        Some(takokit_package::RunnerLifecycleState::Ready) => "ready".to_string(),
        Some(takokit_package::RunnerLifecycleState::RuntimeInstalled)
            if manifest.id == "takokit-onnx" =>
        {
            "runtime installed; missing Piper text frontend and ONNX TTS execution verification"
                .to_string()
        }
        Some(state) => state.to_string(),
        None => "runtime missing".to_string(),
    }
}

#[cfg(test)]
mod tests {
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
    fn cli_parses_library_model_and_runner_commands() {
        let models =
            Cli::try_parse_from(["takokit", "library", "models"]).expect("library models command");
        let runners = Cli::try_parse_from(["takokit", "library", "runners"])
            .expect("library runners command");

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
        let suite = Cli::try_parse_from(["takokit", "test", "--suite", "launch", "--run"])
            .expect("launch run");

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
        let quickstart =
            Cli::try_parse_from(["takokit", "quickstart", "--full"]).expect("quickstart");
        let deps = Cli::try_parse_from(["takokit", "deps", "bootstrap"]).expect("deps");
        let samples = Cli::try_parse_from(["takokit", "samples", "create"]).expect("samples");
        let fast = Cli::try_parse_from(["takokit", "test", "--suite", "fast", "--run"])
            .expect("fast suite");

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
        let suite =
            Cli::try_parse_from(["takokit", "test", "--suite", "launch"]).expect("suite test");

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
            message:
                "ONNX runner contract resolved, but real ONNX execution is not implemented yet."
                    .to_string(),
        });

        assert_eq!(
            error.to_string(),
            "inference_not_implemented: ONNX runner contract resolved, but real ONNX execution is not implemented yet."
        );
    }
}
