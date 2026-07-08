# Takokit Task Tracker

## Current Focus

- Stabilize the Ollama-style local architecture: Rust CLI, Rust daemon/API, local browser GUI, and manifest-backed package manager foundation.

## Next Small Tasks

- Add API tests for `/v1/models/:id`, `/v1/runners`, pull, and delete.
- Add installed runner registry behavior.
- Add GUI pull/remove buttons that call the API.
- Add config-file loading instead of always using `RuntimeConfig::local`.

## Done

- Renamed `apps/desktop` to `apps/gui`.
- Removed the Tauri-ready desktop direction from docs and app wording.
- Added `takokit gui`.
- Added static GUI serving at `/gui`.
- Added typed model and runner manifests in `takokit-package`.
- Added local mock registry manifests for Kokoro, Whisper Base, Piper Lessac, and runner contracts.
- Added manifest-backed `pull`, `show`, `list models`, `list runners`, and `rm`.
- Expanded local storage layout for models, runners, blobs, manifests, voices, datasets, outputs, cache, logs, and config.
- Kept real inference explicitly unimplemented outside `mock-tts`.

## Blocked / Needs Decision

- Real runner implementation strategy is still open.
- Artifact download source and checksum policy need a concrete design before network downloads.

## Notes

- `takokit pull kokoro` installs only the local mock manifest for now.
- `takokit speak "Hello" --model kokoro` should not claim real Kokoro inference until a runner exists.
