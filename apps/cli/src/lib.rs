mod daemon;
mod daemon_client;
mod direct_inference;
mod display;
mod distribution;
mod doctor;
mod gui;
mod license_command;
mod reset_command;
mod session_commands;
mod storage_command;
mod tui;
mod update_command;
mod workspace;

use clap::Parser;
use display::format_model_show;
use serde::Serialize;
use std::{path::PathBuf, time::Instant};
use takokit_audio::{write_silence_wav, WavSpec};
use takokit_core::{CapabilityKind, RuntimeConfig, TakokitError};
use takokit_models::{execute_speech, execute_transcription, MockTextToSpeechEngine, TextToSpeechEngine};
use takokit_package::{
    acquire_maintenance_lock, bootstrap_uv, custom_model_record, custom_model_records, find_uv,
    initialize_runner_runtime, install_model_complete, install_python_adapter,
    model_info_from_plan, plan_model, python_adapter_record, python_adapter_records,
    register_custom_model, remove_custom_model, remove_model_complete, require_custom_model_id,
    resolve_execution_plan, runner_runtime_layout, InstallModelOptions, InstalledRegistry,
    ModelPlan, PackageError, PackageRegistry, RemoveModelOptions, RunnerManifest,
};
use takokit_server::{run_server, AppState};
use takokit_store::{LocalStore, VoiceProfileStore};

mod args;
mod daemon_commands;
mod local_setup;
mod output;
mod rvc_voice_command;
mod test_commands;
mod voice_command;

use args::*;
use daemon_commands::*;
use direct_inference::*;
use license_command::*;
use local_setup::*;
use output::*;
use session_commands::*;
use storage_command::run_storage_command;
use test_commands::*;
use workspace::CliWorkspace;

