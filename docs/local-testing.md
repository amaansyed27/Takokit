# Local Testing

Takokit should be tested as one local product shell: CLI, `tako` alias, storage, API, GUI, runners, and honest model plans.

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

Piper has verified model/config artifacts, but real TTS is still blocked at the text frontend. The final command must return typed `piper_text_frontend_not_implemented`, not a fake WAV. The next implementation step is a verified non-GPL path for text normalization, phonemization, and Piper phoneme ID preparation before ONNX session execution.

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
```

For the verified Qwen3-TTS path, continue with:

```bash
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
curl http://127.0.0.1:5050/v1/test/launch
```

Transcription uses a local file path:

```bash
curl -X POST http://127.0.0.1:5050/v1/audio/transcriptions \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"whisper-base\",\"file_path\":\"C:\\\\path\\\\to\\\\sample.wav\"}"
```

## GUI

```bash
target/release/takokit.exe gui
```

The GUI opens `/gui` and should show:

- Models with lifecycle state, executable yes/no, blockers, and next command.
- Runners with contract/runtime state and doctor actions.
- Library entries clearly separated from runtime models.
- Transcribe page that calls `/v1/audio/transcriptions`.
- Speak page that only enables executable TTS models.
- Diagnostics page backed by `/v1/doctor`.

## Launch Matrix

```bash
target/release/takokit.exe test --suite launch
target/release/takokit.exe test --suite launch --json
```

The launch suite is non-destructive. It reports manifest/planning status and only runs real smoke execution where a model is executable and the required local input is provided.
