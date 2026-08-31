# Takokit

Takokit is a Rust-first local voice AI runtime: an Ollama-like pull, run, inspect and remove experience for text-to-speech, speech-to-text, consent-backed voice cloning, voice conversion and local voice training.

It exposes one shared runtime through:

- `tako` / `takokit` CLI,
- a task-oriented Ratatui interface when started without a subcommand,
- a local browser GUI opened with `tako gui`,
- an Axum API on `127.0.0.1:5050`.

CLI, TUI, GUI and API share the same registry, installed records, runners, voice profiles, sessions and output locations.

## Release status

The current public stable release is Takokit `v0.2.0` for Windows x86_64. Takokit `v0.3.0` is the in-review multi-platform distribution candidate for Windows x86_64, Linux x86_64, and macOS arm64. The bundled registry currently publishes **24 model families and 31 immutable releases** across:

- local TTS,
- speech-to-text,
- zero-shot voice cloning,
- tone-colour and RVC conversion,
- GPT-SoVITS training,
- audio-language / Omni models.

Support labels remain evidence-based:

- **Locally verified** — real installation and inference were observed on a recorded device.
- **Executable path** — pull, runner, adapter and output contracts are implemented but still require the hardware smoke record.
- **Hardware-blocked** — the execution path exists, but the test device cannot satisfy the declared memory requirement.

Compilation or a manifest entry is not treated as a model pass. See [docs/model-support.md](docs/model-support.md) and [docs/MODEL_SMOKE_TESTS.md](docs/MODEL_SMOKE_TESTS.md).

## Storage

Reusable runtime data is global:

```text
~/.takokit/
├── models/
├── runners/
├── blobs/
├── cache/
├── manifests/
├── voices/
└── logs/
```

Outputs and history belong to the directory that launched Takokit:

```text
<project>/.tako/
├── active-session
├── version
└── sessions/
    └── <session-id>/
        ├── session.json
        ├── events.jsonl
        └── outputs/
```

Use `--workspace <path>` and `--session <uuid>` for explicit context.

## Build from source

Prerequisites:

- Rust stable,
- Node.js LTS and npm,
- enough disk space for the selected models,
- a compatible NVIDIA driver for CUDA models.

```powershell
cd apps\gui
npm ci
npm run build
cd ..\..

cargo build --release
.\target\release\tako.exe doctor
.\target\release\tako.exe
```

`target\release\takokit.exe` and `target\release\tako.exe` are aliases for the same application and storage.

Stop the daemon before rebuilding if Windows has locked the executable:

```powershell
.\target\release\tako.exe daemon stop
cargo build --release
```

## Core commands

```bash
tako version
tako doctor
tako status
tako capabilities

tako list                    # verified models installed on this machine
tako models                  # browse the bundled runtime catalog
tako library sync            # refresh the signed/pinned registry cache
tako library models          # browse versioned model families
tako library show whisper:small
tako runners
tako show kokoro
tako plan whisper:tiny
tako pull kokoro
tako pull whisper:tiny
tako rm whisper:tiny

tako speak "Hello from Takokit" --model kokoro
tako transcribe ./sample.wav --model whisper-tiny
tako run kokoro "Hello from the unified run command"
tako run whisper-tiny --file ./sample.wav

tako clone ./reference.wav --name "My Voice" --model chatterbox --consent
tako convert ./source.wav --target-voice ./target.wav --model openvoice --consent
tako train ./dataset --name "My Voice" --model gpt-sovits --epochs 1 --consent

tako list voices
tako sessions list
tako sessions new --title "Narration tests"
tako sessions show <session-id>
tako sessions open <session-id>
tako sessions rm <session-id>

tako daemon start
tako daemon status
tako daemon restart
tako daemon logs
tako daemon stop

tako gui
```


`tako list` is local-only: it shows models whose artifacts or pinned snapshots
are still present and verified. Metadata-only catalog entries belong to
`tako models`, not the installed-model list.

## Model library and versioned references

Takokit's curated registry is the control plane between upstream model sources and local storage. References use:

```text
[namespace/]model[:tag][@sha256:digest]
```

The default namespace is `library`, and an omitted tag resolves to the family's Takokit-verified default. `latest` means the latest default Takokit has tested and published; it does not silently chase an upstream branch.

```bash
tako library sync
tako library models
tako library show whisper:small

tako pull whisper:small
tako pull qwen3-tts:0.6b-base
tako pull library/openvoice:2
```

Legacy names remain aliases. For example, `whisper-tiny` and `whisper:tiny` resolve to the same pinned manifest, install ID and content-addressed files. The registry carries metadata and verified manifests; model bytes still download directly from the pinned upstream source. See [docs/registry.md](docs/registry.md).

## Custom voices and custom checkpoints

`tako voice add`, `tako voice list`, and `tako voice show <model>` expose reusable
consent-backed reference voices and each model's preset/reference/design contract.

`tako custom-model add <manifest.toml>` registers a pinned checkpoint only when it
extends a bundled model with a verified generic runner contract. Takokit does not execute
arbitrary scripts supplied by a model manifest. See [docs/custom-models.md](docs/custom-models.md).

