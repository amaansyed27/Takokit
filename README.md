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
- Axum local daemon on `127.0.0.1:5050`.
- React + TypeScript + Vite browser GUI in `apps/gui`.
- Static GUI serving from `apps/gui/dist` at `/gui`.
- Local mock model and runner registry under `registry/`.
- Manifest-backed `pull`, `show`, `list models`, and `list runners` command flow.
- Typed capability taxonomy and runner resolution layer.
- Local storage layout under `~/.takokit`.
- Mock TTS engine for `mock-tts` only.

## Commands

```bash
cargo run -p takokit-cli -- serve
cargo run -p takokit-cli -- gui
cargo run -p takokit-cli -- capabilities
cargo run -p takokit-cli -- pull kokoro
cargo run -p takokit-cli -- show kokoro
cargo run -p takokit-cli -- list models
cargo run -p takokit-cli -- list runners
cargo run -p takokit-cli -- speak "Hello from Takokit" --model mock-tts
cargo run -p takokit-cli -- transcribe ./audio.wav --model whisper-base
```

`takokit pull kokoro` currently installs the local mock registry manifest into `~/.takokit/manifests/models/`. It does not download weights or enable real Kokoro inference yet.

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
takokit speak "Hello" --model kokoro
takokit transcribe ./audio.wav --model whisper-base
takokit gui
```

Today, those real-model speech and transcription commands return typed runner-resolution errors because runners are not wired. That is intentional.

## Local Storage

```txt
~/.takokit/
  models/
  runners/
  blobs/
  manifests/
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
