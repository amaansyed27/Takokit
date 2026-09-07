# Local API route inventory and stability classes

This inventory records the release-facing compatibility boundary for the Takokit beta line. The executable router and `/openapi.json` are the machine-readable contract; `scripts/check_api_contract.py` guards the public compatibility surface and bundled clients.

## OpenAI-compatible public audio subset

- `GET /v1/models`
- `GET /v1/models/{model}`
- `POST /v1/audio/speech`
- `POST /v1/audio/transcriptions`

This is an **audio-only subset**. Takokit does not claim Chat Completions, Responses, embeddings, images, or general OpenAI API compatibility.

## Takokit-native public API

Stable Takokit-specific schemas use `/api/v1` for:

- status and diagnostics,
- model, runner, and adapter management,
- voice profiles and cloning,
- voice conversion and Advanced RVC,
- workspace files/context,
- sessions and outputs,
- storage/maintenance state,
- update settings/status.

Native and OpenAI adapters share the same planner/execution runtime rather than implementing separate inference behavior.

## GUI/internal privileged helpers

Folder picker, open-path/media, daemon/server shutdown, update apply, and destructive maintenance endpoints are local product-control helpers, not advertised stable third-party APIs. Host, Origin, loopback, non-loopback Bearer authentication, and destructive-operation rules apply before those helpers.

Do not turn a narrowly scoped GUI helper into a general arbitrary command/filesystem endpoint.

## Legacy compatibility

Common non-conflicting historical native `/v1` aliases can remain during the beta line for backwards compatibility. The OpenAI-compatible paths above always use compatibility semantics. New Takokit-native integrations must use `/api/v1`.

## Documentation rule

Any release-facing route addition/removal/schema change must update the executable OpenAPI contract, API tests, this inventory, and public docs in the same change. Compatibility claims should never be expanded by documentation alone.
