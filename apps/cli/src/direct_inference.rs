use super::*;
use crate::workspace::CliWorkspace;
use takokit_core::{
    NewSessionEvent, SessionEventState, SessionTask, SpeechRequest, TrainVoiceRequest,
    TranscriptionRequest, VoiceConversionRequest,
};
use takokit_models::{execute_voice_conversion, execute_voice_training};
use takokit_store::VoiceProfileStore;

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
        language: args.language,
        instruction: args.instruction,
        reference_text: args.reference_text,
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
            print_serializable(&response)?;
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
                language: args.language,
                instruction: args.instruction,
                reference_text: args.reference_text,
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
            print_serializable(&response)?;
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
    if !args.consent {
        let error = anyhow::anyhow!(
            "voice cloning requires --consent to confirm ownership or explicit permission"
        );
        workspace.record_failure(
            SessionTask::VoiceCloning,
            Some(args.model),
            Some(args.sample),
            Some(args.name),
            &error,
        );
        return Err(error);
    }

    let store = LocalStore::new(LocalStore::default_root());
    store.ensure_layout()?;
    let package_registry = PackageRegistry::bundled();
    let installed_registry = InstalledRegistry::new(store.manifests_dir());
    let manifest = package_registry.model(&args.model).map_err(cli_error)?;
    if !manifest.capabilities.voice_cloning {
        return Err(anyhow::anyhow!(
            "model {} does not support voice cloning",
            args.model
        ));
    }
    let plan =
        plan_model(&package_registry, &installed_registry, &args.model).map_err(cli_error)?;
    if !plan.executable {
        let error = anyhow::anyhow!(
            "model {} is not ready: {}. Next: {}",
            args.model,
            if plan.missing.is_empty() {
                "runtime setup is incomplete".to_string()
            } else {
                plan.missing.join("; ")
            },
            plan.next_command
        );
        workspace.record_failure(
            SessionTask::VoiceCloning,
            Some(args.model),
            Some(args.sample),
            Some(args.name),
            &error,
        );
        return Err(error);
    }

    let profile = VoiceProfileStore::new(store.voices_dir()).create(
        &args.name,
        &args.model,
        &args.sample,
        true,
        Some("Consent affirmed through Takokit CLI.".to_string()),
    )?;
    workspace.store.append_event(
        workspace.session_id(),
        NewSessionEvent {
            task: SessionTask::VoiceCloning,
            state: SessionEventState::Completed,
            model: Some(profile.model_id.clone()),
            input: Some(profile.name.clone()),
            source_path: Some(args.sample),
            output_path: Some(profile.sample_path.clone()),
            text: None,
            message: Some(format!(
                "Created reusable voice profile {}. Use --voice {} for compatible models.",
                profile.name, profile.id
            )),
        },
    )?;
    print_serializable(&profile)?;
    Ok(())
}

pub(crate) async fn run_convert(
    args: ConvertArgs,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    workspace: &CliWorkspace,
) -> anyhow::Result<()> {
    let request = VoiceConversionRequest {
        model: args.model.clone(),
        source_path: args.source.clone(),
        target_voice: args.target_voice,
        pitch_shift: args.pitch_shift,
        consent_affirmed: args.consent,
    };
    let plan = resolve_execution_plan(
        package_registry,
        installed_registry,
        &args.model,
        CapabilityKind::VoiceConversion,
    )
    .map_err(cli_error)?;
    match execute_voice_conversion(&plan, request.clone(), &workspace.outputs_dir())
        .await
        .map_err(runtime_error)
    {
        Ok(response) => {
            workspace.store.append_event(
                workspace.session_id(),
                NewSessionEvent {
                    task: SessionTask::VoiceConversion,
                    state: SessionEventState::Completed,
                    model: Some(args.model),
                    input: Some(request.target_voice.clone()),
                    source_path: Some(request.source_path),
                    output_path: Some(response.output_path.clone()),
                    text: None,
                    message: Some(format!("Converted {} bytes", response.bytes)),
                },
            )?;
            print_serializable(&response)?;
            Ok(())
        }
        Err(error) => {
            workspace.record_failure(
                SessionTask::VoiceConversion,
                Some(args.model),
                Some(args.source),
                Some(request.target_voice),
                &error,
            );
            Err(error)
        }
    }
}

pub(crate) async fn run_train(
    args: TrainArgs,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    workspace: &CliWorkspace,
) -> anyhow::Result<()> {
    let request = TrainVoiceRequest {
        samples_path: args.samples.clone(),
        name: args.name.clone(),
        model: args.model.clone(),
        consent_affirmed: args.consent,
        epochs: args.epochs,
    };
    let plan = resolve_execution_plan(
        package_registry,
        installed_registry,
        &args.model,
        CapabilityKind::VoiceTraining,
    )
    .map_err(cli_error)?;
    match execute_voice_training(&plan, request.clone())
        .await
        .map_err(runtime_error)
    {
        Ok(response) => {
            workspace.store.append_event(
                workspace.session_id(),
                NewSessionEvent {
                    task: SessionTask::VoiceTraining,
                    state: SessionEventState::Completed,
                    model: Some(args.model),
                    input: Some(args.name),
                    source_path: Some(args.samples),
                    output_path: Some(response.output_path.clone()),
                    text: None,
                    message: Some(format!("Training status: {}", response.status)),
                },
            )?;
            print_serializable(&response)?;
            Ok(())
        }
        Err(error) => {
            workspace.record_failure(
                SessionTask::VoiceTraining,
                Some(args.model),
                Some(args.samples),
                Some(args.name),
                &error,
            );
            Err(error)
        }
    }
}
