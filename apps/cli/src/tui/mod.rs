mod app;
mod command;
mod ui;

use app::{App, TuiAction};
use takokit_core::RuntimeConfig;
use takokit_package::{
    initialize_runner_runtime, install_model_complete, plan_model, InstallModelOptions,
    InstalledRegistry, PackageRegistry,
};
use takokit_store::LocalStore;

use crate::{doctor, gui};

pub async fn run_launcher(
    config: &RuntimeConfig,
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<()> {
    let mut status = format!(
        "Ready. Storage: {}. Press ? for keyboard help.",
        store.root().display()
    );

    loop {
        let mut state = App::new(
            config,
            store,
            package_registry,
            installed_registry,
            status,
        )?;
        let mut selected_action = None;
        ratatui::run(|mut terminal| {
            selected_action = Some(app::run(&mut terminal, &mut state)?);
            Ok(())
        })?;

        let action = selected_action.unwrap_or(TuiAction::Quit);
        if action == TuiAction::Quit {
            return Ok(());
        }

        status = match execute_action(
            action,
            config,
            store,
            package_registry,
            installed_registry,
        )
        .await
        {
            Ok(message) => message,
            Err(error) => format!("Error: {error:#}"),
        };
    }
}

async fn execute_action(
    action: TuiAction,
    config: &RuntimeConfig,
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> anyhow::Result<String> {
    match action {
        TuiAction::Quit => Ok("Exiting Takokit.".to_string()),
        TuiAction::Refresh => Ok("Catalog and runtime state refreshed.".to_string()),
        TuiAction::PullModel(model_id) => {
            let report = install_model_complete(
                package_registry,
                installed_registry,
                store.root(),
                &model_id,
                InstallModelOptions {
                    metadata_only: false,
                },
            )?;
            Ok(format!(
                "Pulled {}. Executable: {}. Missing: {}. Logs: {}",
                report.model_id,
                yes_no(report.executable),
                if report.missing.is_empty() {
                    "none".to_string()
                } else {
                    report.missing.join("; ")
                },
                report.logs_path.display()
            ))
        }
        TuiAction::PlanModel(model_id) => {
            let plan = plan_model(package_registry, installed_registry, &model_id)?;
            Ok(format!(
                "{} [{}] · lifecycle={} · runner={} ({}) · executable={} · missing={} · next={}",
                plan.model_name,
                plan.model_id,
                plan.lifecycle_state,
                plan.required_runner,
                plan.runner_runtime_state,
                yes_no(plan.executable),
                if plan.missing.is_empty() {
                    "none".to_string()
                } else {
                    plan.missing.join("; ")
                },
                plan.next_command
            ))
        }
        TuiAction::InstallRunner(runner_id) => {
            let runner = package_registry.runner(&runner_id)?;
            if !installed_registry.is_runner_installed(&runner.id) {
                installed_registry.install_runner(&runner)?;
            }
            let report = initialize_runner_runtime(store.root(), installed_registry, &runner)?;
            Ok(format!(
                "Runner {} initialized. {} Log/manifest: {}",
                runner.id,
                report.note,
                report.manifest_path.display()
            ))
        }
        TuiAction::ShowRunner(runner_id) => {
            let runner = package_registry.runner(&runner_id)?;
            let record = installed_registry.installed_runner_record(&runner.id).ok();
            Ok(format!(
                "{} [{}] · version={} · state={} · platforms={} · families={} · {}",
                runner.name,
                runner.id,
                runner.version,
                record
                    .as_ref()
                    .map(|record| record.status.to_string())
                    .unwrap_or_else(|| "available".to_string()),
                runner.platforms.join(", "),
                runner.supported_model_families.join(", "),
                record
                    .as_ref()
                    .map(|record| record.note.as_str())
                    .unwrap_or(&runner.description)
            ))
        }
        TuiAction::Doctor => {
            let report = doctor::run_doctor(config, store, package_registry, installed_registry);
            Ok(serde_json::to_string_pretty(&report)?)
        }
        TuiAction::OpenGui => {
            gui::open_gui(store, config).await?;
            Ok(format!("GUI opened. Daemon: {}", config.local_base_url()))
        }
        TuiAction::StartServer => {
            gui::ensure_server(store, config).await?;
            Ok(format!(
                "Managed daemon is available at {}",
                config.local_base_url()
            ))
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
