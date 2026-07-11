# Takokit

Takokit is a Rust-first local voice AI runtime, similar in spirit to Ollama, but for voice models.

Takokit is not a Tauri desktop app. It ships a Rust CLI and daemon/API. The GUI is a local web app served by the daemon and opened with:

```bash
takokit gui
```

Normal CLI operations are daemon-backed: they start and verify one managed local daemon when needed. Use `takokit --direct <command>` for in-process development; `takokit serve` is the foreground direct-development server and never claims managed ownership.

`tako` is also built as a short command alias for the same CLI. It uses the same `~/.takokit/` storage root and is not a separate product.

Target local GUI URL:

```txt
http://127.0.0.1:5050/gui
```

Real execution is available on the verified Windows x64 path for `whisper-base`, `whisper-tiny`, Kokoro ONNX TTS, and Qwen3-TTS CustomVoice TTS. Piper has verified artifacts but remains blocked by its safe text-frontend boundary.

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
- `tako` short command alias backed by the same CLI entrypoint and `~/.takokit` storage.
- `takokit doctor` setup check for storage, registry, server, GUI build, runner state, mock execution, and platform state.
- `takokit doctor --json`, `takokit plan --json <model>`, and `takokit runner doctor --json <runner>` for scriptable diagnostics.
- Axum local daemon on `127.0.0.1:5050`.
- React + TypeScript + Vite browser GUI in `apps/gui`.
- Static GUI serving from `apps/gui/dist` at `/gui`.
- Local mock model and runner registry under `registry/`.
- Curated model library metadata under `registry/library/` for future GUI and website discovery.
- Manifest-backed `pull`, `show`, `plan`, `list models`, and `list runners` command flow.
- Installed model and runner records under `~/.takokit/manifests/`.
- Checksum-backed artifact install foundation with content-addressed blobs.
- Verified Piper Lessac, Kokoro ONNX, Whisper Base/Tiny, and Qwen3-TTS artifact-backed pulls.
- Typed capability taxonomy, lifecycle states, and execution planning layer shared by CLI, API, and GUI.
- Shared runner contracts for ONNX, whisper.cpp, managed Python, Transformers audio, and NeMo.
- Explicit runner runtime install path via `takokit runner install <runner>`.
- Runner execution interface with a real Kokoro ONNX TTS adapter, Qwen3-TTS managed adapter, typed Piper boundary, and real whisper.cpp transcription adapters.
- Verified Whisper Base ggml artifact pull and Windows x64 whisper.cpp runtime install.
- Local storage layout under `~/.takokit`.
- Mock TTS engine for API-contract tests only.

## Commands

```bash
cargo run -p takokit-cli
cargo run -p takokit-cli -- serve
cargo run -p takokit-cli -- gui
cargo run -p takokit-cli -- doctor
cargo run -p takokit-cli -- version
cargo run -p takokit-cli -- capabilities
cargo run -p takokit-cli -- pull kokoro
cargo run -p takokit-cli -- pull piper-lessac
cargo run -p takokit-cli -- pull piper-lessac --metadata-only
cargo run -p takokit-cli -- show kokoro
cargo run -p takokit-cli -- plan qwen3-tts
cargo run -p takokit-cli -- runner pull takokit-onnx
cargo run -p takokit-cli -- runner install takokit-onnx
cargo run -p takokit-cli -- speak "Hello from Takokit" --model kokoro
cargo run -p takokit-cli -- runner install takokit-python-managed
cargo run -p takokit-cli -- adapter install qwen3-tts
cargo run -p takokit-cli -- pull qwen3-tts
cargo run -p takokit-cli -- speak "Hello from Takokit" --model qwen3-tts --voice Ryan
cargo run -p takokit-cli -- runner install takokit-whispercpp
cargo run -p takokit-cli -- runner show takokit-python-managed
cargo run -p takokit-cli -- test --suite launch
cargo run -p takokit-cli -- models
cargo run -p takokit-cli -- runners
cargo run -p takokit-cli -- library models
cargo run -p takokit-cli -- library runners
cargo run -p takokit-cli -- list models
cargo run -p takokit-cli -- list runners
cargo run -p takokit-cli -- speak "Hello from Takokit" --model mock-tts
cargo run -p takokit-cli -- transcribe ./audio.wav --model whisper-base
cargo run -p takokit-cli --bin tako -- doctor
```

