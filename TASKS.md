# Takokit Task Tracker

## Current Focus

- Make Takokit behave like a real Ollama-style local voice runtime: install once, `takokit pull <model>`, then run/speak/transcribe without manual Python, PyTorch, FFmpeg, repo cloning, or model-specific setup.
- Keep development incremental and testable. No phase-gated roadmap here; this file tracks the next small pieces to build and verify.

## Next Small Tasks

- [ ] Implement Piper ONNX session loading and audio generation.
- [ ] Add Piper text normalization/tokenization planning without vendoring GPL runtime code.
- [ ] Wire curated library metadata into the GUI models browser.
- [ ] Add API routes for curated library metadata.
- [ ] Add release packaging.
- [ ] Add actual install script release URLs after artifacts and checksums exist.
- [ ] Add public model library website.

## Done

- [x] Added curated model and runner library metadata under `registry/library/`.
- [x] Added typed library manifest parsing for discovery metadata.
- [x] Added `takokit library models` and `takokit library runners`.
- [x] Added `tako` as a short command alias for `takokit` while preserving the `~/.takokit` storage root.
- [x] Added Piper ONNX runner artifact loading scaffold for installed Lessac model/config artifacts.
- [x] Added typed Piper JSON config parsing in the ONNX runner layer.
- [x] Added checksum-backed artifact install foundation with temporary downloads and content-addressed blobs.
- [x] Made artifact-backed model installs atomic so failed artifact verification leaves no final model manifest or installed-model record.
- [x] Added typed artifact errors for missing URLs, missing checksums, failed downloads, checksum mismatches, and install failures.
- [x] Added model/config artifact roles to manifests and installed records.
- [x] Finalized verified Piper Lessac SHA256 values and enabled artifact-backed `pull piper-lessac`.
- [x] Updated `piper-lessac` with the Piper Lessac medium ONNX model/config artifact shape.
- [x] Added artifact lifecycle docs and Piper source/licensing references.
- [x] Split execution planning from runner execution.
- [x] Added a runner execution interface for speech and transcription.
- [x] Added an ONNX runner scaffold that returns typed `inference_not_implemented`.
- [x] Chose Piper ONNX as the first real ONNX target in `docs/decisions/0001-first-onnx-model.md`.
- [x] Added bare `takokit` interactive terminal launcher.
- [x] Added `takokit doctor` for storage, registry, installed record, server, GUI, mock execution, and platform checks.
- [x] Added safe future installer scaffolds in `scripts/install.sh` and `scripts/install.ps1`.
- [x] Added `takokit models` and `takokit runners` aliases for model and runner listings.
- [x] Added typed installed model records under `manifests/installed-models`.
- [x] Added typed installed runner records under `manifests/installed-runners`.
- [x] Kept legacy installed manifest copies under `manifests/models` and `manifests/runners`.
- [x] Added runner lifecycle API routes for show, pull, and remove.
- [x] Added `takokit runner pull/show/rm`.
- [x] Updated runner resolution to check model install state before runner install state.
- [x] Updated `takokit pull <model>` to write fuller installed model metadata.
- [x] Wired GUI model show, pull, remove, and pull-required-runner actions through the local API.
- [x] Added spec alignment tracker in `docs/spec-alignment.md`.
- [x] Added five first-class Takokit product surfaces: TTS, STT, Voice Cloning, Live Transcription Local API, and Live Audio API.
- [x] Added typed capability declarations to model manifests.
- [x] Updated mock registry manifests for Kokoro, Whisper Base, Piper Lessac, Chatterbox, and GPT-SoVITS.
- [x] Added runner resolution: model manifest -> capability check -> required runner -> platform/install check -> typed plan/error.
- [x] Added `takokit capabilities`.
- [x] Routed `takokit speak` and `takokit transcribe` through runner resolution while preserving the `mock-tts` WAV path.
- [x] Updated `takokit show <model>` to print capabilities, hardware notes, runner status, installed status, and honest execution status.
- [x] Added `GET /v1/capabilities`.
- [x] Added API model capability and runner status fields.
- [x] Routed speech and transcription API endpoints through runner resolution with typed JSON errors.
- [x] Updated the GUI to show the five product surfaces and model capability badges.
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

- Artifact hosting source beyond the initial Hugging Face Piper voice source: GitHub Releases, Takokit registry service, or static CDN.
- Signature policy after checksum-backed downloads.
- Whether managed Python runners are allowed in v0.1, or whether v0.1 should stay native/ONNX/whisper.cpp only.
- Public library website structure and domain deployment target.

## Notes

- `takokit pull kokoro` installs only the local mock manifest for now.
- `takokit pull piper-lessac` downloads and verifies the Piper Lessac medium ONNX model/config artifacts.
- `takokit speak "Hello" --model kokoro` must not claim real Kokoro inference until a runner exists.
- `takokit speak "Hello" --model mock-tts` is the only current speech path and is for CLI/API contract testing.
- The GUI is a local browser GUI, not a Tauri app.
- Keep every new task small enough to test immediately with `cargo check`, CLI commands, or GUI build.
