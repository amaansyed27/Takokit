//! Human-readable and JSON rendering for package, plan, and runner views.

use super::*;

mod human;
pub(crate) use human::{
    json_output_requested, print_serializable, print_value, set_json_output,
};

pub(crate) fn print_or_json_plan(plan: &ModelPlan, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(plan)?);
    } else {
        print_model_plan(plan);
    }
    Ok(())
}

pub(crate) fn print_models(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let models: Vec<_> = package_registry
        .models()
        .map_err(cli_error)?
        .into_iter()
        .map(|model| {
            model_info_from_plan(package_registry, installed_registry, &model.id).map_err(cli_error)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    print_serializable(&models)
}
