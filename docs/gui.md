# Local GUI

Takokit's GUI is a local browser app served by the Rust daemon. It is not a public model-library website and it is not a Tauri desktop app.

```bash
takokit gui
```

The command starts the local API if needed, waits for it, then opens:

```txt
http://127.0.0.1:5050/gui
```

## Pages

- Home: local runtime overview and quick navigation.
- Models: runtime manifests, lifecycle state, executable yes/no, missing blockers, next commands, and model/runner actions.
- Runners: shared runner contracts, runtime state, install/doctor/remove actions, supported families, and dependency strategy.
- Library: curated discovery metadata separated from executable runtime manifests.
- Speak: local speech API flow. Only executable TTS models are enabled; `mock-tts` is clearly labeled as the internal test path. Successful responses show model id, engine, content type, byte count, sample rate, and local output path.
- Transcribe: local file-path transcription through `/v1/audio/transcriptions`, with Whisper prerequisites visible.
- Diagnostics: `/v1/doctor` checks, logs paths, runner states, and API routes.
- Settings: local runtime and storage settings.

## State Rules

The GUI consumes the same model summary fields as the CLI and API:

- `lifecycle_state`
- `runner_runtime_state`
- `executable`
- `missing`
- `next_command`
- `license_warning`

No page should show a run button for a non-executable real model. Blocked models should show the planner blocker and next command.

The Speak page does not show fake playback for blocked models. Until local output serving/playback is implemented, successful speech generation is reported as “Audio saved locally” with the returned output path.

## Development

```bash
cd apps/gui
npm install
npm run dev
npm run build
```

The production build is served from `apps/gui/dist`. If the dist is missing, `takokit doctor` reports a warning and `takokit gui` shows the build command.
