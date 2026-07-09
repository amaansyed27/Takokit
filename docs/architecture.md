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
- `takokit-package`: model manifests, runner manifests, local mock registry lookup, installed manifest registry, and execution planning.
- `takokit-models`: adapter traits, runner execution traits, ONNX runner scaffold, and mock TTS implementation.
- `takokit-audio`: audio helpers and valid mock WAV writing.
- `takokit-store`: local filesystem layout under `~/.takokit`.
- `takokit-safety`: consent, license, and safety policy primitives.

## Apps

- `apps/cli`: Clap CLI. Bare `takokit` opens a lightweight interactive terminal launcher. `tako` is a short binary alias for the same entrypoint and storage root. Subcommands start the daemon, open the GUI, run doctor checks, and call package/model functionality.
- `apps/gui`: Browser GUI. It calls the local API through `src/lib/api.ts`.

## Local Launcher And Doctor

Running `takokit` without a subcommand opens a simple terminal launcher for common local actions: mock speech generation, model metadata pulls, runner contract pulls, GUI launch, server startup, package listing, and doctor checks. The launcher is honest about current execution limits and does not claim real Kokoro, Whisper, voice cloning, training, or conversion inference works yet.

Running `tako` invokes the same CLI implementation. It intentionally continues to use `~/.takokit/`, not a separate `~/.tako/` tree.

`takokit doctor` checks storage directories, `config.toml`, local registry availability, model and runner manifest parsing, installed model and runner record parsing, server availability, GUI build output, `mock-tts` availability, and the platform identifier. Missing GUI build output and unimplemented real runners are warnings, not fatal errors for current mock/runtime development.

## Package Manager Boundary

Takokit should eventually own model and runner installation:

```txt
model manifest -> runner manifest -> content-addressed blobs -> installed registry -> adapter dispatch
```

The current implementation installs local mock metadata for models without verified artifacts. For artifact-backed manifests, pull is explicit and the package layer downloads to `cache/downloads`, verifies SHA256, then installs into `blobs/sha256/<hash>` before writing installed artifact records.

It writes:

```txt
~/.takokit/cache/downloads/
~/.takokit/blobs/sha256/<hash>
~/.takokit/manifests/models/<model>.toml
~/.takokit/manifests/runners/<runner>.toml
~/.takokit/manifests/installed-models/<model>.toml
~/.takokit/manifests/installed-runners/<runner>.toml
```

The `models/` and `runners/` manifest copies preserve the existing behavior. The `installed-*` records track lifecycle metadata such as source, installed time, artifact URL/checksum/role/local path, required runner, platforms, and status. Takokit still does not install Python packages or execute real runners.

Artifact install refuses missing URLs or checksums unless the manifest or request is explicitly metadata-only. Checksum mismatches delete the temporary download and return a typed error. See [artifacts.md](artifacts.md).

## Execution Planning And Runner Execution

Execution requests follow this flow:

```txt
model id
  -> load model manifest
  -> check model install record
  -> check requested capability
  -> resolve required runner
  -> check platform and installed runner status
  -> return ExecutionPlan
  -> execute plan through runner engine
  -> return output or typed execution error
```

Planning belongs to `takokit-package`. It returns typed planning failures such as `ModelNotFound`, `ModelNotInstalled`, `CapabilityUnsupported`, `RunnerNotFound`, `RunnerNotInstalled`, and `RunnerUnsupportedOnPlatform`.

Execution belongs to the model/runner layer. `takokit-models` exposes `SpeechRunner` and `TranscriptionRunner` traits plus dispatcher helpers. Execution plans include the installed model record so runner code can inspect verified artifact paths without doing storage discovery itself. The current ONNX runner scaffold returns typed `InferenceNotImplemented` with the message:

```txt
ONNX runner contract resolved, but real ONNX execution is not implemented yet.
```

`mock-tts` remains the only path that writes a test WAV.

## Runner Isolation

Runner types are modeled now for future backends:

- `native`
- `onnx`
- `whispercpp`
- `python-managed`
- `external`

Runners must communicate through explicit contracts. UI and API callers should only see model IDs, voice IDs, request contracts, package metadata, and typed errors.

## First ONNX Target

The first real ONNX model target is Piper ONNX, documented in [decisions/0001-first-onnx-model.md](decisions/0001-first-onnx-model.md). Piper is the shortest path to one real model artifact manifest, checksum-backed artifact download, and local ONNX execution. Kokoro ONNX remains the next TTS target after the Piper runner path is proven.

`piper-lessac` records verified Piper Lessac medium ONNX model/config artifacts. Pulling it downloads and verifies those files, but ONNX execution still returns `inference_not_implemented`.

The Piper ONNX scaffold resolves the installed `en_US-lessac-medium.onnx` and `en_US-lessac-medium.onnx.json` blob paths, parses the Piper JSON config, and then stops before inference.

## Installer Scaffolds

The repository includes `scripts/install.sh` and `scripts/install.ps1` as safe future-release scaffolds. They detect OS/architecture and describe the planned GitHub Releases artifact/checksum/install flow, but they do not download binaries until real release artifacts and checksums exist.
