# Roadmap

## Phase 1: Core Runtime

- Complete Rust CLI and Axum server scaffold.
- Keep mock TTS adapter for contract testing.
- Add robust config loading from `~/.takokit/config.toml`.
- Add structured logs.

## Phase 2: TTS Runner

- Add Kokoro Python runner.
- Add Piper ONNX runner.
- Generate real WAV output through adapter traits.
- Add audio preview/export in desktop.

## Phase 3: Transcription

- Add Whisper or whisper.cpp adapter.
- Implement `/v1/audio/transcriptions`.
- Add transcript output and dataset-preparation primitives.

## Phase 4: Voice Cloning

- Add Chatterbox or GPT-SoVITS runner.
- Add voice profile storage.
- Add consent capture and visible safety checks.

## Phase 5: Training

- Add multi-sample dataset handling.
- Add transcription/cleaning flow.
- Add GPT-SoVITS few-shot training jobs.
- Add progress and artifact tracking.

## Phase 6: Voice Design And Conversion

- Add Qwen3-TTS for voice design and streaming.
- Add RVC for conversion.
- Define streaming API response contracts.

## Phase 7: Desktop Hardening

- Add Tauri `src-tauri/` wiring.
- Add daemon launch/stop controls.
- Add logs and hardware status.
- Add settings persistence.

