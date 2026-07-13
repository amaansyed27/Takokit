use super::*;
use crate::workspace::CliWorkspace;
use takokit_core::{SessionTask, SpeechRequest, TranscriptionRequest};

pub(crate) async fn run_speak(
    args: SpeakArgs,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    workspace: &CliWorkspace,
) -> anyhow::Result<()> {
    let request = SpeechRequest {
        model: args.model.clone(),
        input: args.text,
        voice: Some(args.voice),
        response_format: Some("wav".to_string()),
    };
    let result = if request.model != "mock-tts" {
        let plan = resolve_execution_plan(
            package_registry,
            installed_registry,
            &request.model,
            CapabilityKind::TextToSpeech,
        )
        .map_err(cli_error)?;
        execute_speech(&plan, request.clone(), &workspace.outputs_dir())
            .await
            .map_err(runtime_error)
    } else {
        MockTextToSpeechEngine
            .synthesize(request.clone(), &workspace.outputs_dir())
            .await
            .map_err(anyhow::Error::from)
    };
    match result {
        Ok(response) => {
            workspace.record_speech(&request, &response)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Err(error) => {
            workspace.record_failure(
                SessionTask::TextToSpeech,
                Some(request.model),
                None,
                Some(request.input),
                &error,
            );
            Err(error)
        }
    }
}

pub(crate) async fn run_model(
    args: RunArgs,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    workspace: &CliWorkspace,
) -> anyhow::Result<()> {
    validate_run_args(&args)?;
    let manifest = package_registry.model(&args.model).map_err(cli_error)?;
    if let Some(text) = args.text {
        if !manifest.capabilities.tts {
            return Err(anyhow::anyhow!(
                "model {} does not support text to speech",
                args.model
            ));
        }
        return run_speak(
            SpeakArgs {
                text,
                model: args.model,
                voice: args.voice.unwrap_or_else(|| "default".to_string()),
            },
            package_registry,
            installed_registry,
            workspace,
        )
        .await;
    }
    if !manifest.capabilities.stt {
        return Err(anyhow::anyhow!(
            "model {} does not support speech to text",
            args.model
        ));
    }
    run_transcription(
        args.file.expect("validated file input"),
        args.model,
        package_registry,
        installed_registry,
        workspace,
    )
    .await
}

pub(crate) async fn run_transcription(
    audio: PathBuf,
    model: String,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    workspace: &CliWorkspace,
) -> anyhow::Result<()> {
    let request = TranscriptionRequest {
        file_path: audio,
        model: Some(model.clone()),
    };
    let plan = resolve_execution_plan(
        package_registry,
        installed_registry,
        &model,
        CapabilityKind::SpeechToText,
    )
    .map_err(cli_error)?;
    match execute_transcription(&plan, request.clone())
        .await
        .map_err(runtime_error)
    {
        Ok(response) => {
            let output = workspace.record_transcription(&request, &response)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            eprintln!("Transcript saved: {}", output.display());
            Ok(())
        }
        Err(error) => {
            workspace.record_failure(
                SessionTask::SpeechToText,
                Some(model),
                Some(request.file_path),
                None,
                &error,
            );
            Err(error)
        }
    }
}

pub(crate) fn run_clone(args: CloneArgs, workspace: &CliWorkspace) -> anyhow::Result<()> {
    let error = anyhow::anyhow!(
        "voice cloning model {} is not executable until its managed adapter is installed",
        args.model
    );
    workspace.record_failure(
        SessionTask::VoiceCloning,
        Some(args.model),
        Some(args.sample),
        Some(args.name),
        &error,
    );
    Err(error)
}

pub(crate) fn run_train(args: TrainArgs, workspace: &CliWorkspace) -> anyhow::Result<()> {
    let error = anyhow::anyhow!(
        "voice training model {} is not executable until its training adapter is installed",
        args.model
    );
    workspace.record_failure(
        SessionTask::VoiceTraining,
        Some(args.model),
        Some(args.samples),
        Some(args.name),
        &error,
    );
    Err(error)
}
