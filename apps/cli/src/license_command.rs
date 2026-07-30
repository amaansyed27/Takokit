use crate::{args::{LicenseCommand, PullArgs}, cli_error, print_serializable};
use std::io::{self, IsTerminal, Write};
use takokit_package::{
    ensure_model_license_accepted, list_license_receipts, model_license_info,
    revoke_license_receipt, valid_license_receipt, PackageRegistry,
};

pub(crate) fn prepare_pull_license(
    takokit_root: &std::path::Path,
    registry: &PackageRegistry,
    args: &PullArgs,
    json_output: bool,
) -> anyhow::Result<()> {
    let model = registry.model_for_pull(&args.model).map_err(cli_error)?;
    let Some(license) = model_license_info(&model) else {
        return Ok(());
    };
    if valid_license_receipt(takokit_root, &model).map_err(cli_error)?.is_some() {
        return Ok(());
    }
    if let Some(accepted) = args.accept_license.as_deref() {
        ensure_model_license_accepted(takokit_root, &model, Some(accepted)).map_err(cli_error)?;
        return Ok(());
    }
    if json_output || !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return ensure_model_license_accepted(takokit_root, &model, None)
            .map(|_| ())
            .map_err(cli_error);
    }

    println!("Model: {} ({})", model.name, model.id);
    println!("License: {} {}", license.name, license.version);
    println!("Warning: {}", license.notice);
    println!("License URL: {}", license.url);
    print!("Do you accept {} for this model? [y/N] ", license.id);
    io::stdout().flush()?;
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    if !matches!(response.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        anyhow::bail!("license acceptance declined; no model data was downloaded");
    }
    ensure_model_license_accepted(takokit_root, &model, Some(&license.id)).map_err(cli_error)?;
    Ok(())
}

pub(crate) fn run_license_command(
    takokit_root: &std::path::Path,
    command: LicenseCommand,
) -> anyhow::Result<()> {
    match command {
        LicenseCommand::List => print_serializable(&list_license_receipts(takokit_root).map_err(cli_error)?)?,
        LicenseCommand::Show { license } => {
            let receipts = list_license_receipts(takokit_root)
                .map_err(cli_error)?
                .into_iter()
                .filter(|receipt| receipt.license_id.eq_ignore_ascii_case(&license))
                .collect::<Vec<_>>();
            print_serializable(&receipts)?;
        }
        LicenseCommand::Revoke { license, model } => {
            let removed = revoke_license_receipt(takokit_root, &license, model.as_deref()).map_err(cli_error)?;
            print_serializable(&serde_json::json!({
                "license": license,
                "model": model,
                "revoked_receipts": removed,
            }))?;
        }
    }
    Ok(())
}
