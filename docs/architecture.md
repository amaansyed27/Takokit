# Architecture

Takokit is organized as:

```txt
Rust CLI
  |
Rust daemon / local HTTP API
  |
local browser GUI
  |
package registry + installed manifests
  |
model adapters + future runners
```

The GUI is a normal React + TypeScript + Vite web app in `apps/gui`. The daemon serves the built GUI at `/gui`, and `takokit gui` opens `http://127.0.0.1:5050/gui`.

## Product Surfaces

Takokit has five first-class local voice surfaces:

1. TTS: text input to speech/audio output.
2. STT: audio file/input to text transcript.
3. Voice Cloning: voice sample(s) to a reusable voice profile.
4. Live Transcription Local API: local Whisper or other STT models exposed through an API for streaming or submitted audio.
5. Live Audio API: compatible local voice models exposed through an API for speech output.

Model manifests declare these surfaces as typed capabilities. CLI, API, and GUI code should read capabilities from manifests rather than hardcoding model-specific behavior.

## Crates

- `takokit-core`: API request/response types, shared model metadata, runtime config, and typed errors.
- `takokit-server`: Axum router, daemon entry point, API handlers, and static GUI serving.
- `takokit-package`: model manifests, runner manifests, local mock registry lookup, and installed manifest registry.
- `takokit-models`: adapter traits and mock TTS implementation.
- `takokit-audio`: audio helpers and valid mock WAV writing.
- `takokit-store`: local filesystem layout under `~/.takokit`.
- `takokit-safety`: consent, license, and safety policy primitives.

## Apps

- `apps/cli`: Clap CLI. It starts the daemon, opens the GUI, and calls package/model functionality.
- `apps/gui`: Browser GUI. It calls the local API through `src/lib/api.ts`.

## Package Manager Boundary

Takokit should eventually own model and runner installation:

```txt
model manifest -> runner manifest -> content-addressed blobs -> installed registry -> adapter dispatch
```

The current implementation installs local mock manifests only. It does not download model weights, install Python packages, or execute real runners.

## Runner Resolution

Execution requests follow this flow:

```txt
model id
  -> load model manifest
  -> check requested capability
  -> resolve required runner
  -> check platform and installed runner status
  -> return execution plan or typed error
```

The resolver returns typed failures such as `ModelNotFound`, `CapabilityUnsupported`, `RunnerNotFound`, `RunnerNotInstalled`, `RunnerUnsupportedOnPlatform`, and `InferenceNotImplemented`. `mock-tts` remains the only path that writes a test WAV.

## Runner Isolation

Runner types are modeled now for future backends:

- `native`
- `onnx`
- `whispercpp`
- `python-managed`
- `external`

Runners must communicate through explicit contracts. UI and API callers should only see model IDs, voice IDs, request contracts, package metadata, and typed errors.
