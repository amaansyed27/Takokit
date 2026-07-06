# Local API

Takokit serves a local HTTP API on `127.0.0.1:5050` by default.

## Routes

```http
GET  /health
GET  /v1/status
GET  /v1/models
GET  /v1/voices
POST /v1/audio/speech
POST /v1/audio/transcriptions
POST /v1/voices/clone
POST /v1/voices/train
```

Implemented now:

- `GET /health`
- `GET /v1/status`
- `GET /v1/models`
- `GET /v1/voices`
- `POST /v1/audio/speech`

The speech route accepts an OpenAI-compatible shape where practical:

```json
{
  "model": "mock-tts",
  "input": "Hello from Takokit",
  "voice": "default",
  "response_format": "wav"
}
```

Current response:

```json
{
  "id": "uuid",
  "model": "mock-tts",
  "voice": "default",
  "engine": "mock-tts",
  "output_path": "~/.takokit/outputs/speech-uuid.wav",
  "content_type": "audio/wav",
  "bytes": 19244
}
```

Transcription, clone, and train routes return structured not-implemented errors until their adapters are wired.

