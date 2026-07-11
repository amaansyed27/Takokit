//! Local dependency diagnostics, quickstart, and sample creation.

use super::*;

pub(crate) fn validate_run_args(args: &RunArgs) -> anyhow::Result<()> {
    if args.text.is_some() == args.file.is_some() {
        return Err(anyhow::anyhow!(
            "provide text for TTS or --file for STT, but not both"
        ));
    }
    Ok(())
}

pub(crate) fn print_adapter_doctor(
    store: &LocalStore,
    record: &takokit_package::AdapterRecord,
    json: bool,
) -> anyhow::Result<()> {
    let adapter_dir = store.python_managed_adapters_dir().join(&record.id);
    let python = if cfg!(windows) {
        store
            .python_managed_env_dir()
            .join("venv")
            .join("Scripts")
            .join("python.exe")
    } else {
        store
            .python_managed_env_dir()
            .join("venv")
            .join("bin")
            .join("python3")
    };
    let payload = serde_json::json!({
        "id": record.id,
        "model_family": record.model_family,
        "state": record.state,
        "notes": record.notes,
        "adapter_path": adapter_dir,
        "adapter_script": adapter_dir.join("qwen3_tts.py"),
        "python": python,
        "python_present": python.is_file(),
        "log_path": adapter_dir.join("install.log"),
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("Adapter Doctor: {} ({})", record.id, record.model_family);
        println!("state: {}", record.state);
        println!("python: {}", python.display());
        println!("python present: {}", yes_no(python.is_file()));
        println!("adapter path: {}", adapter_dir.display());
        println!("log path: {}", adapter_dir.join("install.log").display());
        println!("note: {}", record.notes);
    }
    Ok(())
}

pub(crate) fn print_deps_doctor(store: &LocalStore) {
    let uv = find_uv(store.root());
    println!("Dependency doctor");
    match uv {
        Some(path) => println!("uv: ready ({})", path.display()),
        None => {
            println!("uv: missing");
            println!("next: takokit deps bootstrap");
            println!(
                "log: {}",
                store.logs_dir().join("uv-bootstrap.log").display()
            );
        }
    }
    println!(
        "managed Python: {}",
        if store.python_managed_env_dir().join("venv").is_dir() {
            "present"
        } else {
            "missing"
        }
    );
    println!("storage: {}", store.root().display());
}

pub(crate) async fn run_quickstart(
    store: &LocalStore,
    registry: &PackageRegistry,
    installed: &InstalledRegistry,
    full: bool,
) -> anyhow::Result<()> {
    println!("Takokit fast local setup");
    let uv = bootstrap_uv(store.root()).map_err(cli_error)?;
    println!("Dependency bootstrap: ready ({})", uv.display());
    for runner in ["takokit-onnx", "takokit-whispercpp"] {
        let manifest = registry.runner(runner).map_err(cli_error)?;
        installed.install_runner(&manifest).map_err(cli_error)?;
        initialize_runner_runtime(store.root(), installed, &manifest).map_err(cli_error)?;
    }
    for model in ["kokoro", "whisper-tiny"] {
        let manifest = registry.model(model).map_err(cli_error)?;
        installed.install_model(&manifest).map_err(cli_error)?;
    }
    if full {
        let manifest = registry
            .runner("takokit-python-managed")
            .map_err(cli_error)?;
        installed.install_runner(&manifest).map_err(cli_error)?;
        initialize_runner_runtime(store.root(), installed, &manifest).map_err(cli_error)?;
        let record = install_python_adapter(store.root(), "qwen3_tts").map_err(cli_error)?;
        println!("Qwen adapter: {}", record.state);
        let manifest = registry.model("qwen3-tts").map_err(cli_error)?;
        installed.install_model(&manifest).map_err(cli_error)?;
    }
    // This is intentionally an execution test, not a lifecycle check.  It
    // creates speech with Kokoro and transcribes that exact WAV with Whisper.
    run_fast_smokes(store, registry, installed, true, false).await?;
    for (label, model) in [
        ("Kokoro TTS", "kokoro"),
        ("Whisper Tiny STT", "whisper-tiny"),
    ] {
        let plan = plan_model(registry, installed, model).map_err(cli_error)?;
        println!(
            "{label}: {}",
            if plan.executable {
                "ready".to_string()
            } else {
                format!("blocked with reason: {}", plan.missing.join("; "))
            }
        );
    }
    println!(
        "GUI: {}",
        if takokit_server::router::gui_dist_path()
            .join("index.html")
            .is_file()
        {
            "available"
        } else {
            "build missing"
        }
    );
    println!("Storage: {}", store.root().display());
    println!("Next:\n  takokit speak \"Hello from Takokit\" --model kokoro\n  takokit samples create\n  takokit transcribe {} --model whisper-tiny\n  takokit gui", store.root().join("samples").join("hello.wav").display());
    Ok(())
}

pub(crate) async fn create_samples(
    store: &LocalStore,
    registry: &PackageRegistry,
    installed: &InstalledRegistry,
) -> anyhow::Result<()> {
    let samples = store.root().join("samples");
    std::fs::create_dir_all(&samples)?;
    let hello = samples.join("hello.wav");
    let silence = samples.join("silence.wav");
    let plan = plan_model(registry, installed, "kokoro").map_err(cli_error)?;
    if plan.executable {
        match resolve_execution_plan(registry, installed, "kokoro", CapabilityKind::TextToSpeech)
            .map_err(cli_error)
        {
            Ok(execution) => match execute_speech(
                &execution,
                SpeechRequest {
                    model: "kokoro".to_string(),
                    input: "Hello from Takokit.".to_string(),
                    voice: Some("af_heart".to_string()),
                    response_format: Some("wav".to_string()),
                },
                &samples,
            )
            .await
            {
                Ok(response) => {
                    std::fs::copy(response.output_path, &hello)?;
                    println!("hello.wav: Kokoro speech ({})", hello.display());
                }
                Err(error) => return Err(runtime_error(error)),
            },
            Err(error) => return Err(error),
        }
    } else {
        return Err(anyhow::anyhow!(
            "Kokoro is not executable; refusing to create hello.wav from silence"
        ));
    }
    write_silence_wav(&silence, 500, WavSpec::default())?;
    println!("silence.wav: {}", silence.display());
    Ok(())
}
