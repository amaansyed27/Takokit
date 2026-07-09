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
POST   /v1/models/pull
DELETE /v1/models/:id
GET    /v1/runners
GET    /v1/runners/:id
POST   /v1/runners/pull
DELETE /v1/runners/:id
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

`GET /v1/models` and `GET /v1/models/:id` include supported capabilities, backend, runner, installed status, runner installed status, hardware notes, artifact count, and honest execution status.

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

`POST /v1/runners/pull` installs a local runner contract record:

```json
{
  "runner": "takokit-onnx"
}
```

This does not download or install an execution binary. `DELETE /v1/runners/:id` removes the local installed runner metadata.

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

`POST /v1/audio/transcriptions` uses the same planning and execution split for STT. Planning checks model install state before capability support, so an unpulled model returns `model_not_installed`. After `kokoro` metadata is pulled, transcription with `kokoro` returns `capability_unsupported` for STT. `whisper-base` returns `runner_not_installed` until `takokit-whispercpp` exists, then the executor returns `inference_not_implemented`.

Typed errors use this shape:

```json
{
  "error": {
    "code": "runner_not_installed",
    "message": "whisper-base supports STT, but runner takokit-whispercpp is not installed or not implemented yet."
  }
}
```

After a model and its runner contract are installed, ONNX execution currently returns:

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

Other artifact error codes are `artifact_url_missing`, `artifact_download_failed`, `artifact_checksum_mismatch`, and `artifact_install_failed`.

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
