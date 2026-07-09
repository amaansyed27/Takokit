# Takokit Model Library

The Takokit library is curated discovery metadata. It is not the same thing as a runtime manifest.

Runtime manifests live under:

```txt
registry/models/
registry/runners/
```

They describe packages that Takokit can pull into the local runtime lifecycle.

Library manifests live under:

```txt
registry/library/models/
registry/library/runners/
```

They describe models and runners that should appear in a future Takokit website or GUI model browser. Library entries can point to original projects, forks, optimized exports, quantized variants, voice packs, or external runner ecosystems. A library entry does not make a model executable.

## Status Values

- `supported`: Takokit has concrete lifecycle support for the entry. Today this only means `piper-lessac` artifact pull is supported; ONNX inference is still not implemented.
- `experimental`: a runner contract or scaffold exists, but execution is incomplete.
- `planned`: useful target for Takokit, not executable yet.
- `metadata-only`: catalog entry only.
- `blocked-license`: license or commercial-use terms prevent normal runtime support until reviewed.
- `external-runner-needed`: useful model family that needs a runner outside the current native/ONNX/whisper.cpp path.

## Curation Rules

- Do not index every Hugging Face model.
- Prefer original projects, strong forks, optimized exports, quantized variants, and popular community voice/audio families.
- Do not add artifact URLs or SHA256 values unless they are verified for runtime install.
- Do not mark entries as `supported` unless Takokit can actually support that lifecycle surface.
- Keep general text LLMs out unless they are directly voice/audio models.
- Use `unknown` for commercial use when licensing is unclear.
- Use `blocked-license` for non-commercial or custom-license entries that should not become normal runtime packages yet.

## Current Seed Families

The initial model library includes Piper, Kokoro, Whisper, Qwen TTS/Omni, CosyVoice, F5-TTS, Fish Speech, Dia, Spark-TTS, IndexTTS, Chatterbox, GPT-SoVITS, OpenVoice, RVC, SenseVoice, Parakeet, Canary, and Voxtral.

Runner library entries include the Takokit ONNX, whisper.cpp, managed Python, Transformers-audio, and NeMo shared runner contracts.

## CLI

The CLI can print the library metadata as JSON:

```bash
takokit library models
takokit library runners
tako library models
```

These commands do not download models and do not modify `~/.takokit/`.
