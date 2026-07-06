use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use takokit_core::{RuntimeConfig, SpeechRequest, TakokitError};
use takokit_models::{MockTextToSpeechEngine, ModelRegistry, TextToSpeechEngine};
use takokit_server::{run_server, AppState};
use takokit_store::LocalStore;

#[derive(Debug, Parser)]
#[command(name = "takokit", version, about = "Local voice AI runtime")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve,
    Status,
    Speak(SpeakArgs),
    Pull {
        model: String,
    },
    List {
        #[command(subcommand)]
        target: ListTarget,
    },
    Transcribe {
        audio: PathBuf,
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
    Voices,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let store = LocalStore::new(LocalStore::default_root());
    store.ensure_layout()?;

    match cli.command {
        Command::Serve => {
            let config = RuntimeConfig::local(store.root().to_path_buf());
            run_server(AppState::new(config, store)).await?;
        }
        Command::Status => {
            let config = RuntimeConfig::local(store.root().to_path_buf());
            let state = AppState::new(config, store);
            println!("{}", serde_json::to_string_pretty(&state.status())?);
        }
        Command::Speak(args) => {
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
        Command::Pull { model: _ } => {
            return not_implemented(
                "model pull",
                "model download plumbing is planned but not implemented",
            )
        }
        Command::List { target } => {
            let registry = ModelRegistry::default();
            match target {
                ListTarget::Models => {
                    println!("{}", serde_json::to_string_pretty(registry.models())?)
                }
                ListTarget::Voices => {
                    println!("{}", serde_json::to_string_pretty(registry.voices())?)
                }
            }
        }
        Command::Transcribe { audio: _ } => {
            return not_implemented("speech transcription", "Whisper adapters are not wired yet")
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
    }

    Ok(())
}

fn not_implemented(feature: &'static str, reason: &'static str) -> anyhow::Result<()> {
    Err(TakokitError::NotImplemented { feature, reason }.into())
}
