# 0001 First ONNX Model Target

## Status

Accepted.

## Decision

Takokit will target Piper ONNX first for the first real ONNX runner pass. Kokoro ONNX remains the next TTS target after Piper proves the artifact download, checksum, model manifest, and local runner path end to end.

## Context

Takokit needs the smallest honest path from:

```txt
takokit pull <model>
takokit runner pull takokit-onnx
takokit speak "Hello" --model <model>
```

to real local audio without cloud calls or manual dependency setup.

Piper is described upstream as a fast local neural TTS system, and the public Piper voices repository is organized as ONNX voice files with matching config metadata, covers many languages, and is marked MIT licensed:

- https://github.com/rhasspy/piper
- https://huggingface.co/rhasspy/piper-voices

Kokoro is a strong follow-up target. The upstream Kokoro project describes an 82M parameter open-weight TTS model with Apache-licensed weights, and kokoro-onnx provides an ONNX Runtime package with MIT code and Apache model licensing:

- https://github.com/hexgrad/kokoro
- https://github.com/thewh1teagle/kokoro-onnx

Kokoro ONNX currently has a more involved package shape for Takokit's first runner slice: model file, voices file, and G2P/tokenizer expectations. Piper's voice package maps more directly to Takokit's next required work: real model artifact manifest, checksum-backed download, and ONNX runner execution.

## Criteria

- CPU-first local execution: Piper is designed for local speech and already has ONNX voices. Kokoro is also CPU-capable through ONNX Runtime but has a larger model/voice packaging surface.
- Minimal external dependencies: Piper still needs phonemization work, but the first Takokit slice can focus on one known voice/config pair. Kokoro adds a separate voices file and G2P/tokenizer story immediately.
- Easy package format: Piper voice artifacts are naturally model plus JSON config. This fits Takokit's manifest model.
- License clarity: Piper voices are MIT. Kokoro model weights are Apache 2.0 and kokoro-onnx code is MIT.
- Small artifact size: Piper low/medium voice artifacts are practical for the first checksum-backed pull. Kokoro's quantized option is attractive but still adds voices packaging.
- Install-once UX: Piper gives the shortest path to `pull piper-lessac`, verify artifacts, and execute locally.

## Consequences

- Next implementation work should rename the current placeholder from metadata-only to a real Piper ONNX artifact manifest once checksum-backed downloads exist.
- The ONNX runner should be implemented against a single Piper voice/config pair first.
- Kokoro ONNX should remain in the registry as a planned TTS + Live Audio API model, but it should keep returning `inference_not_implemented` until Piper proves the runner path.
