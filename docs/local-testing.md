# Local Testing

Takokit should be tested as one local product shell: CLI, `tako` alias, storage, API, GUI, runners, and honest model plans.

## Fast path

```powershell
target\release\takokit.exe quickstart
target\release\takokit.exe samples create
target\release\takokit.exe test --suite fast --run
```

`quickstart` prepares only Kokoro and Whisper Tiny by default. `takokit deps bootstrap` copies the pinned `uv 0.11.24` binary into `~/.takokit/tools/uv/`, verifies `uv --version`, and records its source, SHA-256, version, path, and output at `~/.takokit/logs/uv-bootstrap.log`. Runner installation uses that managed path, never PATH. `UV` remains a development bootstrap override only. The fast suite never downloads model artifacts.

`samples create` writes `hello.wav` and `silence.wav` under `~/.takokit/samples/`. `hello.wav` is real Kokoro speech and the command fails if Kokoro cannot create it. `silence.wav` remains a separate deliberate fixture.

`test --suite fast --run --json` emits one JSON row per tested model, including result status, output path, transcript, duration, error, and log path. A `failed` executable model gives a non-zero process exit; lifecycle-blocked models are reported as `skipped`.

## Isolated Storage

Use `TAKOKIT_HOME` when a test should not touch the normal `~/.takokit` root.

```powershell
$env:TAKOKIT_HOME="$env:TEMP\takokit-smoke"
```

Unset it to return to the default root.

## Build

```bash
cargo build --release
cd apps/gui
npm install
npm run build
cd ../..
```

## CLI Smoke

```bash
target/release/takokit.exe version
target/release/takokit.exe doctor
target/release/takokit.exe doctor --json
target/release/takokit.exe models
target/release/takokit.exe runners
target/release/takokit.exe plan whisper-base
target/release/takokit.exe plan whisper-base --json
target/release/tako.exe doctor
```

`takokit` and `tako` must report the same storage root.

## Whisper STT

The normal local workflow is `takokit pull whisper-tiny` followed by `takokit run whisper-tiny --file sample.wav`. A normal pull orchestrates required managed runner components and verified artifacts; `takokit pull --metadata-only` deliberately does neither.

Normal `cargo test --workspace` uses local deterministic fixtures and must not download model artifacts, bootstrap Python, invoke curl, or access external network resources. Real-model smoke coverage is opt-in only: `TAKOKIT_REAL_MODEL_TESTS=1 cargo test --test real_models -- --ignored`.

Whisper Tiny, Base, and Small are catalogued as whisper.cpp-backed models. Tiny/Base have the current runner path; Small stays metadata/artifact status until a verified execution run exists. Normal `run`, `speak`, `transcribe`, package, runner, and adapter commands use the managed daemon. Prefix a development command with `--direct` to retain in-process behavior. `takokit daemon start`, `takokit list`, and `takokit ps` exercise lifecycle without downloading artifacts.

```bash
target/release/takokit.exe runner pull takokit-whispercpp
target/release/takokit.exe runner install takokit-whispercpp
target/release/takokit.exe pull whisper-base
target/release/takokit.exe transcribe ./sample.wav --model whisper-base
```

This is a real path on Windows x64. The runner installer verifies the whisper.cpp ZIP checksum and marks the runner `ready` only when `whisper-cli.exe` is present. If `sample.wav` is missing or unreadable, the command should fail with a concrete file-path error.

## Piper TTS

```bash
target/release/takokit.exe runner pull takokit-onnx
target/release/takokit.exe runner install takokit-onnx
target/release/takokit.exe pull piper-lessac
target/release/takokit.exe plan piper-lessac
target/release/takokit.exe speak "Hello" --model piper-lessac
```

Piper has verified model/config artifacts, but real TTS is still blocked at the text frontend. The final command must return typed `piper_text_frontend_not_implemented`, not a fake WAV.

## Kokoro TTS

```bash
target/release/takokit.exe runner pull takokit-onnx
target/release/takokit.exe runner install takokit-onnx
target/release/takokit.exe pull kokoro
target/release/takokit.exe speak "Hello from Takokit" --model kokoro
```

The result is JSON containing the real WAV path, byte count, content type, engine, and sample rate.

## Python-Managed

```bash
target/release/takokit.exe runner pull takokit-python-managed
target/release/takokit.exe runner install takokit-python-managed
target/release/takokit.exe runner doctor takokit-python-managed
target/release/takokit.exe plan qwen3-tts
target/release/takokit.exe adapter install qwen3-tts
target/release/takokit.exe pull qwen3-tts
target/release/takokit.exe speak "Hello from Takokit" --model qwen3-tts --voice Ryan
```

Takokit owns the Python environment, package install, logs, pinned artifact pull, and local output. It does not expose Qwen reference-audio cloning without a future consent gate.

## API

```bash
target/release/takokit.exe serve
curl http://127.0.0.1:5050/v1/models
curl http://127.0.0.1:5050/v1/models/whisper-base/plan
curl http://127.0.0.1:5050/v1/doctor
curl http://127.0.0.1:5050/v1/adapters
```

Adapter install uses `POST /v1/adapters/install` with `{ "adapter": "qwen3_tts" }`.

## GUI

```bash
target/release/takokit.exe gui
```

The GUI opens `/gui` and shows model lifecycle state, executable blockers, runner diagnostics, and Python adapter state. When Python-managed is ready, the runner detail exposes the real Qwen adapter install action.

## Launch Matrix

```bash
target/release/takokit.exe test --suite launch
target/release/takokit.exe test --suite launch --json
```

The launch suite is non-destructive. It reports manifest/planning status and only runs real smoke execution where a model is executable. Still blocked: Piper Lessac, OpenVoice, RVC, NeMo/Parakeet/Canary, and transformers-audio/Qwen-Omni/Voxtral.
