mod daemon;
mod daemon_client;
mod display;
mod doctor;
mod gui;
mod tui;

use clap::Parser;
use display::format_model_show;
use serde::Serialize;
use std::{path::PathBuf, time::Instant};
use takokit_audio::{write_silence_wav, WavSpec};
use takokit_core::{CapabilityKind, RuntimeConfig, SpeechRequest, TakokitError};
use takokit_models::{
    execute_speech, execute_transcription, MockTextToSpeechEngine, ModelRegistry,
    TextToSpeechEngine,
};
use takokit_package::{
    bootstrap_uv, find_uv, initialize_runner_runtime, install_model_complete,
    install_python_adapter, model_info_from_plan, plan_model, python_adapter_record,
    python_adapter_records, resolve_execution_plan, runner_runtime_layout, InstallModelOptions,
    InstalledRegistry, ModelPlan, PackageError, PackageRegistry, RunnerManifest,
};
use takokit_server::{run_server, AppState};
use takokit_store::LocalStore;

mod args;
mod daemon_commands;
mod local_setup;
mod output;
mod test_commands;

use args::*;
use daemon_commands::*;
use local_setup::*;
use output::*;
use test_commands::*;

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

    if !cli.direct && route_daemon_command(&cli.command, &store, &config).await? {
        return Ok(());
    }

    match cli.command {
        None => tui::run_launcher(&config, &store, &package_registry, &installed_registry).await?,
        Some(Command::Serve {
            daemon_child,
            instance_id,
        }) => {
            if daemon_child {
                daemon::child(
                    store,
                    config,
                    instance_id.ok_or_else(|| {
                        anyhow::anyhow!("managed daemon child requires --instance-id")
                    })?,
                )
                .await?;
            } else {
                run_server(AppState::new(config, store)).await?;
            }
        }
        Some(Command::Daemon { command }) => match command {
            DaemonCommand::Start => println!(
                "{}",
                serde_json::to_string_pretty(&daemon::start(&store, &config)?)?
            ),
            DaemonCommand::Stop => println!("stopped: {}", daemon::stop(&store, &config)?),
            DaemonCommand::Restart => {
                let _ = daemon::stop(&store, &config)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&daemon::start(&store, &config)?)?
                );
            }
            DaemonCommand::Status => match daemon::status(&store, &config)? {
                Some(info) => println!("{}", serde_json::to_string_pretty(&info)?),
                None => println!("not running"),
            },
            DaemonCommand::Logs => println!("{}", daemon::logs(&store).display()),
        },
        Some(Command::Gui) => gui::open_gui(&store, &config).await?,
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
            let report = install_model_complete(
                &package_registry,
                &installed_registry,
                store.root(),
                &args.model,
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
                None | Some(ListTarget::Models) => {
                    print_models(&package_registry, &installed_registry)?
                }
                Some(ListTarget::Runners) => print_runners(&package_registry, &installed_registry)?,
                Some(ListTarget::Voices) => {
                    println!("{}", serde_json::to_string_pretty(registry.voices())?)
                }
            }
        }
        Some(Command::Run(args)) => {
            validate_run_args(&args)?;
            let manifest = package_registry.model(&args.model).map_err(cli_error)?;
            if args.text.is_some() && !manifest.capabilities.tts {
                return Err(anyhow::anyhow!(
                    "model {} does not support text to speech",
                    args.model
                ));
            }
            if args.file.is_some() && !manifest.capabilities.stt {
                return Err(anyhow::anyhow!(
                    "model {} does not support speech to text",
                    args.model
                ));
            }
            if let Some(text) = args.text {
                let response = if cli.direct && args.model != "mock-tts" {
                    let plan = resolve_execution_plan(
                        &package_registry,
                        &installed_registry,
                        &args.model,
                        CapabilityKind::TextToSpeech,
                    )
                    .map_err(cli_error)?;
                    execute_speech(
                        &plan,
                        SpeechRequest {
                            model: args.model,
                            input: text,
                            voice: args.voice,
                            response_format: Some("wav".to_string()),
                        },
                        &store.outputs_dir(),
                    )
                    .await
                    .map_err(runtime_error)?
                } else if cli.direct {
                    MockTextToSpeechEngine
                        .synthesize(
                            SpeechRequest {
                                model: args.model,
                                input: text,
                                voice: args.voice,
                                response_format: Some("wav".to_string()),
                            },
                            &store.outputs_dir(),
                        )
                        .await?
                } else {
                    daemon_client::Client::ensure(&store, &config)?.speech(SpeechRequest {
                        model: args.model,
                        input: text,
                        voice: args.voice,
                        response_format: Some("wav".to_string()),
                    })?
                };
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else if cli.direct {
                let model = args.model;
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
                        file_path: args.file.unwrap(),
                        model: Some(model),
                    },
                )
                .await
                .map_err(runtime_error)?;
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                let response: takokit_core::TranscriptionResponse =
                    daemon_client::Client::ensure(&store, &config)?.post(
                        "/v1/audio/transcriptions",
                        &takokit_core::TranscriptionRequest {
                            file_path: args.file.unwrap(),
                            model: Some(args.model),
                        },
                    )?;
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
        }
        Some(Command::Ps) => {
            if cli.direct {
                println!("[]");
            } else {
                let value: serde_json::Value =
                    daemon_client::Client::ensure(&store, &config)?.get("/v1/ps")?;
                println!("{}", serde_json::to_string_pretty(&value)?);
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

#[cfg(test)]
mod tests;
