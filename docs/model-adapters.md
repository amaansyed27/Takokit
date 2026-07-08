# Model Adapters And Packages

Takokit keeps model-specific implementation behind adapter and package contracts.

Current adapter traits:

```rust
TextToSpeechEngine
SpeechToTextEngine
VoiceCloneEngine
```

`mock-tts` is the only executable speech engine today. It writes a deterministic test WAV and is not real inference.

Takokit model manifests describe the five product surfaces:

- TTS
- STT
- Voice Cloning
- Live Transcription Local API
- Live Audio API

## Model Manifest

```toml
id = "kokoro"
name = "Kokoro"
version = "0.1.0"
kind = "tts"
backend = "onnx"
runner = "takokit-onnx"
license = "apache-2.0"
description = "Fast local text-to-speech model."

[capabilities]
tts = true
stt = false
voice_cloning = false
live_transcription = false
live_audio = true

[hardware]
cpu = true
gpu = false
min_ram = "4gb"

[artifacts]
weights = []
voices = []
```

## Runner Manifest

```toml
id = "takokit-onnx"
name = "Takokit ONNX Runner"
version = "0.1.0"
kind = "native"
platforms = ["windows-x64", "linux-x64", "macos-arm64"]
description = "Native ONNX runner for CPU-friendly models."
```

## Rules

- Do not hardcode model-specific behavior inside CLI handlers or React components.
- Do not require users to manually install Python, Torch, CUDA, FFmpeg, clone repos, or run model-specific Gradio apps.
- Store model metadata, license metadata, hardware metadata, and artifact checksums in manifests.
- Return typed unsupported/not-installed errors until real runners are implemented.
- Route execution requests through runner resolution before any model adapter is called.
