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
