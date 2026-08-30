//! CLI routing for managed daemon lifecycle commands.

use super::*;

#[path = "progress.rs"]
mod progress;
use progress::Activity;

pub(crate) async fn run_foreground_server(
    store: LocalStore,
    config: RuntimeConfig,
) -> anyhow::Result<()> {
    surface::validate_server_binding(&config)?;
    match daemon::inspect_server(&store, &config)? {
        daemon::ServerInspection::Verified(identity) => {
            println!(
                "Takokit server is already running at {}",
                config.local_base_url()
            );
            println!("Process: {} ({:?})", identity.pid, identity.mode);
            println!("Stop it with: tako stop");
            println!("Inspect it with: tako server status");
            return Ok(());
        }
        daemon::ServerInspection::ForeignPort => anyhow::bail!(
            "port {} is already in use by another application; Takokit did not stop or replace it",
            config.port
        ),
        daemon::ServerInspection::Stopped => {}
    }
    println!("Takokit {}", env!("CARGO_PKG_VERSION"));
    println!("Server listening on {}", config.local_base_url());
    println!("OpenAI API: {}/v1", config.local_base_url());
    println!("Takokit API: {}/api/v1", config.local_base_url());
    println!("GUI: {}/gui", config.local_base_url());
    println!("\nCtrl+C to stop");
    daemon::run_foreground(store, config).await
}

pub(crate) fn start_server(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    let _ = daemon::ensure_running(store, config)?;
    println!("Takokit server started at {}", config.local_base_url());
    Ok(())
}

pub(crate) fn stop_server(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    if daemon::stop_verified_server(store, config)? {
        println!("Takokit server stopped.");
    } else {
        println!("Takokit server is not running.");
    }
    Ok(())
}

pub(crate) fn normalize_adapter_id(adapter: &str) -> String {
    adapter.trim().replace('-', "_")
}

