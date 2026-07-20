//! Complete registry-derived launch suite.

use super::*;

pub(crate) async fn print_launch_suite(
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    json: bool,
    run: bool,
    file: Option<&std::path::Path>,
    category: Option<&str>,
    include_heavy: bool,
) -> anyhow::Result<()> {
    let mut manifests = package_registry.models().map_err(cli_error)?;
    manifests.sort_by(|left, right| left.id.cmp(&right.id));

    let mut rows = Vec::new();
    for manifest in manifests {
        if !launch_category_matches(&manifest, category)? {
            continue;
        }
        let id = manifest.id.clone();
        match plan_model(package_registry, installed_registry, &id) {
            Ok(plan) => rows.push(LaunchSuiteRow {
                model: plan.model_id,
                task: Some(plan.task),
                runner: Some(plan.required_runner),
                lifecycle: Some(plan.lifecycle_state.to_string()),
                artifacts: Some(plan.artifact_state.to_string()),
                runner_runtime: Some(plan.runner_runtime_state.to_string()),
                executable: Some(plan.executable),
                missing: plan.missing,
                next_command: Some(plan.next_command),
                run_result: None,
                error: None,
            }),
            Err(error) => rows.push(LaunchSuiteRow {
                model: id,
                task: None,
                runner: None,
                lifecycle: None,
                artifacts: None,
                runner_runtime: None,
                executable: None,
                missing: Vec::new(),
                next_command: None,
                run_result: None,
                error: Some(error.to_string()),
            }),
        }
    }

    if run {
        run_launch_smokes(
            store,
            package_registry,
            installed_registry,
            &mut rows,
            file,
            include_heavy,
        )
        .await;
    }

    println!("{}", format_launch_suite(&rows, json)?);
    if run
        && rows.iter().any(|row| {
            row.error.is_some()
                || row
                    .run_result
                    .as_deref()
                    .is_some_and(|result| result.starts_with("failed:"))
        })
    {
        return Err(anyhow::anyhow!(
            "launch suite contains failures; inspect the per-model results"
        ));
    }
    Ok(())
}

