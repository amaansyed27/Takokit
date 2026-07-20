//! Lightweight real-inference smoke suite.

use super::*;

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
                        voice: if id == "qwen3-tts" {
                            Some("Ryan".to_string())
                        } else {
                            None
                        },
                        response_format: Some("wav".to_string()),
                        language: None,
                        instruction: None,
                        reference_text: None,
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
                    rows.push(FastSuiteRow::failed(
                        id,
                        "Kokoro did not produce the required spoken hello.wav; refusing to transcribe silence".to_string(),
                        started.elapsed().as_millis(),
                        store,
                    ));
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
                    Ok(response) if response.text.trim().is_empty() => rows.push(
                        FastSuiteRow::failed(
                            id,
                            "transcript was empty".to_string(),
                            started.elapsed().as_millis(),
                            store,
                        ),
                    ),
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
struct FastSuiteRow {
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
