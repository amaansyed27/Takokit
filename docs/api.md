# Local API

Takokit serves a local HTTP API on `127.0.0.1:5050` by default.

The CLI also includes local inspection commands:

```bash
takokit
takokit doctor
```

These are CLI features rather than HTTP routes. `takokit` opens the interactive terminal launcher, and `takokit doctor` checks local storage, registry manifests, installed records, server availability, GUI build output, mock TTS availability, and platform state.

## Routes

```http
GET    /health
GET    /v1/status
GET    /v1/capabilities
GET    /v1/models
GET    /v1/models/:id
GET    /v1/models/:id/plan
POST   /v1/models/pull
DELETE /v1/models/:id
GET    /v1/runners
GET    /v1/runners/:id
GET    /v1/runners/:id/doctor
POST   /v1/runners/pull
POST   /v1/runners/install
DELETE /v1/runners/:id
GET    /v1/library/models
GET    /v1/library/runners
GET    /v1/doctor
GET    /v1/test/launch
GET    /v1/voices
POST   /v1/audio/speech
POST   /v1/audio/transcriptions
POST   /v1/voices/clone
POST   /v1/voices/train
```

## Capabilities

`GET /v1/capabilities` returns the five Takokit product surfaces:

- TTS
- STT
- Voice Cloning
- Live Transcription Local API
- Live Audio API

These are represented in API JSON using typed capability IDs such as `text_to_speech`, `speech_to_text`, `voice_cloning`, `live_transcription`, and `live_audio`.

## Models

`GET /v1/models` and `GET /v1/models/:id` include supported capabilities, backend, runner, installed status, runner installed status, hardware notes, artifact count, license warning, runner runtime state, lifecycle state, executable flag, missing pieces, next command, and honest execution status. These fields are derived from the same planner used by `takokit plan <model>`.

Important summary fields:

```json
{
  "id": "whisper-base",
  "family": "whisper",
  "runner": "takokit-whispercpp",
  "artifact_count": 1,
  "installed": true,
  "runner_installed": true,
  "runner_runtime_state": "ready",
  "lifecycle_state": "executable",
  "executable": true,
  "missing": [],
  "next_command": "takokit transcribe <audio.wav> --model whisper-base"
}
```

`GET /v1/models/:id/plan` returns the structured lifecycle plan for a model: required runner, model lifecycle state, artifact state, runner contract state, runner runtime state, executable today, missing pieces, and next command.

`POST /v1/models/pull` installs model metadata and, when a manifest has verified artifact URLs/checksums, installs those artifacts into content-addressed blobs.

```json
{
  "model": "piper-lessac",
  "metadata_only": false
}
```

`metadata_only` is optional and defaults to `false`. A manifest can also declare itself metadata-only while artifact checksums are still unresolved. `piper-lessac` now has verified Piper Lessac medium ONNX model/config URLs, byte sizes, and SHA256 values, so the default pull downloads and verifies both artifacts.

`DELETE /v1/models/:id` removes the local installed model metadata.

## Runners

`GET /v1/runners` and `GET /v1/runners/:id` include runner contract metadata and installed status.

`GET /v1/runners/:id/doctor` returns contract/runtime state, note, runtime path, logs path, and Python-managed adapter slots when applicable.

`POST /v1/runners/pull` installs a local runner contract record:

```json
{
  "runner": "takokit-onnx"
}
```

This does not download or install an execution binary. `DELETE /v1/runners/:id` removes the local installed runner metadata.

`POST /v1/runners/install` performs explicit runner runtime setup:

```json
{
  "runner": "takokit-whispercpp"
}
```

On Windows x64, `takokit-whispercpp` installs and verifies the whisper.cpp release binary and marks the runner `ready` when `whisper-cli.exe` is present. `takokit-python-managed` initializes the managed directory layout and adapter slots but does not install Python/Torch yet.

`GET /v1/library/models` and `GET /v1/library/runners` return curated discovery metadata. These routes do not imply runtime executability and do not trigger downloads.

`GET /v1/doctor` returns storage, registry, installed-record, GUI, and runner checks:

```json
{
  "data": {
    "storage_root": "C:\\Users\\Amaan\\.takokit",
    "server": "127.0.0.1:5050",
    "checks": [
      {
        "section": "runner",
        "label": "takokit-whispercpp ready",
        "status": "ok",
        "detail": "whisper.cpp runtime installed..."
      }
    ],
    "executable_models": ["mock-tts", "whisper-base"],
    "logs_path": "C:\\Users\\Amaan\\.takokit\\logs"
  }
}
```

`GET /v1/test/launch` returns the same non-destructive launch matrix style data as `takokit test --suite launch`.

`POST /v1/audio/speech` only supports `mock-tts` right now:

```json
{
  "model": "mock-tts",
  "input": "Hello from Takokit",
  "voice": "default",
  "response_format": "wav"
}
```

Using package models such as `kokoro` for speech runs lifecycle resolution and then runner execution. If the model is not pulled, Takokit returns `model_not_installed`. If the model is pulled but the required runner is missing, Takokit returns `runner_not_installed`. If both metadata records exist, Takokit builds an execution plan and passes it to the runner layer. The current ONNX runner scaffold then returns `inference_not_implemented` until real ONNX execution exists.

`POST /v1/audio/transcriptions` uses the same planning and execution split for STT. Planning checks model install state before capability support, so an unpulled model returns `model_not_installed`. After `kokoro` metadata is pulled, transcription with `kokoro` returns `capability_unsupported` for STT. `whisper-base` returns `runner_not_installed` until `takokit-whispercpp` exists. After the model artifact is pulled and the runner is installed/ready, it invokes whisper.cpp and returns real transcript text.

Typed errors use this shape:

```json
{
  "error": {
    "code": "runner_not_installed",
    "message": "whisper-base supports STT, but runner takokit-whispercpp is not installed or not implemented yet."
  }
}
```

After a model and its runner contract are installed, ONNX execution currently returns typed `inference_not_implemented` responses. ONNX returns:

```json
{
  "error": {
    "code": "inference_not_implemented",
    "message": "ONNX runner contract resolved, but real ONNX execution is not implemented yet."
  }
}
```

## Package Pull

```json
{
  "model": "kokoro"
}
```

Metadata-only models install records from the local mock registry without downloads.

Artifact-backed pulls require URL and SHA256 metadata. Expected artifact failures return typed JSON errors:

```json
{
  "error": {
    "code": "artifact_checksum_missing",
    "message": "artifact checksum missing for piper-lessac: en_US-lessac-medium.onnx"
  }
}
```

Other artifact error codes are `artifact_url_missing`, `artifact_download_failed`, `artifact_checksum_mismatch`, `artifact_install_failed`, `artifact_missing`, `artifact_not_downloaded`, and `artifact_config_invalid`. The last three can surface from runner preparation if an installed model record does not contain usable local artifact paths or a config cannot be parsed.

## GUI

The Rust server serves the built GUI at:

```txt
GET /gui
GET /gui/*
```

Build it with:

```bash
cd apps/gui
npm run build
```

If `takokit doctor` reports the GUI build as missing, that is a warning for packaged/local serving only. `npm run dev` remains valid for GUI development.
