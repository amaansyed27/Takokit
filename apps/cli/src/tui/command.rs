use clap::Parser;

use crate::args::{Cli, Command};

use super::app::TuiAction;

pub fn parse(input: &str) -> Result<TuiAction, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Type a Takokit command. Press F1 for help.".to_string());
    }
    if matches!(trimmed, "q" | "quit" | "exit") {
        return Ok(TuiAction::Quit);
    }
    if matches!(trimmed, "refresh" | "reload") {
        return Ok(TuiAction::Refresh);
    }

    let mut args = split_command(trimmed)?;
    if matches!(args.first().map(String::as_str), Some("takokit" | "tako")) {
        args.remove(0);
    }
    if args.is_empty() {
        return Err("A command is required after `takokit` or `tako`.".to_string());
    }
    validate_cli(&args)?;
    Ok(TuiAction::RunCli(args))
}

pub fn format_args(args: &[String]) -> String {
    args.iter()
        .map(|argument| {
            if argument.is_empty() || argument.chars().any(char::is_whitespace) {
                format!("\"{}\"", argument.replace('"', "\\\""))
            } else {
                argument.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn validate_cli(args: &[String]) -> Result<(), String> {
    let mut argv = vec!["takokit".to_string()];
    argv.extend(args.iter().cloned());
    let cli = Cli::try_parse_from(argv).map_err(|error| clean_clap_error(error.to_string()))?;

    match cli.command {
        None => Err("A command is required inside the TUI command bar.".to_string()),
        Some(Command::Serve {
            daemon_child: false,
            ..
        }) => {
            Err("Foreground `serve` would block the TUI. Use `daemon start` instead.".to_string())
        }
        Some(Command::Serve {
            daemon_child: true, ..
        }) => Err("Internal daemon-child flags cannot be launched from the TUI.".to_string()),
        Some(_) => Ok(()),
    }
}

fn clean_clap_error(error: String) -> String {
    error
        .lines()
        .filter(|line| !line.trim_start().starts_with("For more information"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn split_command(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        match quote {
            Some(delimiter) if character == delimiter => quote = None,
            Some('"') if character == '\\' && characters.peek() == Some(&'"') => {
                characters.next();
                current.push('"');
            }
            Some(_) => current.push(character),
            None if matches!(character, '\'' | '"') => quote = Some(character),
            None if character.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            None => current.push(character),
        }
    }

    if let Some(delimiter) = quote {
        return Err(format!("Unclosed {delimiter} quote in command."));
    }
    if !current.is_empty() {
        args.push(current);
    }
    if args.is_empty() {
        Err("A command is required inside the TUI command bar.".to_string())
    } else {
        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_quoted_text_and_windows_paths() {
        assert_eq!(
            split_command("speak \"Hello from Takokit\" --model kokoro").unwrap(),
            vec!["speak", "Hello from Takokit", "--model", "kokoro"]
        );
        assert_eq!(
            split_command("run whisper-tiny --file \"C:\\Users\\Amaan\\test.wav\"").unwrap(),
            vec![
                "run",
                "whisper-tiny",
                "--file",
                "C:\\Users\\Amaan\\test.wav"
            ]
        );
    }

    #[test]
    fn formats_editable_commands_and_accepts_binary_prefixes() {
        assert_eq!(
            format_args(&["speak".into(), "Hello world".into()]),
            "speak \"Hello world\""
        );
        assert!(parse("takokit plan whisper-tiny").is_ok());
        assert!(parse("tako doctor").is_ok());
    }

    #[test]
    fn accepts_the_complete_public_cli_surface() {
        let commands = [
            "daemon start",
            "daemon stop",
            "daemon restart",
            "daemon status",
            "daemon logs",
            "gui",
            "doctor",
            "doctor --json",
            "version",
            "status",
            "capabilities",
            "models",
            "runners",
            "library models",
            "library runners",
            "speak hello --model mock-tts",
            "pull whisper-tiny",
            "pull whisper-tiny --metadata-only",
            "show whisper-tiny",
            "plan whisper-tiny",
            "plan whisper-tiny --json",
            "rm whisper-tiny",
            "list",
            "list models",
            "list runners",
            "list voices",
            "run kokoro hello --voice default",
            "run whisper-tiny --file sample.wav",
            "ps",
            "runner pull takokit-whispercpp",
            "runner install takokit-whispercpp",
            "runner doctor takokit-whispercpp",
            "runner doctor takokit-whispercpp --json",
            "runner show takokit-whispercpp",
            "runner rm takokit-whispercpp",
            "adapter list",
            "adapter install qwen3_tts",
            "adapter doctor qwen3_tts",
            "adapter doctor qwen3_tts --json",
            "quickstart",
            "quickstart --full",
            "deps doctor",
            "deps bootstrap",
            "samples create",
            "test whisper-tiny",
            "test whisper-tiny --run --file sample.wav",
            "test --suite fast --run",
            "test --suite launch --json",
            "transcribe sample.wav --model whisper-tiny",
            "clone sample.wav --name local-voice",
            "train samples --name local-voice",
        ];

        for command in commands {
            assert!(parse(command).is_ok(), "TUI rejected `{command}`");
        }
    }

    #[test]
    fn handles_tui_control_commands_and_rejects_foreground_server() {
        assert_eq!(parse("quit"), Ok(TuiAction::Quit));
        assert_eq!(parse("refresh"), Ok(TuiAction::Refresh));
        assert!(parse("serve").is_err());
        assert!(parse("unknown").is_err());
        assert!(parse("speak \"unfinished").is_err());
    }
}
