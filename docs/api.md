# Local API

Takokit serves a local HTTP API on `127.0.0.1:5050` by default.

## Routes

```http
GET    /health
GET    /v1/status
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