## Pull lifecycle

`tako pull <model>` owns setup. Users should not clone upstream repositories, launch Gradio applications or install random Python dependencies globally.

A pull resolves:

1. a versioned registry reference, pinned manifest and capabilities,
2. the required runner contract,
3. the runner runtime,
4. an isolated adapter environment when required,
5. pinned model snapshots or checksum-backed artifacts,
6. readiness checks and actionable failure details.

Heavy Python families retain independent environments under:

```text
~/.takokit/runners/python-managed/adapters/<adapter>/venv
```

One family cannot silently mutate another family's environment.

## Model families

The current catalog includes:

- Kokoro and Piper,
- Whisper Tiny/Base/Small,
- Qwen3-TTS 0.6B and 1.7B CustomVoice/Base/VoiceDesign checkpoints,
- Chatterbox, F5-TTS, XTTS v2, YourTTS, CosyVoice2 and Fish Speech,
- Dia, Bark, MMS and Kyutai DSM TTS,
- Distil-Whisper, Wav2Vec2, SenseVoice, Voxtral, Canary and Parakeet,
- OpenVoice V2 conversion,
- RVC conversion with user-supplied checkpoints,
- GPT-SoVITS reference inference and local training,
- Qwen2.5-Omni and Qwen3-Omni audio execution paths.

Qwen3-Omni requires workstation-class memory beyond the primary 8 GB laptop GPU and must be reported as hardware-blocked there rather than falsely passed.

Primary runner contracts:

- `takokit-onnx`
- `takokit-whispercpp`
- `takokit-python-managed`
- `takokit-transformers-audio` compatibility contract
- `takokit-nemo` compatibility contract

See [docs/runners.md](docs/runners.md).

## Voice consent

Voice-profile creation, conversion and training require confirmation that the user owns the supplied voice or has explicit permission to use it.

```bash
tako clone ./reference.wav --name "My Voice" --model chatterbox --consent
tako speak "Profile reuse test" --model chatterbox --voice my-voice
```

Takokit stores reference material locally and records consent-backed operations in the active project session. CLI, TUI, GUI and API enforce the same boundary.

## TUI

Run Takokit without a subcommand:

```bash
tako
```

Primary sections:

- Models
- Speak
- Transcribe
- Clone
- Sessions
- Runners
- System

Useful shortcuts:

```text
Arrow keys          Navigate
Tab / Shift+Tab     Move through form fields
Enter               Run the visible action
/sessions           Open saved sessions
/new                Create a session
/clone              Open voice profile creation
F1                  Help
Ctrl+C               Exit safely
```

## GUI

```bash
cd apps/gui
npm ci
npm run build
cd ../..
tako gui
```

The GUI shares model state, runners, Speak, Transcribe, Voices, searchable History, diagnostics and settings with the CLI and TUI.

## Local API

Default base URL:

```text
http://127.0.0.1:5050
```

Core routes include:

```text
GET    /health
GET    /v1/status
GET    /v1/models
GET    /v1/runners
GET    /v1/voices
POST   /v1/audio/speech
POST   /v1/audio/transcriptions
POST   /v1/voices/clone
POST   /v1/sessions/open
GET    /v1/sessions
GET    /v1/sessions/:id
DELETE /v1/sessions/:id
GET    /v1/sessions/:id/outputs/:filename
```

## Architecture

```text
CLI ─────┐
TUI ─────┼── shared application/runtime services ── model runners
GUI/API ─┘                 │
                           ├── global ~/.takokit resources
                           └── project .tako sessions and outputs
```

Major workspace components:

- `takokit-core` — shared types, capabilities, sessions and errors,
- `takokit-package` — registry, pull/install, planning and runner lifecycle,
- `takokit-models` — execution adapters,
- `takokit-server` — daemon and local API,
- `takokit-store` — global resources, voices and project sessions,
- `takokit-audio` — audio utilities,
- `apps/cli` — CLI and Ratatui interface,
- `apps/gui` — React/Vite local GUI,
- `site` — public companion library, model pages, docs and registry endpoint.

## Test the complete catalog

Automated repository gates:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
python scripts/audit_file_sizes.py
cd apps/gui && npm ci && npm run build
```

Catalog planning:

```powershell
.\target\release\tako.exe test --suite launch --json
```

Full hardware runner:

```powershell
.\scripts\run_all_model_smokes.ps1 `
  -Audio C:\path\to\test01_20s.wav `
  -ReferenceAudio C:\path\to\owned-reference.wav `
  -TrainingSamples C:\path\to\gpt-sovits-dataset `
  -RvcTarget C:\path\to\owned-rvc-checkpoint
```

The repository is ready for website work only after the automated gates pass and every model has an honest `passed`, `failed`, `blocked-input` or `blocked-hardware` evidence record.

## Remaining release work

- complete real-device evidence for every advertised verified model,
- run Qwen3-Omni on suitable workstation-class hardware,
- finish production installers and signed release artifacts,
- complete broad macOS/Linux heavy-model verification,
- resolve all P0/P1 findings from the release review.