fn launch_category_matches(
    manifest: &takokit_package::ModelManifest,
    category: Option<&str>,
) -> anyhow::Result<bool> {
    let Some(category) = category else {
        return Ok(true);
    };
    let matches = match category.to_ascii_lowercase().as_str() {
        "all" => true,
        "tts" => manifest.capabilities.tts,
        "stt" => manifest.capabilities.stt,
        "clone" | "cloning" => manifest.capabilities.voice_cloning,
        "train" | "training" => manifest.capabilities.voice_training,
        "convert" | "conversion" => manifest.capabilities.voice_conversion,
        "live" => manifest.capabilities.live_transcription || manifest.capabilities.live_audio,
        other => {
            return Err(TakokitError::InvalidRequest(format!(
                "unknown launch category {other:?}; use all|tts|stt|clone|train|convert|live"
            ))
            .into())
        }
    };
    Ok(matches)
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LaunchSuiteRow {
    pub(crate) model: String,
    pub(crate) task: Option<String>,
    pub(crate) runner: Option<String>,
    pub(crate) lifecycle: Option<String>,
    pub(crate) artifacts: Option<String>,
    pub(crate) runner_runtime: Option<String>,
    pub(crate) executable: Option<bool>,
    pub(crate) missing: Vec<String>,
    pub(crate) next_command: Option<String>,
    pub(crate) run_result: Option<String>,
    pub(crate) error: Option<String>,
}

async fn run_launch_smokes(
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    rows: &mut [LaunchSuiteRow],
    file: Option<&std::path::Path>,
    include_heavy: bool,
) {
    for row in rows {
        if !row.executable.unwrap_or(false) {
            row.run_result =
                Some("skipped: model is blocked by its recorded lifecycle state".to_string());
            continue;
        }

        let manifest = match package_registry.model(&row.model) {
            Ok(manifest) => manifest,
            Err(error) => {
                row.run_result = Some(format!("failed: {error}"));
                continue;
            }
        };
        if launch_model_is_heavy(&manifest) && !include_heavy {
            row.run_result = Some(
                "skipped: heavy model; rerun with --include-heavy on suitable hardware"
                    .to_string(),
            );
            continue;
        }

        if manifest.capabilities.stt {
            run_stt_smoke(package_registry, installed_registry, row, file).await;
            continue;
        }
        if manifest.capabilities.tts {
            run_tts_smoke(store, package_registry, installed_registry, row, file).await;
            continue;
        }

        row.run_result = Some(if manifest.capabilities.voice_conversion {
            "manual: run `tako convert` with consent-backed source and target inputs".to_string()
        } else if manifest.capabilities.voice_training {
            "manual: run `tako train` with a valid consent-backed dataset".to_string()
        } else if manifest.capabilities.voice_cloning {
            "manual: run `tako clone` with a consent-backed reference sample".to_string()
        } else {
            "manual: no unattended smoke handler is appropriate for this capability set"
                .to_string()
        });
    }
}

async fn run_stt_smoke(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    row: &mut LaunchSuiteRow,
    file: Option<&std::path::Path>,
) {
    let Some(audio) = file else {
        row.run_result = Some(
            "manual: provide --file <speech.wav> to run a real transcription smoke".to_string(),
        );
        return;
    };
    let result = match resolve_execution_plan(
        package_registry,
        installed_registry,
        &row.model,
        CapabilityKind::SpeechToText,
    )
    .map_err(cli_error)
    {
        Ok(plan) => execute_transcription(
            &plan,
            takokit_core::TranscriptionRequest {
                file_path: audio.to_path_buf(),
                model: Some(row.model.clone()),
            },
        )
        .await
        .and_then(|response| {
            if response.text.trim().is_empty() {
                Err(TakokitError::Execution("transcript was empty".to_string()))
            } else {
                Ok(response)
            }
        })
        .map(|response| format!("passed: transcript={:?}", response.text))
        .map_err(runtime_error),
        Err(error) => Err(error),
    };
    row.run_result = Some(match result {
        Ok(result) => result,
        Err(error) => format!("failed: {error}"),
    });
}

async fn run_tts_smoke(
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    row: &mut LaunchSuiteRow,
    file: Option<&std::path::Path>,
) {
    let reference_required = matches!(
        row.model.as_str(),
        "qwen3-tts-0.6b-base"
            | "qwen3-tts-1.7b-base"
            | "xtts-v2"
            | "yourtts"
            | "cosyvoice2"
            | "fish-speech"
            | "openvoice"
            | "gpt-sovits"
    );
    let voice = if reference_required {
        let Some(reference) = file else {
            row.run_result = Some(
                "manual: provide --file <reference.wav> for reference-conditioned speech"
                    .to_string(),
            );
            return;
        };
        Some(reference.display().to_string())
    } else if matches!(row.model.as_str(), "qwen3-tts" | "qwen3-tts-1.7b-custom") {
        Some("Ryan".to_string())
    } else {
        None
    };
    let instruction = (row.model == "qwen3-tts-1.7b-voice-design")
        .then(|| "Warm, clear, confident narration with a natural pace.".to_string());
    let reference_text = reference_required.then(|| "Hello from Takokit.".to_string());
    let result = match resolve_execution_plan(
        package_registry,
        installed_registry,
        &row.model,
        CapabilityKind::TextToSpeech,
    )
    .map_err(cli_error)
    {
        Ok(plan) => execute_speech(
            &plan,
            SpeechRequest {
                model: row.model.clone(),
                input: "Takokit launch smoke test.".to_string(),
                voice,
                response_format: Some("wav".to_string()),
                language: None,
                instruction,
                reference_text,
            },
            &store.outputs_dir(),
        )
        .await
        .map_err(runtime_error),
        Err(error) => Err(error),
    };
    row.run_result = Some(match result {
        Ok(response) => format!(
            "passed: {} ({} bytes, {} Hz)",
            response.output_path.display(),
            response.bytes,
            response
                .sample_rate
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "unknown sample rate".to_string())
        ),
        Err(error) => format!("failed: {error}"),
    });
}

fn launch_model_is_heavy(manifest: &takokit_package::ModelManifest) -> bool {
    parse_memory_gb(manifest.hardware.min_vram.as_deref()).is_some_and(|value| value >= 6)
        || parse_memory_gb(manifest.hardware.min_ram.as_deref()).is_some_and(|value| value >= 24)
}

fn parse_memory_gb(value: Option<&str>) -> Option<u32> {
    value?
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

pub(crate) fn format_launch_suite(rows: &[LaunchSuiteRow], json: bool) -> anyhow::Result<String> {
    if json {
        return Ok(serde_json::to_string_pretty(rows)?);
    }

    let mut output = format!("Launch test suite ({} models)\n", rows.len());
    for row in rows {
        if let Some(error) = &row.error {
            output.push_str(&format!("- {}: error: {error}\n", row.model));
            continue;
        }

        output.push_str(&format!(
            "- {}: lifecycle={}, runner={}, executable={}\n",
            row.model,
            row.lifecycle.as_deref().unwrap_or("unknown"),
            row.runner.as_deref().unwrap_or("unknown"),
            yes_no(row.executable.unwrap_or(false))
        ));
        if !row.missing.is_empty() {
            output.push_str(&format!("  missing: {}\n", row.missing.join("; ")));
        }
        if let Some(next) = &row.next_command {
            output.push_str(&format!("  next: {next}\n"));
        }
        if let Some(result) = &row.run_result {
            output.push_str(&format!("  run: {result}\n"));
        }
    }
    Ok(output.trim_end().to_string())
}
