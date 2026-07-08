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

## What Exists

- Rust workspace with separated CLI, server, core, model, package, audio, store, and safety crates.
- Axum local daemon on `127.0.0.1:5050`.
- React + TypeScript + Vite browser GUI in `apps/gui`.
- Static GUI serving from `apps/gui/dist` at `/gui`.
- Local mock model and runner registry under `registry/`.
- Manifest-backed `pull`, `show`, `list models`, and `list runners` command flow.
- Local storage layout under `~/.takokit`.
- Mock TTS engine for `mock-tts` only.

## Commands

```bash
cargo run -p takokit-cli -- serve
cargo run -p takokit-cli -- gui
cargo run -p takokit-cli -- pull kokoro
cargo run -p takokit-cli -- show kokoro
cargo run -p takokit-cli -- list models
cargo run -p takokit-cli -- list runners
cargo run -p takokit-cli -- speak "Hello from Takokit" --model mock-tts
```

`takokit pull kokoro` currently installs the local mock registry manifest into `~/.takokit/manifests/models/`. It does not download weights or enable real Kokoro inference yet.

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
takokit pull kokoro
takokit speak "Hello" --model kokoro
takokit gui
```

Today, `speak --model kokoro` returns a typed not-implemented error because runners are not wired. That is intentional.

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
