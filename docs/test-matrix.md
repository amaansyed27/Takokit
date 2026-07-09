# Launch Test Matrix

This matrix tracks launch-relevant runtime manifests. It is intentionally honest: models are not executable unless Takokit has a verified artifact path, a ready runner runtime, and a real executor.

## Commands

```bash
takokit test --suite launch
takokit test --suite launch --json
takokit plan <model>
takokit plan <model> --json
takokit test <model> --file ./sample.wav
```

The suite is non-destructive. It parses manifests and installed records, then reports lifecycle state, missing pieces, and next command. It does not pull artifacts or install runners.

## Verified In This Run

Integration root: temporary `TAKOKIT_HOME` under `%TEMP%`.

```bash
takokit runner pull takokit-whispercpp
takokit runner install takokit-whispercpp
takokit pull whisper-base
takokit transcribe <generated-sapi-sample.wav> --model whisper-base
```

Result: real whisper.cpp transcription returned `Hello from Taco Kid.` for a generated local WAV saying "hello from takokit". The runner installed whisper.cpp v1.9.1 from the official Windows x64 release ZIP and verified SHA256 `7d8be46ecd31828e1eb7a2ecdd0d6b314feafd82163038ab6092594b0a063539`.

## Matrix

| Model | Category | Runner | Artifact status | Runner status | Executable | Command tested | Result | Blocker | License/commercial status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `piper-lessac` | TTS | `takokit-onnx` | Verified ONNX/config artifacts available | Runtime-installed scaffold | No | `takokit speak "Hello" --model piper-lessac` | Typed `piper_text_frontend_not_implemented` after artifact/config prep | text normalization, phonemizer/token preparation, then ONNX session execution | Piper voice artifacts usable; current upstream runtime licensing needs GPL boundary review |
| `kokoro` | TTS | `takokit-onnx` | Metadata-only | Runtime scaffold | No | `takokit plan kokoro` | Planned | verified ONNX artifacts and ONNX execution | Apache-style metadata in registry; artifact source not verified |
| `whisper-base` | STT | `takokit-whispercpp` | Verified ggml artifact | Ready on Windows x64 | Yes | `takokit transcribe <sample.wav> --model whisper-base` | Real transcript | None on verified Windows x64 path | MIT |
| `qwen3-tts` | TTS / cloning | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan qwen3-tts` | Planned | verified artifacts and managed adapter install/run | Apache 2.0 upstream; heavy Python runtime not installed |
| `cosyvoice2` | TTS / cloning | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan cosyvoice2` | Planned | verified artifacts and managed adapter install/run | Non-commercial risk noted in registry |
| `f5-tts` | TTS / cloning | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan f5-tts` | Planned | verified artifacts and managed adapter install/run | Code MIT, pretrained weights CC-BY-NC |
| `dia` | TTS | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan dia` | Planned | verified artifacts and managed adapter install/run | License needs per-artifact verification |
| `fish-speech` | TTS / cloning | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan fish-speech` | Planned | verified artifacts and managed adapter install/run | License needs per-artifact verification |
| `sensevoice` | STT | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan sensevoice` | Planned | verified artifacts and managed adapter install/run | License needs per-artifact verification |
| `parakeet` | STT | `takokit-nemo` | Metadata-only | Runtime scaffold | No | `takokit plan parakeet` | Planned | NeMo adapter and managed dependencies | License needs per-artifact verification |
| `canary` | STT | `takokit-nemo` | Metadata-only | Runtime scaffold | No | `takokit plan canary` | Planned | NeMo adapter and managed dependencies | License needs per-artifact verification |
| `openvoice` | voice cloning / conversion | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan openvoice` | Planned | verified artifacts, consent-gated adapter, managed runtime | License and consent restrictions require review |
| `rvc` | voice conversion | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan rvc` | Planned | verified artifacts, consent-gated adapter, managed runtime | GPL/runtime risk documented; do not auto-install without boundary |
| `gpt-sovits` | TTS / cloning | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan gpt-sovits` | Planned | verified artifacts, consent-gated adapter, managed runtime | License/runtime risk requires review |
| `chatterbox` | TTS / cloning | `takokit-python-managed` | Metadata-only | Runtime layout only | No | `takokit plan chatterbox` | Planned | verified artifacts and managed adapter install/run | MIT upstream, adapter not installed |

## Next Verification Targets

- Piper ONNX: validate a non-GPL text frontend, then implement ONNX Runtime session loading, tensor execution, and WAV output.
- Whisper Tiny/Small: add manifests only after verified artifact SHA256 values are computed.
- Python-managed: implement one adapter end-to-end after dependency lock, license review, consent policy, and reproducible install plan.
