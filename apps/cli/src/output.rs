//! Human-readable and JSON rendering for package, plan, and runner views.

use super::*;
use std::io::IsTerminal;

mod human;
pub(crate) use human::{print_serializable, print_value};

pub(crate) fn set_json_output(enabled: bool) {
    if enabled || !std::io::stdout().is_terminal() {
        std::env::set_var("TAKOKIT_OUTPUT", "json");
    }
}

pub(crate) fn json_output_requested() -> bool {
    !std::io::stdout().is_terminal()
        || std::env::var("TAKOKIT_OUTPUT")
            .map(|value| value.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
}

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
