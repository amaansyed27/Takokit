# Architecture

Takokit is organized as:

```txt
Rust CLI + interactive terminal launcher
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

- `apps/cli`: Clap CLI. Bare `takokit` opens a lightweight interactive terminal launcher. Subcommands start the daemon, open the GUI, run doctor checks, and call package/model functionality.
- `apps/gui`: Browser GUI. It calls the local API through `src/lib/api.ts`.

## Local Launcher And Doctor

Running `takokit` without a subcommand opens a simple terminal launcher for common local actions: mock speech generation, model metadata pulls, runner contract pulls, GUI launch, server startup, package listing, and doctor checks. The launcher is honest about current execution limits and does not claim real Kokoro, Whisper, voice cloning, training, or conversion inference works yet.

`takokit doctor` checks storage directories, `config.toml`, local registry availability, model and runner manifest parsing, installed model and runner record parsing, server availability, GUI build output, `mock-tts` availability, and the platform identifier. Missing GUI build output and unimplemented real runners are warnings, not fatal errors for current mock/runtime development.

## Package Manager Boundary

Takokit should eventually own model and runner installation:

```txt
model manifest -> runner manifest -> content-addressed blobs -> installed registry -> adapter dispatch
```

The current implementation installs local mock metadata only. It writes:

```txt
~/.takokit/manifests/models/<model>.toml
~/.takokit/manifests/runners/<runner>.toml
~/.takokit/manifests/installed-models/<model>.toml
~/.takokit/manifests/installed-runners/<runner>.toml
```

The `models/` and `runners/` manifest copies preserve the existing behavior. The `installed-*` records track lifecycle metadata such as source, installed time, artifact placeholders, required runner, platforms, and metadata-only status. Takokit still does not download model weights, install Python packages, or execute real runners.

## Runner Resolution

Execution requests follow this flow:

```txt
model id
  -> load model manifest
  -> check requested capability
  -> check model install record
  -> resolve required runner
  -> check platform and installed runner status
  -> return typed error until execution exists
```

The resolver returns typed failures such as `ModelNotFound`, `ModelNotInstalled`, `CapabilityUnsupported`, `RunnerNotFound`, `RunnerNotInstalled`, `RunnerUnsupportedOnPlatform`, and `InferenceNotImplemented`. `mock-tts` remains the only path that writes a test WAV.

## Runner Isolation

Runner types are modeled now for future backends:

- `native`
- `onnx`
- `whispercpp`
- `python-managed`
- `external`

Runners must communicate through explicit contracts. UI and API callers should only see model IDs, voice IDs, request contracts, package metadata, and typed errors.

## Installer Scaffolds

The repository includes `scripts/install.sh` and `scripts/install.ps1` as safe future-release scaffolds. They detect OS/architecture and describe the planned GitHub Releases artifact/checksum/install flow, but they do not download binaries until real release artifacts and checksums exist.
