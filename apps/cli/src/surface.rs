use crate::args::{Command, RvcVoiceCommand, VoiceCommand};
use takokit_core::RuntimeConfig;

pub(super) fn validate_server_binding(config: &RuntimeConfig) -> anyhow::Result<()> {
    config
        .host
        .parse::<std::net::IpAddr>()
        .map_err(|_| anyhow::anyhow!("invalid --host {}; use an IP address", config.host))?;
    let loopback = config
        .host
        .parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_loopback());
    if !loopback
        && std::env::var("TAKOKIT_API_TOKEN")
            .ok()
            .is_none_or(|token| token.trim().len() < 24)
    {
        anyhow::bail!(
            "non-loopback binding requires TAKOKIT_API_TOKEN with at least 24 characters"
        );
    }
    Ok(())
}

pub(super) fn command_uses_workspace(command: &Option<Command>) -> bool {
    matches!(
        command,
        None | Some(Command::Gui)
            | Some(Command::Speak(_))
            | Some(Command::Run(_))
            | Some(Command::Transcribe { .. })
            | Some(Command::Clone(_))
            | Some(Command::Voice {
                command: VoiceCommand::Add { .. }
            })
            | Some(Command::Voice {
                command: VoiceCommand::Rvc {
                    command: RvcVoiceCommand::Test { .. }
                }
            })
            | Some(Command::Convert(_))
            | Some(Command::Train(_))
    )
}

pub(super) fn starts_new_session(command: &Option<Command>) -> bool {
    matches!(command, None | Some(Command::Gui))
}

pub(super) fn surface_title(command: &Option<Command>) -> &'static str {
    match command {
        None => "Takokit TUI",
        Some(Command::Gui) => "Takokit GUI",
        Some(Command::Speak(_)) => "CLI speech",
        Some(Command::Transcribe { .. }) => "CLI transcription",
        Some(Command::Clone(_)) => "CLI voice cloning",
        Some(Command::Voice {
            command: VoiceCommand::Add { .. },
        }) => "CLI voice profile",
        Some(Command::Voice {
            command:
                VoiceCommand::Rvc {
                    command: RvcVoiceCommand::Test { .. },
                },
        }) => "CLI RVC voice test",
        Some(Command::Convert(_)) => "CLI voice conversion",
        Some(Command::Train(_)) => "CLI voice training",
        _ => "Takokit CLI",
    }
}