Running bare `takokit` opens a lightweight interactive terminal launcher. It can pull models/runners, open the local web GUI, start the server, and run doctor checks. Real execution remains contingent on the planner reporting the selected model executable.

`takokit doctor` prints readable `[ok]`, `[warn]`, and `[fail]` checks for the local storage layout, registry parsing, installed record parsing, server availability, GUI build output, runner state, mock TTS availability, and platform identifier. The Python-managed layout is a non-failing warning until `takokit runner install takokit-python-managed` initializes it.

`tako doctor`, `tako pull piper-lessac`, and `tako gui` are aliases for the same commands. They still read and write `~/.takokit/`.

`takokit pull kokoro` downloads and verifies the pinned INT8 ONNX model and voices bundle. `takokit runner install takokit-onnx` creates an isolated managed environment and installs the MIT `kokoro-onnx` adapter; `takokit speak "Hello" --model kokoro` writes a real WAV under `~/.takokit/outputs/`.

`takokit pull piper-lessac` downloads the Piper Lessac medium ONNX model and config files, validates byte sizes and SHA256 checksums, stores verified blobs under `~/.takokit/blobs/sha256/`, and writes downloaded artifact records. Repeat pulls re-verify and reuse matching local artifacts without contacting their source. Use `--metadata-only` when a pull should explicitly skip artifact downloads; it never downgrades a ready installation.

`takokit runner pull <runner>` installs a runner contract record under `~/.takokit/manifests/installed-runners/`.

`takokit runner install takokit-whispercpp` downloads the official whisper.cpp Windows x64 release ZIP, verifies its SHA256, extracts it under `~/.takokit/runners/whispercpp/runtime/`, and marks the runner `ready` only when `whisper-cli.exe` is present. Other platforms currently get an honest runtime-installed state with a platform-specific blocker.

`takokit runner install takokit-python-managed` creates Takokit's isolated Python 3.12 environment. `takokit adapter install qwen3-tts` installs the official package and JSON adapter only inside that environment; users do not manually install Python, Torch, or requirements.

`takokit plan <model>` prints the model family, task, required runner, artifact state, runner contract/runtime state, whether it is executable today, missing pieces, and the next command to run.

`takokit models`, `GET /v1/models`, and the GUI Models page use the same lifecycle planning state. Whisper Base/Tiny, Kokoro, and Qwen3-TTS become executable only after their artifacts, runner, and (where applicable) adapter are ready. `piper-lessac` remains blocked by `piper_text_frontend_not_implemented`.

## Daemon-backed commands

`takokit daemon start|status|stop` manages the owned daemon. `takokit list` is shorthand for the model list; `takokit list models|runners|voices` remains available. `takokit ps` lists active speech/transcription executions (runners are one-shot, so idle is empty). `takokit run <model> "text" [--voice <voice>]` dispatches TTS, while `takokit run <model> --file <audio>` dispatches STT through the same API as `speak` and `transcribe`.

`takokit pull <model>` installs the model artifacts and its required managed runner components. For example:

```bash
takokit pull kokoro
takokit run kokoro "Hello from Takokit"

takokit pull whisper-tiny
takokit run whisper-tiny --file sample.wav
```

Use `--metadata-only` to record a model declaration without initializing runtimes or downloading weights. On an already ready model it is an idempotent no-op that preserves executable state.

`takokit library models` and `takokit library runners` print curated discovery metadata. Library entries are not automatically executable runtime manifests and do not trigger downloads.

`takokit speak "Hello" --model kokoro` and `takokit speak "Hello" --model qwen3-tts --voice Ryan` write real local WAVs after their explicit install/pull lifecycle. Package models always resolve an execution plan first. Piper resolves verified artifacts/config and fails at typed `piper_text_frontend_not_implemented` instead of pretending to run.

`takokit transcribe ./audio.wav --model whisper-base` can produce a real transcript after:

```bash
takokit pull whisper-base
takokit runner pull takokit-whispercpp
takokit runner install takokit-whispercpp
```

The GUI Transcribe page uses the same `/v1/audio/transcriptions` path. Provide a local audio file path that the daemon can read; Takokit does not upload the file to a cloud service.

## Local Testing

