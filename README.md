# Takokit

Takokit is a Rust-first local voice AI runtime, similar in spirit to Ollama, but for voice models.

Takokit is not a Tauri desktop app. It ships a Rust CLI and daemon/API. The GUI is a local web app served by the daemon and opened with:

```bash
takokit gui
```

Target local GUI URL:

```txt
http://127.0.0.1:5050/gui
```

No real ML inference is implemented yet. The current speech path is a deterministic mock WAV generator for API and CLI testing.

## Product Surfaces

Takokit models declare which local voice surfaces they support:

1. TTS: text input to speech/audio output.
2. STT: audio file/input to text transcript.
3. Voice Cloning: voice sample(s) to a reusable voice profile.
4. Live Transcription Local API: local STT models exposed through an API for streaming or submitted audio.
5. Live Audio API: compatible local voice models exposed through an API for speech output.

## What Exists

- Rust workspace with separated CLI, server, core, model, package, audio, store, and safety crates.
- Bare `takokit` interactive terminal launcher for common local actions.
- `takokit doctor` setup check for storage, registry, server, GUI build, mock execution, and platform state.
- Axum local daemon on `127.0.0.1:5050`.
- React + TypeScript + Vite browser GUI in `apps/gui`.
- Static GUI serving from `apps/gui/dist` at `/gui`.
- Local mock model and runner registry under `registry/`.
- Manifest-backed `pull`, `show`, `list models`, and `list runners` command flow.
- Installed model and runner records under `~/.takokit/manifests/`.
- Typed capability taxonomy and runner resolution layer.
- Local storage layout under `~/.takokit`.
- Mock TTS engine for `mock-tts` only.

## Commands

```bash
cargo run -p takokit-cli
cargo run -p takokit-cli -- serve
cargo run -p takokit-cli -- gui
cargo run -p takokit-cli -- doctor
cargo run -p takokit-cli -- capabilities
cargo run -p takokit-cli -- pull kokoro
cargo run -p takokit-cli -- show kokoro
cargo run -p takokit-cli -- runner pull takokit-onnx
cargo run -p takokit-cli -- runner show takokit-onnx
cargo run -p takokit-cli -- models
cargo run -p takokit-cli -- runners
cargo run -p takokit-cli -- list models
cargo run -p takokit-cli -- list runners
cargo run -p takokit-cli -- speak "Hello from Takokit" --model mock-tts
cargo run -p takokit-cli -- transcribe ./audio.wav --model whisper-base
```

Running bare `takokit` opens a lightweight interactive terminal launcher. It can generate speech through `mock-tts`, pull model metadata, pull runner contracts, open the local web GUI, start the server, and run doctor checks. It does not imply Kokoro, Whisper, or other real model execution works yet.

`takokit doctor` prints readable `[ok]`, `[warn]`, and `[fail]` checks for the local storage layout, mock registry parsing, installed record parsing, server availability, GUI build output, mock TTS availability, and platform identifier. Warnings such as missing GUI build output or real runners not being implemented do not fail the current mock/runtime development path.

`takokit pull kokoro` currently installs local mock registry metadata into `~/.takokit/manifests/models/` and writes an installed-model record under `~/.takokit/manifests/installed-models/`. It does not download weights or enable real Kokoro inference yet.

`takokit runner pull takokit-onnx` installs a runner contract record under `~/.takokit/manifests/installed-runners/`. It does not download or install an execution binary yet.

`takokit speak "Hello" --model mock-tts` is the only execution path that writes audio. Package models such as `kokoro`, `piper-lessac`, and `whisper-base` go through runner resolution and return typed errors until their real runners exist.

## GUI Development

```bash
cd apps/gui
npm install
npm run dev
npm run build
```

For normal local CLI usage, build the GUI and let the Rust server serve `apps/gui/dist`:

```bash
cd apps/gui
npm run build
cd ../..
cargo run -p takokit-cli -- gui
```

## Architecture

```txt
Rust CLI
  |
Rust daemon/API
  |
local browser GUI at /gui
  |
package registry + installed manifests
  |
model adapters + future runners
```

Users should not manually install random model dependencies. The intended UX is:

```bash
takokit capabilities
takokit pull kokoro
takokit show kokoro
takokit runner pull takokit-onnx
takokit list runners
takokit speak "Hello" --model kokoro
takokit transcribe ./audio.wav --model whisper-base
takokit gui
```

Today, model and runner lifecycle metadata works. Real-model speech and transcription commands still return typed `inference_not_implemented` errors once their model and runner records exist because real runners are not wired. That is intentional.

## Installer Scaffolds

Future release distribution is planned to support:

```bash
curl -fsSL https://takokit.com/install.sh | sh
```

```powershell
irm https://takokit.com/install.ps1 | iex
```

Today, `scripts/install.sh` and `scripts/install.ps1` are safe scaffolds only. They detect OS/architecture and print the future GitHub Releases artifact flow, but they do not download nonexistent binaries or claim an install succeeded.

## Local Storage

```txt
~/.takokit/
  models/
  runners/
  blobs/
  manifests/
    models/
    runners/
    installed-models/
    installed-runners/
  voices/
  datasets/
  outputs/
  cache/
  logs/
  config.toml
```

## Project Principles

- Rust-first core, CLI, daemon, storage, and API contracts.
- Browser GUI, not Tauri.
- No hidden cloud calls.
- No manual Python/PyTorch/FFmpeg/repo-clone setup for users.
- Model and runner setup belongs in package/runner management.
- UI and CLI must not hardcode model-specific behavior.
- Not-yet-built features return typed errors instead of fake inference claims.