pub(crate) async fn route_daemon_command(
    command: &Option<Command>,
    store: &LocalStore,
    config: &RuntimeConfig,
) -> anyhow::Result<bool> {
    let Some(command) = command else {
        return Ok(false);
    };
    let client = match command {
        Command::Models
        | Command::Runners
        | Command::Status
        | Command::Doctor(_)
        | Command::Capabilities
        | Command::Speak(_)
        | Command::Pull(_)
        | Command::Show { .. }
        | Command::Plan(_)
        | Command::Rm(_)
        | Command::List { .. }
        | Command::Run(_)
        | Command::Ps
        | Command::Transcribe { .. }
        | Command::Voice {
            command: VoiceCommand::Rvc { .. },
        }
        | Command::Runner { .. }
        | Command::Adapter { .. } => daemon_client::Client::ensure(store, config)?,
        Command::Server { .. } | Command::Daemon { .. } => return Ok(false),
        _ => return Ok(false),
    };
    let output = match command {
        Command::Voice {
            command: VoiceCommand::Rvc { command },
        } => crate::rvc_voice_command::run_daemon(&client, command)?,
        Command::Models => client.get::<serde_json::Value>("/api/v1/models")?,
        Command::Runners => client.get("/api/v1/runners")?,
        Command::Status => client.get("/api/v1/status")?,
        Command::Doctor(_) => client.get("/api/v1/doctor")?,
        Command::Capabilities => client.get("/api/v1/capabilities")?,
        Command::Pull(args) => post_model_pull_with_activity(
            &client,
            format!("Pulling {}", args.model),
            &args.model,
            "/api/v1/models/pull",
            &serde_json::json!({
                "model": args.model,
                "metadata_only": args.metadata_only,
                "accepted_license": args.accept_license,
            }),
        )?,
        Command::Show { model } => client.get(&format!("/api/v1/models/{model}"))?,
        Command::Plan(args) => client.get(&format!("/api/v1/models/{}/plan", args.model))?,
        Command::Rm(args) => client.delete_json(&format!(
            "/api/v1/models/{}?dry_run={}",
            args.model, args.dry_run
        ))?,
        Command::List { target } => match target {
            None | Some(ListTarget::Models) => client.get("/api/v1/models/installed")?,
            Some(ListTarget::Runners) => client.get("/api/v1/runners")?,
            Some(ListTarget::Voices) => client.get("/api/v1/voices")?,
        },
        Command::Speak(args) => post_with_activity(
            &client,
            format!("Generating speech with {}", args.model),
            "/api/v1/audio/speech",
            &SpeechRequest {
                model: args.model.clone(),
                input: args.text.clone(),
                voice: Some(args.voice.clone()),
                response_format: Some("wav".to_string()),
                language: args.language.clone(),
                instruction: args.instruction.clone(),
                reference_text: args.reference_text.clone(),
            },
        )?,
        Command::Transcribe { audio, model } => post_with_activity(
            &client,
            format!("Transcribing with {model}"),
            "/api/v1/audio/transcriptions",
            &takokit_core::TranscriptionRequest {
                file_path: audio.clone(),
                model: Some(model.clone()),
            },
        )?,
        Command::Run(args) => {
            validate_run_args(args)?;
            let model: serde_json::Value = client.get(&format!("/api/v1/models/{}", args.model))?;
            let capabilities = model["data"]["capabilities"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let supports = |capability: &str| {
                capabilities
                    .iter()
                    .any(|item| item.as_str() == Some(capability))
            };
            match (&args.text, &args.file) {
                (Some(text), None) if supports("text_to_speech") => post_with_activity(
                    &client,
                    format!("Generating speech with {}", args.model),
                    "/api/v1/audio/speech",
                    &SpeechRequest {
                        model: args.model.clone(),
                        input: text.clone(),
                        voice: args.voice.clone(),
                        response_format: Some("wav".to_string()),
                        language: args.language.clone(),
                        instruction: args.instruction.clone(),
                        reference_text: args.reference_text.clone(),
                    },
                )?,
                (None, Some(file)) if supports("speech_to_text") => post_with_activity(
                    &client,
                    format!("Transcribing with {}", args.model),
                    "/api/v1/audio/transcriptions",
                    &takokit_core::TranscriptionRequest {
                        file_path: file.clone(),
                        model: Some(args.model.clone()),
                    },
                )?,
                (Some(_), None) => {
                    return Err(anyhow::anyhow!(
                        "model {} does not support text to speech",
                        args.model
                    ))
                }
                (None, Some(_)) => {
                    return Err(anyhow::anyhow!(
                        "model {} does not support speech to text",
                        args.model
                    ))
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "provide text for TTS or --file for STT, but not both"
                    ))
                }
            }
        }
        Command::Ps => client.get("/api/v1/ps")?,
        Command::Runner { command } => match command {
            RunnerCommand::Pull { runner } => post_retryable_with_activity(
                &client,
                format!("Pulling runner {runner}"),
                "/api/v1/runners/pull",
                &serde_json::json!({"runner":runner}),
            )?,
            RunnerCommand::Install { runner } => post_retryable_with_activity(
                &client,
                format!("Installing runner {runner}"),
                "/api/v1/runners/install",
                &serde_json::json!({"runner":runner}),
            )?,
            RunnerCommand::Doctor { runner, .. } => {
                client.get(&format!("/api/v1/runners/{runner}/doctor"))?
            }
            RunnerCommand::Show { runner } => client.get(&format!("/api/v1/runners/{runner}"))?,
            RunnerCommand::Rm { runner } => {
                client.delete(&format!("/api/v1/runners/{runner}"))?;
                serde_json::json!({"id":runner,"removed":true})
            }
        },
        Command::Adapter { command } => match command {
            AdapterCommand::List => client.get("/api/v1/adapters")?,
            AdapterCommand::Install { adapter } => post_retryable_with_activity(
                &client,
                format!("Installing adapter {adapter}"),
                "/api/v1/adapters/install",
                &serde_json::json!({"adapter":adapter}),
            )?,
            AdapterCommand::Doctor { adapter, .. } => client.get(&format!(
                "/api/v1/adapters/{}/doctor",
                normalize_adapter_id(adapter)
            ))?,
        },
        _ => return Ok(false),
    };
    if command_requests_json(command) {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_value(&output)?;
    }
    Ok(true)
}

fn post_with_activity<B: serde::Serialize>(
    client: &daemon_client::Client,
    label: String,
    path: &str,
    body: &B,
) -> anyhow::Result<serde_json::Value> {
    let activity = Activity::start(label);
    let result = client.post(path, body);
    drop(activity);
    result
}

fn post_retryable_with_activity<B: serde::Serialize>(
    client: &daemon_client::Client,
    label: String,
    path: &str,
    body: &B,
) -> anyhow::Result<serde_json::Value> {
    let activity = Activity::start(label);
    let result = client.post_retryable(path, body);
    drop(activity);
    result
}

fn post_model_pull_with_activity<B: serde::Serialize>(
    client: &daemon_client::Client,
    label: String,
    model: &str,
    path: &str,
    body: &B,
) -> anyhow::Result<serde_json::Value> {
    let activity = Activity::start_model_pull(label, client.clone(), model.to_string());
    let result = client.post_retryable(path, body);
    drop(activity);
    result
}

fn command_requests_json(command: &Command) -> bool {
    matches!(
        command,
        Command::Doctor(DoctorArgs { json: true })
            | Command::Plan(PlanArgs { json: true, .. })
            | Command::Runner {
                command: RunnerCommand::Doctor { json: true, .. }
            }
            | Command::Adapter {
                command: AdapterCommand::Doctor { json: true, .. }
            }
    )
}