The shortest fresh local flow is:

```bash
cargo build --release
target/release/takokit.exe version
target/release/takokit.exe doctor
target/release/takokit.exe models
target/release/takokit.exe runners
target/release/takokit.exe plan whisper-base
target/release/takokit.exe runner pull takokit-whispercpp
target/release/takokit.exe runner install takokit-whispercpp
target/release/takokit.exe pull whisper-base
target/release/takokit.exe transcribe ./sample.wav --model whisper-base
target/release/takokit.exe test --suite launch
target/release/takokit.exe test --suite launch --json
target/release/tako.exe doctor
```

Use `TAKOKIT_HOME` for isolated tests:

```powershell
$env:TAKOKIT_HOME="$env:TEMP\takokit-smoke"
target/release/takokit.exe doctor --json
```

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
shared runners and model adapters
```

Users should not manually install random model dependencies. The intended UX is:

```bash
takokit capabilities
takokit pull kokoro
takokit show kokoro
takokit plan kokoro
takokit runner pull takokit-onnx
takokit list runners
takokit speak "Hello" --model kokoro
takokit transcribe ./audio.wav --model whisper-base
takokit gui
```

Today, Whisper Base/Tiny, Kokoro ONNX, and Qwen3-TTS have verified local execution paths. Piper, Transformers-audio, NeMo, and all cloning/conversion models remain blocked or metadata-only with explicit reasons; no other registry model is marked executable.

Execution planning and execution are separate:

```txt
model manifest + installed records -> ExecutionPlan
ExecutionPlan + runner engine -> output or typed execution error
```

The first ONNX target decision is documented in [docs/decisions/0001-first-onnx-model.md](docs/decisions/0001-first-onnx-model.md). Kokoro is now the first verified ONNX execution path; Piper remains blocked until a compatible non-GPL frontend is proven.

The ONNX runner now has a Piper planning scaffold: it resolves the installed Lessac model/config artifact paths from the installed model record and parses the Piper JSON config into typed Rust structs. Actual ONNX inference is still not implemented.

Artifact behavior is documented in [docs/artifacts.md](docs/artifacts.md). Runner contracts are documented in [docs/runners.md](docs/runners.md). Source and licensing notes for Piper are tracked in [docs/references.md](docs/references.md). The old `rhasspy/piper` repo is archived and the current `OHF-Voice/piper1-gpl` runtime is GPL-3.0; Takokit must not vendor GPL runtime code without an explicit licensing decision.

Curated discovery metadata is documented in [docs/library.md](docs/library.md). It is intentionally separate from executable runtime manifests.

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
    python-managed/
      runtime/
      env/
      packages/
      wheels/
      logs/
      manifests/
      cache/
      adapters/
  blobs/
    sha256/
  manifests/
    models/
    runners/
    installed-models/
    installed-runners/
  voices/
  datasets/
  outputs/
  cache/
    downloads/
  logs/
  config.toml
```

Set `TAKOKIT_HOME` to use a temporary or test storage root. If it is unset, both `takokit` and `tako` use `~/.takokit/`.

## Project Principles

- Rust-first core, CLI, daemon, storage, and API contracts.
- Browser GUI, not Tauri.
- No hidden cloud calls.
- No manual Python/PyTorch/FFmpeg/repo-clone setup for users.
- Model and runner setup belongs in package/runner management.
- UI and CLI must not hardcode model-specific behavior.
- Not-yet-built features return typed errors instead of fake inference claims.
## Fast local use (Windows)

```powershell
cargo build --release
target\release\takokit.exe quickstart
target\release\takokit.exe samples create
target\release\takokit.exe speak "Hello from Takokit" --model kokoro
target\release\takokit.exe transcribe "$env:USERPROFILE\.takokit\samples\hello.wav" --model whisper-tiny
target\release\takokit.exe test --suite fast --run
target\release\takokit.exe gui
```

Fast usable models: Kokoro for TTS and Whisper Tiny/Base for STT. Qwen3-TTS is the optional heavy path. See [docs/quickstart.md](docs/quickstart.md) and [docs/local-testing.md](docs/local-testing.md).

Still blocked: Piper Lessac, OpenVoice, RVC, NeMo/Parakeet/Canary, and transformers-audio/Qwen-Omni/Voxtral.
