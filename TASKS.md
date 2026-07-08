# Takokit Task Tracker

## Current Focus

- Make Takokit behave like a real Ollama-style local voice runtime: install once, `takokit pull <model>`, then run/speak/transcribe without manual Python, PyTorch, FFmpeg, repo cloning, or model-specific setup.
- Keep development incremental and testable. No phase-gated roadmap here; this file tracks the next small pieces to build and verify.

## Next Small Tasks

- [ ] Add installed runner registry behavior so runners can be tracked separately from models.
- [ ] Make `takokit pull <model>` write a fuller installed-model record, including artifact slots, checksum placeholders, source metadata, install time, and runner reference.
- [ ] Add a runner resolution layer: model manifest -> required runner -> installed runner check -> typed error if runner is missing/unsupported.
- [ ] Add API tests for `GET /v1/models/:id`, `GET /v1/runners`, `POST /v1/models/pull`, and `DELETE /v1/models/:id`.
- [ ] Add GUI model actions for pull/remove/show using the local API, with honest mock/no-real-inference states.
- [ ] Add config loading from `~/.takokit/config.toml` instead of always using `RuntimeConfig::local`.
- [ ] Add a `takokit doctor` command to check storage layout, GUI build availability, server status, registry health, and runner availability.
- [ ] Add `takokit list installed` or improve `takokit list models` so available vs installed state is obvious in CLI output.
- [ ] Decide the first real no-dependency runner target: Kokoro ONNX or Piper ONNX.
- [ ] Define artifact download policy before network downloads: source URL, checksum, retries, partial download cleanup, and no hidden cloud calls.

## Done

- [x] Renamed `apps/desktop` to `apps/gui`.
- [x] Removed the Tauri-ready desktop direction from README/docs and changed product direction to Rust CLI + daemon/API + local browser GUI.
- [x] Added `takokit gui`.
- [x] `takokit gui` starts the server process if unavailable, waits for it, then opens the local GUI URL or prints it.
- [x] Added static GUI serving at `/gui`.
- [x] Added typed model and runner manifests in `takokit-package`.
- [x] Added local mock registry manifests for Kokoro, Whisper Base, Piper Lessac, and runner contracts.
- [x] Added manifest-backed `pull`, `show`, `list models`, `list runners`, and `rm` command flow.
- [x] Expanded local storage layout for models, runners, blobs, manifests, voices, datasets, outputs, cache, logs, and config.
- [x] Kept real inference explicitly unimplemented outside `mock-tts`.
- [x] Preserved modular Rust workspace structure with separated CLI, server, core, model, package, audio, store, and safety crates.
- [x] Preserved local web GUI as React + TypeScript + Vite under `apps/gui`.

## Blocked / Needs Decision

- First real runner target: Kokoro ONNX vs Piper ONNX.
- Artifact hosting source: GitHub Releases, Hugging Face, Takokit registry service, or static CDN.
- Checksum/signature policy before real downloads.
- Whether managed Python runners are allowed in v0.1, or whether v0.1 should stay native/ONNX/whisper.cpp only.
- Public library website structure and domain deployment target.

## Notes

- `takokit pull kokoro` installs only the local mock manifest for now.
- `takokit speak "Hello" --model kokoro` must not claim real Kokoro inference until a runner exists.
- `takokit speak "Hello" --model mock-tts` is the only current speech path and is for CLI/API contract testing.
- The GUI is a local browser GUI, not a Tauri app.
- Keep every new task small enough to test immediately with `cargo check`, CLI commands, or GUI build.
