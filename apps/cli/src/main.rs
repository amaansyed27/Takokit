use clap::{Args, Parser, Subcommand};
use std::{path::PathBuf, process::Stdio, time::Duration};
use takokit_core::{CapabilityKind, RuntimeConfig, SpeechRequest, TakokitError};
use takokit_models::{MockTextToSpeechEngine, ModelRegistry, TextToSpeechEngine};
use takokit_package::{resolve_runner, InstalledRegistry, PackageError, PackageRegistry};
use takokit_server::{run_server, AppState};
use takokit_store::LocalStore;
use tokio::net::TcpStream;

#[derive(Debug, Parser)]
#[command(name = "takokit", version, about = "Local voice AI runtime")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve,
    Gui,
    Status,
    Capabilities,
    Speak(SpeakArgs),
    Pull {
        model: String,
    },
    Show {
        model: String,
    },
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
enum RunnerCommand {
    Pull { runner: String },
    Show { runner: String },
    Rm { runner: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let store = LocalStore::new(LocalStore::default_root());
    store.ensure_layout()?;
    let config = RuntimeConfig::local(store.root().to_path_buf());
    let package_registry = PackageRegistry::bundled();
    let installed_registry = InstalledRegistry::new(store.manifests_dir());

    match cli.command {
        Command::Serve => {
            run_server(AppState::new(config, store)).await?;
        }
        Command::Gui => open_gui(&config).await?,
        Command::Status => {
            let state = AppState::new(config, store);
            println!("{}", serde_json::to_string_pretty(&state.status())?);
        }
        Command::Capabilities => {
            for capability in CapabilityKind::ALL {
                println!("{} - {}", capability.label(), capability.explanation());
            }
        }
        Command::Speak(args) => {
            if args.model != "mock-tts" {
                let plan = resolve_runner(
                    &package_registry,
                    &installed_registry,
                    &args.model,
                    CapabilityKind::TextToSpeech,
                )
                .map_err(cli_error)?;

                return Err(cli_error(PackageError::InferenceNotImplemented {
                    model: plan.model.id,
                    runner: plan.runner.id,
                    capability: plan.capability,
                    capability_label: plan.capability.label(),
                }));
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
        Command::Pull { model } => {
            let manifest = package_registry.model(&model).map_err(cli_error)?;
            let report = installed_registry
                .install_model(&manifest)
                .map_err(cli_error)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Show { model } => {
            let manifest = package_registry.model(&model).map_err(cli_error)?;
            let installed = installed_registry.is_model_installed(&manifest.id);
            let runner_installed = installed_registry.is_runner_installed(&manifest.runner);
            let runner = package_registry.runner(&manifest.runner).ok();
            println!("{} ({})", manifest.name, manifest.id);
            println!("version: {}", manifest.version);
            println!("backend: {:?}", manifest.backend);
            println!("runner: {}", manifest.runner);
            if let Some(runner) = runner {
                println!("runner kind: {:?}", runner.kind);
                println!("runner installed: {}", runner_installed);
            }
            println!("installed: {}", installed);
            if let Ok(record) = installed_registry.installed_model_record(&manifest.id) {
                println!("installed status: {:?}", record.status);
                println!("installed at: {}", record.installed_at);
                println!("source: {}", record.source);
                println!("artifacts: {}", record.artifacts.len());
            } else {
                println!("installed status: not installed");
                println!(
                    "artifacts: {}",
                    manifest.artifacts.weights.len() + manifest.artifacts.voices.len()
                );
            }
            println!("license: {}", manifest.license);
            println!(
                "capabilities: {}",
                capability_labels(&manifest.capabilities.to_model_capabilities())
            );
            println!(
                "hardware: cpu={}, gpu={}, min_ram={}",
                manifest.hardware.cpu,
                manifest.hardware.gpu,
                manifest
                    .hardware
                    .min_ram
                    .as_deref()
                    .unwrap_or("unspecified")
            );
            println!("status: {}", execution_status(installed, runner_installed));
            println!("description: {}", manifest.description);
        }
        Command::Rm { model } => {
            let removed = installed_registry.remove_model(&model).map_err(cli_error)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "id": model,
                    "removed": removed
                }))?
            );
        }
        Command::List { target } => {
            let registry = ModelRegistry::default();
            match target {
                ListTarget::Models => {
                    let models: Vec<_> = package_registry
                        .models()
                        .map_err(cli_error)?
                        .into_iter()
                        .map(|model| {
                            model.to_model_info(
                                installed_registry.is_model_installed(&model.id),
                                installed_registry.is_runner_installed(&model.runner),
                            )
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&models)?)
                }
                ListTarget::Runners => {
                    let runners: Vec<_> = package_registry
                        .runners()
                        .map_err(cli_error)?
                        .into_iter()
                        .map(|runner| {
                            runner
                                .to_runner_info(installed_registry.is_runner_installed(&runner.id))
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&runners)?)
                }
                ListTarget::Voices => {
                    println!("{}", serde_json::to_string_pretty(registry.voices())?)
                }
            }
        }
        Command::Transcribe { audio: _, model } => {
            let plan = resolve_runner(
                &package_registry,
                &installed_registry,
                &model,
                CapabilityKind::SpeechToText,
            )
            .map_err(cli_error)?;

            return Err(cli_error(PackageError::InferenceNotImplemented {
                model: plan.model.id,
                runner: plan.runner.id,
                capability: plan.capability,
                capability_label: plan.capability.label(),
            }));
        }
        Command::Clone(args) => {
            let _ = args;
            return not_implemented(
                "voice cloning",
                "clone adapters require explicit model runner integration",
            );
        }
        Command::Train(args) => {
            let _ = args;
            return not_implemented(
                "voice training",
                "training jobs and dataset preparation are planned for a later phase",
            );
        }
        Command::Runner { command } => {
            match command {
                RunnerCommand::Pull { runner } => {
                    let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                    let report = installed_registry
                        .install_runner(&manifest)
                        .map_err(cli_error)?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                RunnerCommand::Show { runner } => {
                    let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                    let installed = installed_registry.is_runner_installed(&manifest.id);
                    println!("{} ({})", manifest.name, manifest.id);
                    println!("version: {}", manifest.version);
                    println!("kind: {:?}", manifest.kind);
                    println!("platforms: {}", manifest.platforms.join(", "));
                    println!("installed: {}", installed);
                    if let Ok(record) = installed_registry.installed_runner_record(&manifest.id) {
                        println!("installed status: {:?}", record.status);
                        println!("installed at: {}", record.installed_at);
                    } else {
                        println!("installed status: not installed");
                    }
                    println!("status: runner contract installed only; execution binary is not implemented");
                    println!("description: {}", manifest.description);
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
            }
        }
    }

    Ok(())
}

fn not_implemented(feature: &'static str, reason: &'static str) -> anyhow::Result<()> {
    Err(TakokitError::NotImplemented { feature, reason }.into())
}

fn cli_error(error: PackageError) -> anyhow::Error {
    match TakokitError::from(error) {
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

fn execution_status(installed: bool, runner_installed: bool) -> &'static str {
    match (installed, runner_installed) {
        (false, _) => "model manifest is available, but the model is not installed",
        (true, false) => "model installed; required runner is not installed or not implemented yet",
        (true, true) => "runner installed; real inference is not implemented yet",
    }
}

async fn open_gui(config: &RuntimeConfig) -> anyhow::Result<()> {
    if !server_available(config).await {
        start_server_process()?;
        wait_for_server(config).await?;
    }

    let url = config.gui_url();
    match open::that(&url) {
        Ok(()) => println!("Opened Takokit local web GUI at {url}"),
        Err(error) => {
            println!("Takokit local web GUI: {url}");
            eprintln!("Could not open the browser automatically: {error}");
        }
    }

    Ok(())
}

async fn server_available(config: &RuntimeConfig) -> bool {
    TcpStream::connect(config.bind_addr()).await.is_ok()
}

async fn wait_for_server(config: &RuntimeConfig) -> anyhow::Result<()> {
    for _ in 0..20 {
        if server_available(config).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    Err(TakokitError::Storage(format!(
        "Takokit server did not become available at {}",
        config.local_base_url()
    ))
    .into())
}

fn start_server_process() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe)
        .arg("serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}
