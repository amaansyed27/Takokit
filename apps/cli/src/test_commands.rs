//! Test-command dispatch and single-model plan or STT execution.

mod fast;
mod launch;

use super::*;
pub(crate) use fast::run_fast_smokes;
pub(crate) use launch::{format_launch_suite, print_launch_suite, LaunchSuiteRow};

pub(crate) async fn run_test_command(
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    args: TestArgs,
) -> anyhow::Result<()> {
    if args.suite.as_deref() == Some("launch") {
        print_launch_suite(
            store,
            package_registry,
            installed_registry,
            args.json,
            args.run,
            args.file.as_deref(),
            args.category.as_deref(),
            args.include_heavy,
        )
        .await?;
        return Ok(());
    }
    if args.suite.as_deref() == Some("fast") {
        run_fast_smokes(
            store,
            package_registry,
            installed_registry,
            args.run,
            args.json,
        )
        .await?;
        return Ok(());
    }

    let Some(model) = args.model else {
        return Err(TakokitError::InvalidRequest(
            "provide a model id or --suite launch|fast".to_string(),
        )
        .into());
    };
    let plan = plan_model(package_registry, installed_registry, &model).map_err(cli_error)?;
    if let Some(file) = args.file {
        if !plan.executable {
            print_or_json_plan(&plan, args.json)?;
            return Err(anyhow::anyhow!(
                "model is not executable; missing: {}",
                plan.missing.join("; ")
            ));
        }
        let execution = resolve_execution_plan(
            package_registry,
            installed_registry,
            &model,
            CapabilityKind::SpeechToText,
        )
        .map_err(cli_error)?;
        let response = execute_transcription(
            &execution,
            takokit_core::TranscriptionRequest {
                file_path: file,
                model: Some(model),
            },
        )
        .await
        .map_err(runtime_error)?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    print_or_json_plan(&plan, args.json)?;
    if !args.json {
        println!(
            "Test result: {}",
            if plan.executable {
                "executable; provide --file <audio.wav> for a real STT smoke test when applicable"
            } else {
                "blocked"
            }
        );
    }
    Ok(())
}
