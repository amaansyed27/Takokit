# Local API

Takokit serves a local HTTP API on `127.0.0.1:5050` by default.

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

`POST /v1/models/pull` installs local mock metadata only. It writes an installed-model record and a manifest copy; it does not download model weights.

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

Using package models such as `kokoro` for speech runs lifecycle resolution. If the model is not pulled, Takokit returns `model_not_installed`. If the model is pulled but the required runner is missing, Takokit returns `runner_not_installed`. If both metadata records exist, Takokit returns `inference_not_implemented` until real runners exist.

`POST /v1/audio/transcriptions` uses the same runner resolution layer for STT. For example, `kokoro` returns `capability_unsupported` for STT, while `whisper-base` returns `runner_not_installed` until `takokit-whispercpp` exists.

Typed errors use this shape:

```json
{
  "error": {
    "code": "runner_not_installed",
    "message": "whisper-base supports STT, but runner takokit-whispercpp is not installed or not implemented yet."
  }
}
```

## Package Pull

```json
{
  "model": "kokoro"
}
```

The current pull flow installs metadata from the local mock registry. It does not download weights.

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
