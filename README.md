# Takokit

Takokit is a Rust-first local voice AI runtime: an Ollama-like pull, run, inspect, update, and remove experience for text-to-speech, speech-to-text, consent-backed voice cloning, voice conversion, and custom voice workflows.

> **Release maturity:** every Takokit `0.x.x` release is **Beta**. `1.0.0` is the first Stable release target.

Takokit exposes one shared runtime through:

- `tako` — CLI and Ratatui TUI,
- `tako gui` — local browser GUI,
- `tako serve` / `tako server ...` — local server lifecycle,
- an OpenAI-compatible **audio** API at `/v1`,
- Takokit-native APIs at `/api/v1`.

The interfaces share the same registry, model installs, runners, voice profiles, workspaces, sessions, and output history.

## Install

### Windows x86_64

```powershell
irm https://takokit.dawnlightlabs.com/install.ps1 | iex
```

The Windows release also includes a normal Inno Setup installer and portable ZIP.

### Linux x86_64

```bash
curl -fsSL https://takokit.dawnlightlabs.com/install.sh | sh
```

The packaged Linux runtime installs per-user, includes the desktop launcher, and does not require Cargo, Node, Homebrew, or a system Python environment.

### macOS Apple Silicon

```bash
curl -fsSL https://takokit.dawnlightlabs.com/install.sh | sh
```

The macOS package installs per-user and includes `Takokit.app` with the menu-bar resident controller. Intel macOS remains experimental for the v0.3.0 beta; Linux ARM64 is not published in v0.3.0.

See the [platform support matrix](docs/platform-support.md) and the [public documentation](https://takokit.dawnlightlabs.com/docs).

## Quickstart

```bash
tako version
tako doctor

tako pull kokoro
tako speak "Hello from Takokit" --model kokoro

tako pull whisper-tiny
tako transcribe ./recording.wav --model whisper-tiny

tako gui
```

Run `tako` without a subcommand for the TUI.

## Voice workflows

```bash
# Reference voice profile / cloning
tako clone ./reference.wav --name "My Voice" --model chatterbox --consent

# Voice conversion
tako convert ./source.wav --target-voice ./owned-target.wav --model openvoice --consent

# RVC conversion
tako convert ./source.wav --target-voice ./owned-voice.pth --model rvc --consent

# Advanced local RVC project
tako voice rvc create --name "My RVC Voice" --consent
tako voice rvc samples my-rvc-voice add ./samples/*.wav
tako voice rvc preflight my-rvc-voice --preset balanced
tako voice rvc train my-rvc-voice --preset balanced
```

Voice cloning, conversion, and training require ownership or explicit permission for the relevant voice/reference/training material.

## Models and runners

Browse the bundled registry:

```bash
tako models
tako show kokoro
tako plan qwen3-tts:0.6b-base
tako runners
tako library models
```

The registry contains versioned model families and immutable releases. A model being listed does not mean every hardware/backend combination is verified. Takokit keeps support labels evidence-based and reports hardware/upstream limitations rather than treating packaging success as model parity.

Primary runner contracts include:

- `takokit-onnx`
- `takokit-whispercpp`
- `takokit-python-managed`
- `takokit-transformers-audio`
- `takokit-nemo`

See [model support](docs/model-support.md), [runner architecture](docs/runners.md), and [model smoke testing](docs/MODEL_SMOKE_TESTS.md).

## Storage and workspaces

Reusable runtime state is global:

```text
~/.takokit/
```

Project history and outputs are local to the active workspace:

```text
<workspace>/.tako/
```

Use `--workspace <path>` and `--session <uuid>` for explicit project context. Normal application uninstall preserves both global user state and project `.tako` state unless you separately request destructive cleanup.

See [state and storage](docs/state-and-storage.md).

## Server and local API

Default server:

```text
http://127.0.0.1:5050
```

Common lifecycle commands:

```bash
tako start
tako server status
tako stop

# Explicit foreground/developer-owned server
tako serve
```

API surfaces:

```text
GET  /health
GET  /openapi.json

/v1
  OpenAI-compatible audio subset:
  models, audio/speech, audio/transcriptions

/api/v1
  Takokit-native model/runtime/voice/RVC/workspace/session/storage/update APIs
```

Takokit does **not** claim OpenAI Chat, Responses, embeddings, images, or general OpenAI API compatibility. See [API documentation](docs/api.md) and the live `/openapi.json` contract.

## Updates

```bash
tako update status
tako update check
tako update channel stable
tako update download
tako update apply
```

Takokit verifies signed release metadata and artifact checksums before replacement. Installation remains explicit; update replacement preserves mutable user and project state.

## Development

Prerequisites: Rust stable, Node.js 20+ with npm, and platform build prerequisites for the feature you are modifying.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
python scripts/audit_file_sizes.py

cd apps/gui
npm ci
npm run build
cd ../..

cd site
npm ci
npm run check
```

Read [CONTRIBUTING.md](CONTRIBUTING.md), the [architecture guide](docs/architecture.md), [testing guide](docs/TESTING.md), and [release process](docs/release-process.md) before changing runtime or distribution contracts.

## Documentation

- [Public docs](https://takokit.dawnlightlabs.com/docs)
- [Documentation index](docs/README.md)
- [Architecture](docs/architecture.md)
- [CLI and interfaces](https://takokit.dawnlightlabs.com/docs/cli-reference)
- [API](docs/api.md)
- [Registry](docs/registry.md)
- [Runner development](docs/runners.md)
- [Contributor guide](docs/contributor-guide.md)
- [Release process](docs/release-process.md)
- [Security](SECURITY.md)

## License

See [LICENSE](LICENSE) and [third-party/model license notes](docs/licenses.md).
