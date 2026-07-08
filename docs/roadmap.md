# Roadmap

This file tracks near-term direction without phase gates. The source of truth for current work is [../TASKS.md](../TASKS.md).

## Next Useful Increments

- Add installed runner registry support.
- Implement the first real ONNX runner.
- Choose the first real model target: Kokoro ONNX or Piper ONNX.
- Add checksum-backed artifact download before any real model downloads.
- Make package pull write a fuller installed model record, including artifact slots and checksum placeholders.
- Add config loading from `~/.takokit/config.toml`.
- Add browser GUI controls for pulling/removing manifests through the API.
- Add API tests for model detail, runner listing, pull, and delete.
- Add broader API tests for capability and runner resolution errors.

## Product Surface Contract

Takokit's first-class surfaces are TTS, STT, Voice Cloning, Live Transcription Local API, and Live Audio API. New model work should start by declaring which of those surfaces a manifest supports, then resolving the required runner before any execution code is added.

## Keep Out For Now

- Tauri app scaffolding.
- Fake Kokoro or Whisper inference.
- Hidden cloud calls.
- Model-specific dependency instructions as the primary user path.
