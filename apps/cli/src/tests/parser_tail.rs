use super::*;

#[test]
fn cli_parses_model_removal_dry_run() {
    let cli = Cli::try_parse_from(["takokit", "rm", "whisper:tiny", "--dry-run"])
        .expect("model removal dry-run");
    assert!(matches!(
        cli.command,
        Some(Command::Rm(RmArgs { model, dry_run: true })) if model == "whisper:tiny"
    ));
}
#[test]
fn cli_parses_top_level_start_and_stop() {
    let start = Cli::try_parse_from(["tako", "start"]).expect("start alias");
    let stop = Cli::try_parse_from(["tako", "stop"]).expect("stop alias");

    assert!(matches!(start.command, Some(Command::Start)));
    assert!(matches!(stop.command, Some(Command::Stop)));
}
