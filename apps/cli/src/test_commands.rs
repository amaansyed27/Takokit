//! Fast and launch-suite execution plus report construction.

use super::*;

pub(crate) async fn run_test_command(
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    args: TestArgs,
) -> anyhow::Result<()> {
    if args.suite.as_deref() == Some("launch") {
        print_launch_suite(package_registry, installed_registry, args.json, args.run).await?;
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

pub(crate) async fn run_fast_smokes(
    store: &LocalStore,
    registry: &PackageRegistry,
    installed: &InstalledRegistry,
    run: bool,
    json: bool,
) -> anyhow::Result<()> {
    let mut rows = Vec::new();
    let phrase = "Hello from Takokit.";
    let mut spoken_sample: Option<PathBuf> = None;
    for id in ["kokoro", "whisper-tiny", "whisper-base", "qwen3-tts"] {
        let plan = plan_model(registry, installed, id).map_err(cli_error)?;
        if !plan.executable {
            rows.push(FastSuiteRow::skipped(
                id,
                if plan.missing.is_empty() {
                    "not installed".to_string()
                } else {
                    plan.missing.join("; ")
                },
                store,
            ));
            continue;
        }
        if !run {
            rows.push(FastSuiteRow::ready(id, store));
            continue;
        }
        let started = Instant::now();
        match id {
            "kokoro" | "qwen3-tts" => {
                let plan =
                    resolve_execution_plan(registry, installed, id, CapabilityKind::TextToSpeech)
                        .map_err(cli_error)?;
                match execute_speech(
                    &plan,
                    SpeechRequest {
                        model: id.to_string(),
                        input: phrase.to_string(),
                        voice: None,
                        response_format: Some("wav".to_string()),
                    },
                    &store.outputs_dir(),
                )
                .await
                {
                    Ok(response) => {
                        if id == "kokoro" {
                            let sample_dir = store.root().join("samples");
                            std::fs::create_dir_all(&sample_dir)?;
                            let sample = sample_dir.join("hello.wav");
                            std::fs::copy(&response.output_path, &sample)?;
                            spoken_sample = Some(sample);
                        }
                        rows.push(FastSuiteRow::passed(
                            id,
                            Some(response.output_path),
                            None,
                            started.elapsed().as_millis(),
                            store,
                        ));
                    }
                    Err(error) => rows.push(FastSuiteRow::failed(
                        id,
                        error.to_string(),
                        started.elapsed().as_millis(),
                        store,
                    )),
                }
            }
            _ => {
                let Some(sample) = spoken_sample.clone() else {
                    rows.push(FastSuiteRow::failed(id, "Kokoro did not produce the required spoken hello.wav; refusing to transcribe silence".to_string(), started.elapsed().as_millis(), store));
                    continue;
                };
                let plan =
                    resolve_execution_plan(registry, installed, id, CapabilityKind::SpeechToText)
                        .map_err(cli_error)?;
                match execute_transcription(
                    &plan,
                    takokit_core::TranscriptionRequest {
                        file_path: sample.clone(),
                        model: Some(id.to_string()),
                    },
                )
                .await
                {
                    Ok(response) if response.text.trim().is_empty() => {
                        rows.push(FastSuiteRow::failed(
                            id,
                            "transcript was empty".to_string(),
                            started.elapsed().as_millis(),
                            store,
                        ))
                    }
                    Ok(response) => rows.push(FastSuiteRow::passed(
                        id,
                        Some(sample),
                        Some(response.text),
                        started.elapsed().as_millis(),
                        store,
                    )),
                    Err(error) => rows.push(FastSuiteRow::failed(
                        id,
                        error.to_string(),
                        started.elapsed().as_millis(),
                        store,
                    )),
                }
            }
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!("Fast smoke test");
        for row in &rows {
            println!("{:<13} {:<7} {}", row.model, row.status, row.detail());
        }
    }
    if run && rows.iter().any(|row| row.status == "failed") {
        return Err(anyhow::anyhow!(
            "fast suite has executable model failures; inspect the JSON rows and logs"
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct FastSuiteRow {
    model: String,
    status: String,
    output_path: Option<PathBuf>,
    transcript: Option<String>,
    duration_ms: Option<u128>,
    log_path: PathBuf,
    error: Option<String>,
}
impl FastSuiteRow {
    fn skipped(model: &str, reason: String, store: &LocalStore) -> Self {
        Self {
            model: model.to_string(),
            status: "skipped".to_string(),
            output_path: None,
            transcript: None,
            duration_ms: None,
            log_path: store.logs_dir(),
            error: Some(reason),
        }
    }
    fn ready(model: &str, store: &LocalStore) -> Self {
        Self {
            model: model.to_string(),
            status: "ready".to_string(),
            output_path: None,
            transcript: None,
            duration_ms: None,
            log_path: store.logs_dir(),
            error: None,
        }
    }
    fn passed(
        model: &str,
        output_path: Option<PathBuf>,
        transcript: Option<String>,
        duration_ms: u128,
        store: &LocalStore,
    ) -> Self {
        Self {
            model: model.to_string(),
            status: "passed".to_string(),
            output_path,
            transcript,
            duration_ms: Some(duration_ms),
            log_path: store.logs_dir(),
            error: None,
        }
    }
    fn failed(model: &str, error: String, duration_ms: u128, store: &LocalStore) -> Self {
        Self {
            model: model.to_string(),
            status: "failed".to_string(),
            output_path: None,
            transcript: None,
            duration_ms: Some(duration_ms),
            log_path: store.logs_dir(),
            error: Some(error),
        }
    }
    fn detail(&self) -> String {
        self.error
            .clone()
            .or_else(|| {
                self.output_path
                    .as_ref()
                    .map(|path| format!("output={}", path.display()))
            })
            .or_else(|| {
                self.transcript
                    .as_ref()
                    .map(|text| format!("transcript={text:?}"))
            })
            .unwrap_or_default()
    }
}

pub(crate) async fn print_launch_suite(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    json: bool,
    run: bool,
) -> anyhow::Result<()> {
    let ids = [
        "piper-lessac",
        "kokoro",
        "whisper-base",
        "whisper-tiny",
        "qwen3-tts",
        "chatterbox",
        "f5-tts",
        "sensevoice",
        "parakeet",
        "canary",
        "openvoice",
        "rvc",
    ];
    let mut rows = Vec::new();
    for id in ids {
        match plan_model(package_registry, installed_registry, id) {
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
                model: id.to_string(),
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
        run_launch_smokes(package_registry, installed_registry, &mut rows).await;
    }
    println!("{}", format_launch_suite(&rows, json)?);
    Ok(())
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

pub(crate) async fn run_launch_smokes(
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
    rows: &mut [LaunchSuiteRow],
) {
    for row in rows {
        if !row.executable.unwrap_or(false) {
            row.run_result =
                Some("skipped: model is blocked by its recorded lifecycle state".to_string());
            continue;
        }
        match row.model.as_str() {
            "kokoro" | "qwen3-tts" => {
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
                            voice: None,
                            response_format: Some("wav".to_string()),
                        },
                        &installed_registry.storage_root().join("outputs"),
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
            "whisper-base" | "whisper-tiny" => {
                let output_dir = installed_registry.storage_root().join("outputs");
                let sample = output_dir.join("launch-whisper-silence.wav");
                let result = write_silence_wav(&sample, 500, WavSpec::default())
                    .map_err(anyhow::Error::from)
                    .and_then(|_| {
                        resolve_execution_plan(
                            package_registry,
                            installed_registry,
                            &row.model,
                            CapabilityKind::SpeechToText,
                        )
                        .map_err(cli_error)
                    });
                let result = match result {
                    Ok(plan) => execute_transcription(
                        &plan,
                        takokit_core::TranscriptionRequest {
                            file_path: sample,
                            model: Some(row.model.clone()),
                        },
                    )
                    .await
                    .map(|response| {
                        format!(
                            "passed: whisper.cpp completed; transcript={:?}",
                            response.text
                        )
                    })
                    .map_err(runtime_error),
                    Err(error) => Err(error),
                };
                row.run_result = Some(match result {
                    Ok(result) => result,
                    Err(error) => format!("failed: {error}"),
                });
            }
            _ => {
                row.run_result = Some(
                    "skipped: no smoke handler is declared for this executable model".to_string(),
                );
            }
        }
    }
}

pub(crate) fn format_launch_suite(rows: &[LaunchSuiteRow], json: bool) -> anyhow::Result<String> {
    if json {
        return Ok(serde_json::to_string_pretty(rows)?);
    }

    let mut output = String::from("Launch test suite\n");
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
