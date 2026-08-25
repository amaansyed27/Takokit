from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    if old not in text:
        raise RuntimeError(f"expected block missing in {path}")
    file.write_text(text.replace(old, new, 1), encoding="utf-8")


replace(
    "apps/cli/src/lib.rs",
    "mod output;\nmod test_commands;\n",
    "mod output;\nmod rvc_voice_command;\nmod test_commands;\nmod voice_command;\n",
)
replace(
    "apps/cli/src/lib.rs",
    """        Some(Command::Voice { command }) => match command {
            VoiceCommand::List => {
                print_serializable(&VoiceProfileStore::new(store.voices_dir()).list()?)?;
            }
            VoiceCommand::Show { model } => {
                let manifest = package_registry.model(&model).map_err(cli_error)?;
                print_serializable(&voice_contract_for_model(&manifest))?;
            }
            VoiceCommand::Add {
                sample,
                name,
                model,
                consent,
            } => run_clone(
                CloneArgs {
                    sample,
                    name,
                    model,
                    consent,
                },
                workspace.as_ref().expect("voice add workspace"),
            )?,
        },""",
    """        Some(Command::Voice { command }) => {
            voice_command::run_voice_command(
                command,
                &store,
                &package_registry,
                &installed_registry,
                workspace.as_ref(),
            )
            .await?;
        }""",
)
replace(
    "apps/cli/src/lib.rs",
    """            | Some(Command::Voice {
                command: VoiceCommand::Add { .. }
            })
            | Some(Command::Convert(_))""",
    """            | Some(Command::Voice {
                command: VoiceCommand::Add { .. }
            })
            | Some(Command::Voice {
                command: VoiceCommand::Rvc {
                    command: RvcVoiceCommand::Test { .. },
                },
            })
            | Some(Command::Convert(_))""",
)
replace(
    "apps/cli/src/lib.rs",
    """        Some(Command::Voice {
            command: VoiceCommand::Add { .. },
        }) => "CLI voice profile",
        Some(Command::Convert(_)) => "CLI voice conversion",""",
    """        Some(Command::Voice {
            command: VoiceCommand::Add { .. },
        }) => "CLI voice profile",
        Some(Command::Voice {
            command: VoiceCommand::Rvc {
                command: RvcVoiceCommand::Test { .. },
            },
        }) => "CLI RVC voice test",
        Some(Command::Convert(_)) => "CLI voice conversion",""",
)
replace(
    "apps/cli/src/daemon_commands.rs",
    """        | Command::Transcribe { .. }
        | Command::Runner { .. }""",
    """        | Command::Transcribe { .. }
        | Command::Voice {
            command: VoiceCommand::Rvc { .. },
        }
        | Command::Runner { .. }""",
)
replace(
    "apps/cli/src/daemon_commands.rs",
    """    let output = match command {
        Command::Models =>""",
    """    let output = match command {
        Command::Voice {
            command: VoiceCommand::Rvc { command },
        } => crate::rvc_voice_command::run_daemon(&client, command)?,
        Command::Models =>""",
)
replace(
    "apps/cli/src/direct_inference.rs",
    "use takokit_models::{execute_voice_conversion, execute_voice_training};",
    "use takokit_models::{execute_voice_conversion, execute_voice_training, RvcVoiceService};",
)
replace(
    "apps/cli/src/direct_inference.rs",
    """    let request = VoiceConversionRequest {
        model: args.model.clone(),
        source_path: args.source.clone(),
        target_voice: args.target_voice,""",
    """    let target_voice = if args.model == "rvc" {
        let store = LocalStore::new(LocalStore::default_root());
        RvcVoiceService::new(store.root()).resolve_conversion_target(&args.target_voice)?
    } else {
        args.target_voice.clone()
    };
    let request = VoiceConversionRequest {
        model: args.model.clone(),
        source_path: args.source.clone(),
        target_voice,""",
)
