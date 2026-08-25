use crate::{
    args::{CloneArgs, VoiceCommand},
    direct_inference::run_clone,
    output::print_serializable,
    rvc_voice_command,
    workspace::CliWorkspace,
};
use takokit_package::{voice_contract_for_model, InstalledRegistry, PackageRegistry};
use takokit_store::{LocalStore, VoiceProfileStore};

pub(crate) async fn run_voice_command(
    command: VoiceCommand,
    store: &LocalStore,
    packages: &PackageRegistry,
    installed: &InstalledRegistry,
    workspace: Option<&CliWorkspace>,
) -> anyhow::Result<()> {
    match command {
        VoiceCommand::List => {
            let mut value =
                serde_json::to_value(VoiceProfileStore::new(store.voices_dir()).list()?)?;
            let mut managed =
                serde_json::to_value(takokit_models::RvcVoiceService::new(store.root()).list()?)?;
            if let (Some(items), Some(extra)) = (value.as_array_mut(), managed.as_array_mut()) {
                items.append(extra);
            }
            print_serializable(&value)
        }
        VoiceCommand::Show { model } => {
            let manifest = packages.model(&model).map_err(crate::output::cli_error)?;
            print_serializable(&voice_contract_for_model(&manifest))
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
            workspace.ok_or_else(|| anyhow::anyhow!("voice add requires a workspace"))?,
        ),
        VoiceCommand::Rvc { command } => {
            rvc_voice_command::run_direct(command, store, packages, installed, workspace).await
        }
    }
}
