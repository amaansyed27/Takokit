use super::app::TuiAction;

pub fn parse(input: &str) -> Result<TuiAction, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Type a command after `/`. Press `?` for help.".to_string());
    }

    let mut parts = trimmed.split_whitespace();
    let command = parts.next().unwrap_or_default();
    let remaining = parts.collect::<Vec<_>>();

    match command {
        "q" | "quit" | "exit" => expect_no_args(command, &remaining).map(|_| TuiAction::Quit),
        "refresh" | "reload" => {
            expect_no_args(command, &remaining).map(|_| TuiAction::Refresh)
        }
        "doctor" => expect_no_args(command, &remaining).map(|_| TuiAction::Doctor),
        "gui" => expect_no_args(command, &remaining).map(|_| TuiAction::OpenGui),
        "server" | "serve" => {
            expect_no_args(command, &remaining).map(|_| TuiAction::StartServer)
        }
        "pull" => one_arg(command, &remaining).map(TuiAction::PullModel),
        "plan" => one_arg(command, &remaining).map(TuiAction::PlanModel),
        "runner" => parse_runner(&remaining),
        _ => Err(format!(
            "Unknown TUI command `{command}`. Try pull, plan, runner install, doctor, gui, server, refresh, or quit."
        )),
    }
}

fn parse_runner(parts: &[&str]) -> Result<TuiAction, String> {
    match parts {
        ["install", runner] => Ok(TuiAction::InstallRunner((*runner).to_string())),
        ["show", runner] => Ok(TuiAction::ShowRunner((*runner).to_string())),
        _ => Err("Usage: runner install <id> or runner show <id>".to_string()),
    }
}

fn one_arg(command: &str, parts: &[&str]) -> Result<String, String> {
    match parts {
        [value] => Ok((*value).to_string()),
        _ => Err(format!("Usage: {command} <id>")),
    }
}

fn expect_no_args(command: &str, parts: &[&str]) -> Result<(), String> {
    if parts.is_empty() {
        Ok(())
    } else {
        Err(format!("`{command}` does not accept arguments"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_model_and_runner_commands() {
        assert_eq!(
            parse("pull whisper-tiny"),
            Ok(TuiAction::PullModel("whisper-tiny".to_string()))
        );
        assert_eq!(
            parse("runner install takokit-whispercpp"),
            Ok(TuiAction::InstallRunner("takokit-whispercpp".to_string()))
        );
    }

    #[test]
    fn rejects_unknown_or_incomplete_commands() {
        assert!(parse("pull").is_err());
        assert!(parse("unknown").is_err());
    }
}
