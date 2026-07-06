# Architecture

Takokit is organized around a local runtime boundary:

```txt
Desktop UI / CLI
        |
Takokit Local Server
        |
Model Registry + Voice Registry
        |
Engine Adapter Traits
        |
Python / ONNX / whisper.cpp / native runners
        |
Audio outputs, transcripts, voice profiles, API responses
```

## Rust-First Runtime

The server, CLI, shared contracts, storage layer, safety rules, and registry are Rust crates. This keeps process ownership, local API behavior, and filesystem rules predictable.

Python is only a contained runner layer for PyTorch model families such as Kokoro, Chatterbox, GPT-SoVITS, and Qwen3-TTS. Future ONNX and whisper.cpp integrations should avoid Python when native execution is practical.

## Crates

- `takokit-core`: API request/response types, shared model metadata, runtime config, and typed errors.
- `takokit-server`: Axum router and daemon entry point.
- `takokit-models`: model metadata, registry, adapter traits, and mock TTS implementation.
- `takokit-audio`: audio file helpers and FFmpeg bridge location.
- `takokit-store`: local filesystem and future SQLite storage abstraction.
- `takokit-safety`: consent, license, and safety policy primitives.

## Apps

- `apps/cli`: Clap CLI. It should stay thin and call crate-level handlers.
- `apps/desktop`: Tauri-ready Vite React app. It should use feature folders and avoid model-specific internals.

## Runner Isolation

Runners should communicate through explicit request/response contracts. A runner may be a subprocess, sidecar service, or native library adapter, but the UI and API should only see Takokit adapter traits.

