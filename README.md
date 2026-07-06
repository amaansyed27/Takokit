# Takokit

Takokit is a Rust-first local voice AI runtime for running, cloning, training, transcribing, converting, and serving open-source voice models locally.

It is similar in spirit to Ollama, but for speech: local TTS, Whisper/STT, voice cloning, voice training, voice conversion, and OpenAI-compatible local speech APIs where practical.

## Scaffold Status

This repository is an initial commit-worthy foundation. It includes:

- Rust workspace with separated CLI, server, core, model, audio, store, and safety crates.
- Axum local API server with health, status, model, voice, and mock speech routes.
- Clap CLI with typed command handlers for serve, status, speak, pull, list, transcribe, clone, and train.
- Model registry and adapter traits for TTS, STT, cloning, training, and conversion.
- Mock TTS engine that writes a valid test WAV for API and CLI shape validation.
- Local storage layout under `~/.takokit`.
- Vite React desktop scaffold organized by features.
- Runner folders for Python, ONNX, and whisper.cpp integration planning.

No real ML inference is implemented yet.

## Architecture

```txt
apps/
  cli/                  Rust CLI binary
  desktop/              Tauri-ready Vite + React + TypeScript app
crates/
  takokit-core/         shared domain types, errors, config, API contracts
  takokit-server/       local Axum HTTP API server / daemon
  takokit-models/       model registry, metadata, adapter traits, mock TTS
  takokit-audio/        audio helpers and test WAV writing
  takokit-store/        local filesystem storage layout
  takokit-safety/       consent and policy primitives
runners/
  python/               isolated Python runner scaffold for PyTorch models
  onnx/                 future native/ONNX runner notes
  whispercpp/           future whisper.cpp runner notes
```

Python is intentionally constrained to runner processes for model families that require PyTorch. The local server is Rust/Axum.

## Setup

```bash
cargo check
cargo test
cargo run -p takokit-cli -- status
cargo run -p takokit-cli -- speak "Hello from Takokit"
cargo run -p takokit-cli -- serve
```

Desktop scaffold:

```bash
cd apps/desktop
npm install
npm run dev
npm run build
```

The desktop app is Vite/React today and structured for Tauri. The next wiring step is to add `src-tauri/` with Tauri commands that call the Rust server or launch the daemon.

## CLI

```bash
takokit serve
takokit status
takokit speak "Hello from Takokit"
takokit pull kokoro
takokit list models
takokit list voices
takokit transcribe ./audio.wav
takokit clone ./sample.wav --name myvoice
takokit train ./samples --name myvoice-v2
```

Implemented now:

- `serve` starts the local Axum API server on `127.0.0.1:5050`.
- `status` prints local runtime status as JSON.
- `speak` uses the mock TTS adapter and writes a test WAV to `~/.takokit/outputs`.
- `list models` and `list voices` print registry metadata.

Other commands return typed not-implemented errors with clear phase boundaries.

## API

Default local server:

```bash
cargo run -p takokit-cli -- serve
```

Examples:

```bash
curl http://127.0.0.1:5050/health
curl http://127.0.0.1:5050/v1/status
curl http://127.0.0.1:5050/v1/models
curl http://127.0.0.1:5050/v1/voices
curl -X POST http://127.0.0.1:5050/v1/audio/speech \
  -H "Content-Type: application/json" \
  -d '{"model":"mock-tts","input":"Hello from Takokit","voice":"default","response_format":"wav"}'
```

See [docs/api.md](docs/api.md).

## Local Storage

```txt
~/.takokit/
  models/
  voices/
  datasets/
  outputs/
  cache/
  logs/
  config.toml
```

## Project Principles

- Rust-first core, CLI, server, storage, and API contracts.
- Model-specific logic lives behind adapter traits.
- Python is a contained runner layer, not the application backend.
- UI and API never depend on model-specific implementation details.
- No hidden cloud calls.
- Voice cloning and training require explicit user action and consent-oriented flows.
- Not-yet-built features use typed errors and roadmap entries rather than scattered comments.