fn cli_storage_root() -> PathBuf {
    LocalStore::default_root()
}

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    distribution::configure_installed_resources();

    let Cli {
        direct,
        output,
        workspace: workspace_arg,
        session: session_arg,
        command,
    } = Cli::parse();
    if let Some(output) = output {
        set_json_output(matches!(output, OutputFormat::Json));
    }
    let store = LocalStore::new(cli_storage_root());
    store.ensure_layout()?;
    let config = RuntimeConfig::local(store.root().to_path_buf());
    let package_registry = PackageRegistry::bundled();
    let installed_registry = InstalledRegistry::new(store.manifests_dir());
    let workspace = if command_uses_workspace(&command) {
        Some(CliWorkspace::resolve(
            workspace_arg.clone(),
            session_arg,
            starts_new_session(&command),
            surface_title(&command),
        )?)
    } else {
        None
    };

    if let Some(Command::Pull(args)) = command.as_ref() {
        prepare_pull_license(
            store.root(),
            &package_registry,
            args,
            json_output_requested(),
        )?;
    }

    if !direct && route_daemon_command(&command, &store, &config).await? {
        return Ok(());
    }

    match command {
        None => {
            tui::run_launcher(
                &config,
                &store,
                &package_registry,
                &installed_registry,
                workspace.as_ref().expect("TUI workspace"),
            )
            .await?
        }
        Some(Command::Serve { daemon_child, instance_id }) => {
            if daemon_child {
                daemon::child(
                    store,
                    config,
                    instance_id.ok_or_else(|| anyhow::anyhow!("managed daemon child requires --instance-id"))?,
                )
                .await?;
            } else {
                run_server(AppState::new_with_build_id(
                    config,
                    store,
                    daemon::current_build_id(),
                ))
                .await?;
            }
        }
        Some(Command::Daemon { command }) => match command {
            DaemonCommand::Start => print_serializable(&daemon::start(&store, &config)?)?,
            DaemonCommand::Stop => println!("stopped: {}", daemon::stop(&store, &config)?),
            DaemonCommand::Restart => {
                let _ = daemon::stop(&store, &config)?;
                print_serializable(&daemon::start(&store, &config)?)?;
            }
            DaemonCommand::Status => match daemon::status(&store, &config)? {
                Some(info) => print_serializable(&info)?,
                None => println!("not running"),
            },
            DaemonCommand::Logs => println!("{}", daemon::logs(&store).display()),
        },
        Some(Command::Gui) => {
            gui::open_gui(&store, &config, workspace.as_ref().expect("GUI workspace")).await?
        }
        Some(Command::Doctor(args)) => {
            let report = doctor::run_doctor(&config, &store, &package_registry, &installed_registry);
            if args.json || json_output_requested() {
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
                println!("log: {}", store.logs_dir().join("uv-bootstrap.log").display());
            }
        },
        Some(Command::Samples { command: SamplesCommand::Create }) => {
            create_samples(&store, &package_registry, &installed_registry).await?
        }
        Some(Command::Version) => {
            println!("takokit {}", env!("CARGO_PKG_VERSION"));
            println!("build: {}", daemon::current_build_id());
            println!("storage: {}", store.root().display());
            if let Some(metadata) = distribution::distribution_metadata() {
                println!("distribution: {}", metadata.mode);
            }
        }
        Some(Command::Status) => {
            let state = AppState::new_with_build_id(config, store, daemon::current_build_id());
            print_serializable(&serde_json::json!({
                "status": state.status(),
                "build_id": state.build_id,
            }))?;
        }
        Some(Command::Storage(args)) => {
            run_storage_command(store.root(), args, json_output_requested())?;
        }
        Some(Command::Update { command }) => {
            update_command::run_update_command(&store, &config, command, json_output_requested())?;
        }
        Some(Command::Reset(args)) => {
            reset_command::run_reset_command(&store, &config, args, json_output_requested())?;
        }
        Some(Command::Licenses { command }) => {
            run_license_command(store.root(), command)?;
        }
        Some(Command::Capabilities) => {
            for capability in CapabilityKind::ALL {
                println!("{} - {}", capability.label(), capability.explanation());
            }
        }
        Some(Command::Models) => print_models(&package_registry, &installed_registry)?,
        Some(Command::Runners) => print_runners(&package_registry, &installed_registry)?,
        Some(Command::CustomModel { command }) => match command {
            CustomModelCommand::Add { manifest } => {
                let record = register_custom_model(store.root(), &package_registry, &manifest).map_err(cli_error)?;
                print_serializable(&record)?;
            }
            CustomModelCommand::List => {
                print_serializable(&custom_model_records(store.root(), &package_registry).map_err(cli_error)?)?;
            }
            CustomModelCommand::Show { model } => {
                print_serializable(&custom_model_record(store.root(), &package_registry, &model).map_err(cli_error)?)?;
            }
            CustomModelCommand::Rm { model } => {
                let id = require_custom_model_id(&model).map_err(cli_error)?;
                if installed_registry.is_model_installed(&id) {
                    return Err(anyhow::anyhow!(
                        "custom model {id} is installed; run `tako rm {id}` before removing its registration"
                    ));
                }
                let removed = remove_custom_model(store.root(), &id).map_err(cli_error)?;
                print_value(&serde_json::json!({"id": id, "removed": removed}))?;
            }
        },
        Some(Command::Voice { command }) => {
            voice_command::run_voice_command(
                command,
                &store,
                &package_registry,
                &installed_registry,
                workspace.as_ref(),
            )
            .await?;
        }
        Some(Command::Library { target }) => match target {
            LibraryTarget::Models => print_library_models(&package_registry)?,
            LibraryTarget::Runners => print_library_runners(&package_registry)?,
            LibraryTarget::Sync => {
                let updated = package_registry.sync_remote().map_err(cli_error)?;
                print_value(&serde_json::json!({
                    "updated": updated,
                    "models": package_registry.registry_models().map_err(cli_error)?.len()
                }))?;
            }
            LibraryTarget::Show { model } => print_library_model(&package_registry, &model)?,
        },
        Some(Command::Speak(args)) => {
            run_speak(
                args,
                &package_registry,
                &installed_registry,
                workspace.as_ref().expect("speech workspace"),
            )
            .await?
        }
        Some(Command::Pull(args)) => {
            let report = install_model_complete(
                &package_registry,
                &installed_registry,
                store.root(),
                &args.model,
                InstallModelOptions {
                    metadata_only: args.metadata_only,
                    accepted_license: args.accept_license,
                },
            )
            .map_err(cli_error)?;
            print_serializable(&report)?;
        }
        Some(Command::Show { model }) => {
            let manifest = package_registry.model(&model).map_err(cli_error)?;
            let info = model_info_from_plan(&package_registry, &installed_registry, &manifest.id).map_err(cli_error)?;
            let installed_record = installed_registry.installed_model_record(&manifest.id).ok();
            println!("{}", format_model_show(&info, installed_record.as_ref()));
        }
        Some(Command::Plan(args)) => {
            let plan = plan_model(&package_registry, &installed_registry, &args.model).map_err(cli_error)?;
            print_or_json_plan(&plan, args.json || json_output_requested())?;
        }
        Some(Command::Rm(args)) => {
            let resolved = package_registry.resolve_model_reference(&args.model).map_err(cli_error)?;
            let report = remove_model_complete(
                &package_registry,
                &installed_registry,
                &resolved.target,
                RemoveModelOptions { dry_run: args.dry_run },
            )
            .map_err(cli_error)?;
            print_serializable(&report)?;
        }
        Some(Command::List { target }) => match target {
            None | Some(ListTarget::Models) => print_models(&package_registry, &installed_registry)?,
            Some(ListTarget::Runners) => print_runners(&package_registry, &installed_registry)?,
            Some(ListTarget::Voices) => print_serializable(&VoiceProfileStore::new(store.voices_dir()).list()?)?,
        },
        Some(Command::Run(args)) => {
            run_model(
                args,
                &package_registry,
                &installed_registry,
                workspace.as_ref().expect("run workspace"),
            )
            .await?
        }
        Some(Command::Ps) => {
            if direct {
                print_value(&serde_json::json!([]))?;
            } else {
                let value: serde_json::Value = daemon_client::Client::ensure(&store, &config)?.get("/v1/ps")?;
                print_value(&value)?;
            }
        }
        Some(Command::Transcribe { audio, model }) => {
            run_transcription(
                audio,
                model,
                &package_registry,
                &installed_registry,
                workspace.as_ref().expect("transcription workspace"),
            )
            .await?
        }
        Some(Command::Clone(args)) => run_clone(args, workspace.as_ref().expect("clone workspace"))?,
        Some(Command::Convert(args)) => {
            run_convert(
                args,
                &package_registry,
                &installed_registry,
                workspace.as_ref().expect("conversion workspace"),
            )
            .await?
        }
        Some(Command::Train(args)) => {
            run_train(
                args,
                &package_registry,
                &installed_registry,
                workspace.as_ref().expect("training workspace"),
            )
            .await?
        }
        Some(Command::Sessions { command }) => run_sessions_command(workspace_arg, command)?,
        Some(Command::Runner { command }) => match command {
            RunnerCommand::Pull { runner } => {
                let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                let report = installed_registry.install_runner(&manifest).map_err(cli_error)?;
                print_serializable(&report)?;
            }
            RunnerCommand::Install { runner } => {
                let _maintenance_guard = acquire_maintenance_lock(store.root()).map_err(cli_error)?;
                let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                let report = initialize_runner_runtime(store.root(), &installed_registry, &manifest).map_err(cli_error)?;
                print_serializable(&report)?;
            }
            RunnerCommand::Doctor { runner, json } => {
                let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                if json || json_output_requested() {
                    print_runner_doctor_json(&store, &installed_registry, &manifest)?;
                } else {
                    print_runner_doctor(&store, &installed_registry, &manifest);
                }
            }
            RunnerCommand::Show { runner } => {
                let manifest = package_registry.runner(&runner).map_err(cli_error)?;
                let installed = installed_registry.is_runner_installed(&manifest.id);
                let record = installed_registry.installed_runner_record(&manifest.id).ok();
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
                let removed = installed_registry.remove_runner(&runner).map_err(cli_error)?;
                print_value(&serde_json::json!({"id": runner, "removed": removed}))?;
            }
        },
        Some(Command::Adapter { command }) => match command {
            AdapterCommand::List => {
                let records = python_adapter_records(store.root()).map_err(cli_error)?;
                print_serializable(&records)?;
            }
            AdapterCommand::Install { adapter }) => {
                let _maintenance_guard = acquire_maintenance_lock(store.root()).map_err(cli_error)?;
                let adapter = normalize_adapter_id(&adapter);
                let record = install_python_adapter(store.root(), &adapter).map_err(cli_error)?;
                print_serializable(&record)?;
            }
            AdapterCommand::Doctor { adapter, json } => {
                let adapter = normalize_adapter_id(&adapter);
                let record = python_adapter_record(store.root(), &adapter).map_err(cli_error)?;
                print_adapter_doctor(&store, &record, json || json_output_requested())?;
            }
        },
        Some(Command::Test(mut args)) => {
            args.json |= json_output_requested();
            run_test_command(&store, &package_registry, &installed_registry, args).await?
        }
    }

    Ok(())
}

