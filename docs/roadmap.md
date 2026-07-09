# Roadmap

This file tracks near-term direction without phase gates. The source of truth for current work is [../TASKS.md](../TASKS.md).

## Next Useful Increments

- Implement Piper ONNX session loading and audio generation.
- Add Piper text normalization/tokenization planning without vendoring GPL runtime code.
- Add Whisper Tiny/Small manifests after verifying exact artifact SHA256 values.
- Design explicit managed Python installation flow before installing Python/Torch.
- Add release packaging and actual install script release URLs after artifacts and checksums exist.

## Current Product Shell

Takokit now supports the Ollama-like command shape for local development:

```bash
takokit
takokit doctor
takokit pull kokoro
takokit runner pull takokit-onnx
takokit runner install takokit-whispercpp
takokit plan kokoro
takokit speak "Hello" --model mock-tts
takokit transcribe ./audio.wav --model whisper-base
takokit gui
```

The bare command opens an interactive terminal launcher. The doctor command checks local setup health. Whisper Base execution works through whisper.cpp after model pull and runner install. Real Kokoro, Piper, Chatterbox, GPT-SoVITS, and Python-managed execution remains unimplemented.

## Product Surface Contract

Takokit's first-class surfaces are TTS, STT, Voice Cloning, Live Transcription Local API, and Live Audio API. New model work should start by declaring which of those surfaces a manifest supports, then resolving the required runner before any execution code is added.

## Planning And Execution Contract

Resolution now produces an `ExecutionPlan` when model and runner metadata are valid. `takokit plan <model>` produces a user-facing lifecycle plan even before the model is installed. Runner execution is separate. The current ONNX scaffold returns typed `inference_not_implemented` until real TTS execution is implemented. The whisper.cpp runner executes `whisper-base` when artifacts and runtime are ready.

The first ONNX model target is Piper ONNX. See [decisions/0001-first-onnx-model.md](decisions/0001-first-onnx-model.md).

## Artifact Contract

Checksum-backed artifact install exists for explicit pulls. Downloads go through `~/.takokit/cache/downloads/`, are verified with SHA256, and only then move into `~/.takokit/blobs/sha256/<hash>`. Checksum mismatches delete the temporary file.

`piper-lessac` now records verified Lessac medium ONNX model/config artifacts. `takokit pull piper-lessac` downloads and verifies both files. ONNX execution is still unimplemented.

The ONNX runner scaffold now loads the installed Piper artifact paths and parses the JSON config, but it still does not run an ONNX session or synthesize audio.

## Keep Out For Now

- Tauri app scaffolding.
- Fake Kokoro, Piper, or Python-managed inference.
- Hidden cloud calls.
- Model-specific dependency instructions as the primary user path.
- Install scripts that download nonexistent release artifacts.
