mod doctor;
mod gui;
mod tui;

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use takokit_core::{CapabilityKind, RuntimeConfig, SpeechRequest, TakokitError};
use takokit_models::{
    execute_speech, execute_transcription, MockTextToSpeechEngine, ModelRegistry,
    TextToSpeechEngine,
};
use takokit_package::{
    resolve_execution_plan, InstallModelOptions, InstalledRegistry, PackageError, PackageRegistry,
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
    Doctor,
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
struct PullArgs {
    model: String,
    #[arg(long)]
    metadata_only: bool,
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
    Pull { runner: String },
    Show { runner: String },
    Rm { runner: String },
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
        Some(Command::Doctor) => {
            let report =
                doctor::run_doctor(&config, &store, &package_registry, &installed_registry);
            doctor::print_report(&report);
            if report.has_failures() {
                std::process::exit(1);
            }
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
                println!("artifacts: {}", manifest.artifacts.all().count());
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
        Some(Command::Runner { command }) => {
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

fn print_models(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
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

fn execution_status(installed: bool, runner_installed: bool) -> &'static str {
    match (installed, runner_installed) {
        (false, _) => "model manifest is available, but the model is not installed",
        (true, false) => "model installed; required runner is not installed or not implemented yet",
        (true, true) => "runner installed; real inference is not implemented yet",
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

        assert!(matches!(cli.command, Some(Command::Doctor)));
    }

    #[test]
    fn tako_alias_parses_doctor_and_uses_takokit_storage_root() {
        let cli = Cli::try_parse_from(["tako", "doctor"]).expect("tako doctor cli parse");
        let storage_root = cli_storage_root();

        assert!(matches!(cli.command, Some(Command::Doctor)));
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
