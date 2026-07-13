use super::app::TuiRow;

pub fn operation_rows() -> Vec<TuiRow> {
    [
        (
            "speech",
            "Generate speech",
            "TTS",
            "speak \"Hello from Takokit\" --model kokoro --voice default",
            "Generate WAV speech with any executable TTS model.",
        ),
        (
            "run",
            "Run a model",
            "TTS / STT",
            "run whisper-tiny --file \"C:\\path\\audio.wav\"",
            "Unified model execution. Supply text for TTS or --file for STT.",
        ),
        (
            "transcribe",
            "Transcribe audio",
            "STT",
            "transcribe \"C:\\path\\audio.wav\" --model whisper-tiny",
            "Transcribe a local audio file.",
        ),
        (
            "clone",
            "Clone a voice",
            "planned",
            "clone \"C:\\path\\sample.wav\" --name my-voice",
            "Consent-gated voice cloning command. The backend currently reports not implemented.",
        ),
        (
            "train",
            "Train a voice",
            "planned",
            "train \"C:\\path\\samples\" --name my-voice",
            "Voice training job command. The backend currently reports not implemented.",
        ),
        (
            "adapter-list",
            "List adapters",
            "runtime",
            "adapter list",
            "List managed Python adapter records.",
        ),
        (
            "adapter-install",
            "Install adapter",
            "runtime",
            "adapter install qwen3_tts",
            "Install a managed Python model adapter.",
        ),
        (
            "adapter-doctor",
            "Inspect adapter",
            "diagnostics",
            "adapter doctor qwen3_tts",
            "Inspect adapter state, paths, runtime and logs.",
        ),
        (
            "test-model",
            "Test one model",
            "test",
            "test whisper-tiny --run --file \"C:\\path\\audio.wav\"",
            "Run model planning or a real model smoke test.",
        ),
        (
            "test-fast",
            "Run fast suite",
            "test",
            "test --suite fast --run",
            "Run the fast executable-model suite.",
        ),
        (
            "test-launch",
            "Run launch suite",
            "test",
            "test --suite launch --run",
            "Run the complete launch readiness suite.",
        ),
        (
            "quickstart",
            "Quickstart",
            "setup",
            "quickstart",
            "Prepare Kokoro and Whisper Tiny and run smoke tests.",
        ),
        (
            "quickstart-full",
            "Full quickstart",
            "setup",
            "quickstart --full",
            "Also prepare managed Python and Qwen3-TTS.",
        ),
        (
            "deps",
            "Bootstrap dependencies",
            "setup",
            "deps bootstrap",
            "Prepare Takokit's pinned uv and managed Python tooling.",
        ),
        (
            "samples",
            "Create samples",
            "audio",
            "samples create",
            "Create real hello.wav and silence.wav fixtures.",
        ),
    ]
    .into_iter()
    .map(|(id, title, state, template, detail)| TuiRow {
        id: id.into(),
        title: title.into(),
        state: state.into(),
        detail: format!(
            "{}\n\nCommand template\n{}\n\nPress Enter to load this template into the command bar. Edit it, then press Enter again to run.",
            detail, template
        ),
        command: None,
        template: Some(template.into()),
    })
    .collect()
}

pub fn system_rows() -> Vec<TuiRow> {
    [
        ("status", "Runtime status", "read", vec!["status"]),
        ("doctor", "Doctor", "diagnostics", vec!["doctor"]),
        (
            "capabilities",
            "Capabilities",
            "read",
            vec!["capabilities"],
        ),
        ("models", "Model catalog", "read", vec!["models"]),
        ("runners", "Runner catalog", "read", vec!["runners"]),
        (
            "library-models",
            "Library models",
            "read",
            vec!["library", "models"],
        ),
        (
            "library-runners",
            "Library runners",
            "read",
            vec!["library", "runners"],
        ),
        ("voices", "Voice catalog", "read", vec!["list", "voices"]),
        ("processes", "Active executions", "read", vec!["ps"]),
        (
            "daemon-status",
            "Daemon status",
            "daemon",
            vec!["daemon", "status"],
        ),
        (
            "daemon-start",
            "Start daemon",
            "daemon",
            vec!["daemon", "start"],
        ),
        (
            "daemon-stop",
            "Stop daemon",
            "daemon",
            vec!["daemon", "stop"],
        ),
        (
            "daemon-restart",
            "Restart daemon",
            "daemon",
            vec!["daemon", "restart"],
        ),
        (
            "daemon-logs",
            "Daemon logs",
            "daemon",
            vec!["daemon", "logs"],
        ),
        (
            "deps-doctor",
            "Dependency doctor",
            "diagnostics",
            vec!["deps", "doctor"],
        ),
        ("gui", "Open GUI", "surface", vec!["gui"]),
        ("version", "Version", "read", vec!["version"]),
    ]
    .into_iter()
    .map(|(id, title, state, command)| {
        let args = command.into_iter().map(str::to_string).collect::<Vec<_>>();
        TuiRow {
            id: id.into(),
            title: title.into(),
            state: state.into(),
            detail: format!(
                "Command\n\ntakokit {}\n\nPress Enter to load it into the command bar, then press Enter again to run. Output and timing remain visible without leaving the TUI.",
                args.join(" ")
            ),
            command: Some(args),
            template: None,
        }
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_cover_execution_setup_testing_and_system_controls() {
        let operation_ids = operation_rows()
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        for expected in [
            "speech",
            "run",
            "transcribe",
            "clone",
            "train",
            "adapter-install",
            "test-fast",
            "quickstart",
            "deps",
            "samples",
        ] {
            assert!(operation_ids.iter().any(|id| id == expected));
        }

        let system_ids = system_rows()
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        for expected in ["status", "doctor", "daemon-start", "daemon-stop", "gui"] {
            assert!(system_ids.iter().any(|id| id == expected));
        }
    }
}
