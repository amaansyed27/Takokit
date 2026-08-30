# Slice 5 route inventory and stability classes

This inventory records the classification used for the 0.2.0 migration. The executable router and `/openapi.json` are the machine-readable contract; `scripts/check_api_contract.py` guards the conflicting routes and bundled clients.

## OpenAI-compatible public

- `GET /v1/models`
- `GET /v1/models/{model}`
- `POST /v1/audio/speech`
- `POST /v1/audio/transcriptions`

This is an audio-only subset. Chat Completions, Responses, embeddings, images, and general OpenAI API compatibility are not claimed.

## Takokit-native public

Stable Takokit schemas use `/api/v1`: status and diagnostics; model, runner, and adapter management; voices, cloning, conversion and Advanced RVC; workspace files; sessions and outputs; storage; and update settings. Native and OpenAI adapters share the same planner and execution runtime.

## GUI/internal privileged helpers

`system/picker`, `system/open`, local media, daemon shutdown, update apply, and destructive maintenance are local control helpers, not advertised stable third-party APIs. Host, Origin, loopback, and non-loopback Bearer policy is applied before them.

## Legacy compatibility

Common non-conflicting native `/v1` aliases remain during 0.2.0. The three conflicting paths always use OpenAI-compatible semantics. New native integrations must use `/api/v1`.
