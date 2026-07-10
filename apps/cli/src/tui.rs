use std::io::{self, Write};

use takokit_core::{RuntimeConfig, SpeechRequest};
use takokit_models::{MockTextToSpeechEngine, TextToSpeechEngine};
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

use crate::{doctor, gui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherAction {
    ChatRunVoiceModel,
    GenerateMockSpeech,
    TranscribeAudio,
    PullModel,
    PullRunner,
    OpenGui,
    StartServer,
    ShowInstalledModels,
    ShowRunners,
    Doctor,
    Quit,
}

impl LauncherAction {
    pub fn label(self) -> &'static str {
        match self {
            LauncherAction::ChatRunVoiceModel => "Chat / run a voice model",
            LauncherAction::GenerateMockSpeech => "Generate speech with mock-tts",
            LauncherAction::TranscribeAudio => "Transcribe audio",
            LauncherAction::PullModel => "Pull model metadata",
            LauncherAction::PullRunner => "Pull runner contract",
            LauncherAction::OpenGui => "Open local web GUI",
            LauncherAction::StartServer => "Start server",
            LauncherAction::ShowInstalledModels => "Show installed models",
            LauncherAction::ShowRunners => "Show runners",
            LauncherAction::Doctor => "Doctor",
            LauncherAction::Quit => "Quit",
        }
    }
}

const ACTIONS: [LauncherAction; 11] = [
    LauncherAction::ChatRunVoiceModel,
    LauncherAction::GenerateMockSpeech,
    LauncherAction::TranscribeAudio,
    LauncherAction::PullModel,
    LauncherAction::PullRunner,
    LauncherAction::OpenGui,
    LauncherAction::StartServer,
    LauncherAction::ShowInstalledModels,
    LauncherAction::ShowRunners,
    LauncherAction::Doctor,
    LauncherAction::Quit,
];

pub fn launcher_actions() -> &'static [LauncherAction] {
    &ACTIONS
}

pub async fn run_launcher(
    config: &RuntimeConfig,
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    println!("Takokit");
    println!("Local voice AI runtime. Real model inference is not implemented yet.");

    loop {
        println!();
        for (index, action) in launcher_actions().iter().enumerate() {
            println!("  {}. {}", index + 1, action.label());
        }

        let Some(action) = prompt_action()? else {
            println!("Choose a number from the menu.");
            continue;
        };

        match action {
            LauncherAction::ChatRunVoiceModel => {
                println!("Use `takokit run <model> <text>` for TTS or `takokit run <model> --file <audio>` for STT. The daemon reports model readiness blockers.");
            }
            LauncherAction::GenerateMockSpeech => {
                generate_mock_speech(store).await?;
            }
            LauncherAction::TranscribeAudio => {
                println!("Use `takokit run whisper-tiny --file <audio>` or `takokit transcribe <audio> --model whisper-tiny`.");
            }
            LauncherAction::PullModel => pull_model_metadata(package_registry, installed_registry)?,
            LauncherAction::PullRunner => {
                pull_runner_contract(package_registry, installed_registry)?
            }
            LauncherAction::OpenGui => gui::open_gui(store, config).await?,
            LauncherAction::StartServer => {
                gui::ensure_server(store, config).await?;
                println!(
                    "Takokit managed daemon available at {}",
                    config.local_base_url()
                );
            }
            LauncherAction::ShowInstalledModels => {
                for model in package_registry.models()? {
                    let status = if installed_registry.is_model_installed(&model.id) {
                        "installed"
                    } else {
                        "available"
                    };
                    println!("{} ({}) - {}", model.name, model.id, status);
                }
            }
            LauncherAction::ShowRunners => {
                for runner in package_registry.runners()? {
                    let status = if installed_registry.is_runner_installed(&runner.id) {
                        "installed"
                    } else {
                        "available"
                    };
                    println!("{} ({}) - {}", runner.name, runner.id, status);
                }
            }
            LauncherAction::Doctor => {
                let report =
                    doctor::run_doctor(config, store, package_registry, installed_registry);
                doctor::print_report(&report);
            }
            LauncherAction::Quit => break,
        }
    }

    Ok(())
}

fn prompt_action() -> anyhow::Result<Option<LauncherAction>> {
    let input = prompt("Select an option: ")?;
    let Ok(index) = input.trim().parse::<usize>() else {
        return Ok(None);
    };

    Ok(launcher_actions().get(index.saturating_sub(1)).copied())
}

async fn generate_mock_speech(store: &LocalStore) -> anyhow::Result<()> {
    let text = prompt("Text: ")?;
    if text.trim().is_empty() {
        println!("No text provided.");
        return Ok(());
    }

    let voice = prompt("Voice [default]: ")?;
    let voice = if voice.trim().is_empty() {
        "default".to_string()
    } else {
        voice.trim().to_string()
    };

    let response = MockTextToSpeechEngine
        .synthesize(
            SpeechRequest {
                model: "mock-tts".to_string(),
                input: text,
                voice: Some(voice),
                response_format: Some("wav".to_string()),
            },
            &store.outputs_dir(),
        )
        .await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn pull_model_metadata(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let models = package_registry.models()?;
    for (index, model) in models.iter().enumerate() {
        println!("  {}. {} ({})", index + 1, model.name, model.id);
    }

    let input = prompt("Model number: ")?;
    let Ok(index) = input.trim().parse::<usize>() else {
        println!("No model selected.");
        return Ok(());
    };
    let Some(model) = models.get(index.saturating_sub(1)) else {
        println!("No model selected.");
        return Ok(());
    };

    let report = installed_registry.install_model(model)?;
    println!("Installed {} metadata.", report.id);
    println!("{}", report.note);
    Ok(())
}

fn pull_runner_contract(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let runners = package_registry.runners()?;
    for (index, runner) in runners.iter().enumerate() {
        println!("  {}. {} ({})", index + 1, runner.name, runner.id);
    }

    let input = prompt("Runner number: ")?;
    let Ok(index) = input.trim().parse::<usize>() else {
        println!("No runner selected.");
        return Ok(());
    };
    let Some(runner) = runners.get(index.saturating_sub(1)) else {
        println!("No runner selected.");
        return Ok(());
    };

    let report = installed_registry.install_runner(runner)?;
    println!("Installed {} runner contract.", report.id);
    println!("{}", report.note);
    Ok(())
}

fn prompt(label: &str) -> anyhow::Result<String> {
    print!("{label}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim_end_matches(['\r', '\n']).to_string())
}
