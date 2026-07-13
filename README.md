# Takokit

Takokit is a Rust-first local voice AI runtime: an Ollama-like pull, run, inspect and remove experience for text-to-speech, speech-to-text and consent-backed voice profiles.

It ships one shared backend with three interfaces:

- `tako` / `takokit` command-line interface,
- a task-oriented Ratatui terminal interface when started without a subcommand,
- a local browser GUI opened with `tako gui`.

The local Axum daemon binds to `127.0.0.1:5050`. CLI, TUI, GUI and API use the same registry, installed records, execution planner, runners, sessions and output locations.

## Current release status

Takokit is preparing for a `v0.1.0` public beta. The codebase contains a **27-model catalog** across TTS, STT, cloning, conversion and training families, but it deliberately distinguishes:

- **locally verified** models,
- **executable paths awaiting hardware smoke tests**,
- **planned or blocked** models.

Compilation or a registry manifest alone is not treated as model support. See [docs/model-support.md](docs/model-support.md) for the complete model and runner matrix.

The website and public model library are intentionally postponed until the stability guide and Codex review are complete.

## Storage model

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

User-facing outputs and history belong to the directory that launched Takokit:

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

Starting CLI inference, the TUI or `tako gui` from a project creates or resumes that project's `.tako` workspace. Use `--workspace <path>` and `--session <uuid>` for explicit context.

## Quick start from source

Prerequisites:

- Rust stable,
- Node.js LTS and npm for the GUI build,
- sufficient disk space for selected models.

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

A running Windows daemon can lock the release executable. Stop it before rebuilding:

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

tako models
tako runners
tako show kokoro
tako plan whisper-tiny
tako pull kokoro
tako rm kokoro

tako speak "Hello from Takokit" --model kokoro
tako transcribe ./sample.wav --model whisper-tiny
tako run kokoro "Hello from the unified run command"
tako run whisper-tiny --file ./sample.wav

tako clone ./reference.wav --name "My Voice" --model chatterbox --consent
tako list voices

tako sessions list
tako sessions list --query interview
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

## Ollama-style pull lifecycle

`tako pull <model>` owns the model setup. Users should not manually clone upstream repositories, launch Gradio applications or install random Python requirements globally.

A pull resolves and prepares:

1. model manifest and capabilities,
2. required runner contract,
3. required runner runtime,
4. isolated adapter environment when needed,
5. pinned artifacts or official upstream weight resolution,
6. readiness state and actionable failure details.

Examples:

```bash
tako pull whisper-tiny
tako pull kokoro
tako pull qwen3-tts
tako pull chatterbox
tako pull kyutai-tts-1.6b
```

Repeat pulls are designed to be idempotent. Checksum-backed artifacts use content-addressed storage and verified local reuse. Heavy managed-Python models keep independent adapter environments under:

```text
~/.takokit/runners/python-managed/adapters/<adapter>/venv
```

One model family cannot silently mutate another family's environment.

## Models and runners

The catalog currently spans:

- native/managed ONNX TTS,
- whisper.cpp STT,
- managed Qwen, Chatterbox, F5-TTS and Coqui TTS/cloning,
- Transformers audio generation and transcription,
- FunASR SenseVoice,
- NeMo Canary and Parakeet,
- Voxtral audio-language transcription,
- Kyutai DSM TTS,
- planned OpenVoice, GPT-SoVITS and RVC conversion/training workflows.

Primary runner contracts:

- `takokit-onnx`
- `takokit-whispercpp`
- `takokit-python-managed`
- `takokit-transformers-audio`
- `takokit-nemo`

See [docs/runners.md](docs/runners.md) and [docs/model-support.md](docs/model-support.md).

## Voice profiles and consent

Voice-profile creation requires explicit confirmation that the user owns the voice or has permission to use it.

```bash
tako clone ./reference.wav --name "My Voice" --model chatterbox --consent
tako speak "Profile reuse test" --model chatterbox --voice my-voice
```

Takokit copies the reference into global local-only voice storage and records the profile-creation event in the active project session. CLI, TUI, GUI and API enforce the consent boundary.

Kyutai TTS uses official precomputed voice embeddings. Takokit does not claim arbitrary reference-audio cloning for Kyutai.

## Terminal interface

Run Takokit without a subcommand:

```bash
tako
```

The TUI is task-oriented, not a raw CLI command editor. It provides:

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
Enter               Run the visible primary action
/sessions           Open saved sessions
/new                Create a session
/clone              Open voice profile creation
F1                   Help
Ctrl+C               Exit safely
```

## Local GUI

Build the GUI and open it through the daemon:

```bash
cd apps/gui
npm ci
npm run build
cd ../..
tako gui
```

The GUI includes model and runner state, Speak, Transcribe, consent-backed Voices, searchable History, diagnostics and settings. Its workspace and session are carried in the launch context, and its API requests write to the same active `.tako` session as CLI and TUI.

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

Workspace-aware requests use Takokit workspace and session headers. Output-serving routes validate single filenames and remain restricted to the selected session output directory.

## Architecture

```text
CLI ─────┐
TUI ─────┼── shared application/runtime services ── model runners
GUI/API ─┘                 │
                           ├── global ~/.takokit resources
                           └── project .tako sessions and outputs
```

Major workspace crates:

- `takokit-core` — shared types, capabilities, sessions and typed errors,
- `takokit-package` — registry, pull/install, planning and runner lifecycle,
- `takokit-models` — execution adapters,
- `takokit-server` — daemon and local API,
- `takokit-store` — global resources, voice profiles and project sessions,
- `takokit-audio` — audio utilities,
- `apps/cli` — CLI and Ratatui interface,
- `apps/gui` — React/Vite local GUI.

## Stability gate

Before website or public release work:

1. Follow [docs/TESTING.md](docs/TESTING.md) on the Windows RTX 5060 test machine.
2. Record real-device evidence for each model promoted to verified.
3. Run the prompt in [docs/CODEX_REVIEW_PROMPT.md](docs/CODEX_REVIEW_PROMPT.md).
4. Resolve all P0/P1 findings.
5. Keep untested heavy models labelled executable-path or planned.

Automated gates:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
python scripts/check_file_sizes.py
cd apps/gui && npm ci && npm run build
```

The repository is ready for website work only after these automated gates and the documented hardware/core-flow smoke tests pass.

## Development status boundaries

The following remain honest roadmap items until their complete workflows pass testing:

- GPT-SoVITS dataset preparation and training jobs,
- RVC training and conversion,
- OpenVoice tone-colour conversion,
- Piper's safe text/phoneme frontend,
- production installers and signed release artifacts,
- broad macOS/Linux heavy-model hardware verification.

Takokit returns typed, actionable blockers for these paths rather than fake audio or success responses.
