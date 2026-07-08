# Local API

Takokit serves a local HTTP API on `127.0.0.1:5050` by default.

## Routes

```http
GET    /health
GET    /v1/status
GET    /v1/capabilities
GET    /v1/models
GET    /v1/models/:id
GET    /v1/runners
POST   /v1/models/pull
DELETE /v1/models/:id
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

`GET /v1/models` and `GET /v1/models/:id` include supported capabilities, backend, runner, installed status, runner installed status, hardware notes, and honest execution status.

`POST /v1/audio/speech` only supports `mock-tts` right now:

```json
{
  "model": "mock-tts",
  "input": "Hello from Takokit",
  "voice": "default",
  "response_format": "wav"
}
```

Using package models such as `kokoro` for speech returns a typed not-implemented error until real runners exist.

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

The current pull flow installs a manifest from the local mock registry. It does not download weights.

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
