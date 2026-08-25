use crate::{
    args::{rvc::RvcTrainingPresetArg, RvcSampleCommand, RvcVoiceCommand},
    Cli, Command, VoiceCommand,
};
use clap::Parser;
use std::path::PathBuf;

fn rvc(command: Command) -> RvcVoiceCommand {
    match command {
        Command::Voice {
            command: VoiceCommand::Rvc { command },
        } => command,
        other => panic!("expected voice rvc command, got {other:?}"),
    }
}

#[test]
fn parses_create_and_unicode_name() {
    let cli = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "create",
        "--name",
        "Studio Voice ü",
        "--consent",
    ])
    .unwrap();
    assert!(matches!(
        rvc(cli.command.unwrap()),
        RvcVoiceCommand::Create { name, consent: true, .. } if name == "Studio Voice ü"
    ));
}

#[test]
fn parses_multi_sample_paths_with_spaces() {
    let cli = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "samples",
        "voice-id",
        "add",
        r"C:\Voice Project ü\one.wav",
        r"C:\Voice Project ü\two.wav",
    ])
    .unwrap();
    assert!(matches!(
        rvc(cli.command.unwrap()),
        RvcVoiceCommand::Samples {
            voice,
            command: RvcSampleCommand::Add { paths }
        } if voice == "voice-id"
            && paths == vec![PathBuf::from(r"C:\Voice Project ü\one.wav"), PathBuf::from(r"C:\Voice Project ü\two.wav")]
    ));
}

#[test]
fn parses_import_checkpoint_and_index() {
    let cli = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "import",
        r"C:\models\voice ü.pth",
        "--index",
        r"C:\models\voice ü.index",
        "--name",
        "Imported ü",
        "--consent",
    ])
    .unwrap();
    assert!(matches!(
        rvc(cli.command.unwrap()),
        RvcVoiceCommand::Import { checkpoint, index: Some(index), name, consent: true, .. }
            if checkpoint == PathBuf::from(r"C:\models\voice ü.pth")
                && index == PathBuf::from(r"C:\models\voice ü.index")
                && name == "Imported ü"
    ));
}

#[test]
fn parses_custom_training_and_test_voice() {
    let train = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "train",
        "voice-id",
        "--preset",
        "custom",
        "--epochs",
        "24",
        "--batch-size",
        "2",
        "--save-every-epochs",
        "6",
        "--device",
        "cpu",
        "--precision",
        "fp32",
    ])
    .unwrap();
    assert!(matches!(
        rvc(train.command.unwrap()),
        RvcVoiceCommand::Train { voice, training }
            if voice == "voice-id" && training.preset == RvcTrainingPresetArg::Custom
    ));

    let test = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "test",
        "voice-id",
        r"C:\Voice Project ü\source.wav",
    ])
    .unwrap();
    assert!(matches!(
        rvc(test.command.unwrap()),
        RvcVoiceCommand::Test { voice, input }
            if voice == "voice-id" && input == PathBuf::from(r"C:\Voice Project ü\source.wav")
    ));
}

#[test]
fn parses_package_export_verify_import_and_dry_run_remove() {
    let export = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "export",
        "voice-id",
        r"C:\exports\voice ü.takovoice",
        "--sign",
    ])
    .unwrap();
    assert!(matches!(
        rvc(export.command.unwrap()),
        RvcVoiceCommand::Export { voice, output, sign: true, .. }
            if voice == "voice-id" && output == PathBuf::from(r"C:\exports\voice ü.takovoice")
    ));

    let verify = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "verify",
        r"C:\exports\voice ü.takovoice",
    ])
    .unwrap();
    assert!(matches!(
        rvc(verify.command.unwrap()),
        RvcVoiceCommand::Verify { package }
            if package == PathBuf::from(r"C:\exports\voice ü.takovoice")
    ));

    let import = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "import-package",
        r"C:\exports\voice ü.takovoice",
        "--consent",
    ])
    .unwrap();
    assert!(matches!(
        rvc(import.command.unwrap()),
        RvcVoiceCommand::ImportPackage { consent: true, .. }
    ));

    let remove = Cli::try_parse_from([
        "tako",
        "voice",
        "rvc",
        "remove",
        "voice-id",
        "--dry-run",
    ])
    .unwrap();
    assert!(matches!(
        rvc(remove.command.unwrap()),
        RvcVoiceCommand::Remove { voice, dry_run: true } if voice == "voice-id"
    ));
}