fn command_uses_workspace(command: &Option<Command>) -> bool {
    matches!(
        command,
        None | Some(Command::Gui)
            | Some(Command::Speak(_))
            | Some(Command::Run(_))
            | Some(Command::Transcribe { .. })
            | Some(Command::Clone(_))
            | Some(Command::Voice { command: VoiceCommand::Add { .. } })
            | Some(Command::Voice { command: VoiceCommand::Rvc { command: RvcVoiceCommand::Test { .. } } })
            | Some(Command::Convert(_))
            | Some(Command::Train(_))
    )
}

fn starts_new_session(command: &Option<Command>) -> bool {
    matches!(command, None | Some(Command::Gui))
}

fn surface_title(command: &Option<Command>) -> &'static str {
    match command {
        None => "Takokit TUI",
        Some(Command::Gui) => "Takokit GUI",
        Some(Command::Speak(_)) => "CLI speech",
        Some(Command::Transcribe { .. }) => "CLI transcription",
        Some(Command::Clone(_)) => "CLI voice cloning",
        Some(Command::Voice { command: VoiceCommand::Add { .. } }) => "CLI voice profile",
        Some(Command::Voice { command: VoiceCommand::Rvc { command: RvcVoiceCommand::Test { .. } } }) => "CLI RVC voice test",
        Some(Command::Convert(_)) => "CLI voice conversion",
        Some(Command::Train(_)) => "CLI voice training",
        _ => "Takokit CLI",
    }
}

#[cfg(test)]
mod tests;
