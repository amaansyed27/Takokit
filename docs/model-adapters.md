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

## Installed Records

Pulling a model writes a metadata-only installed record:

```txt
~/.takokit/manifests/installed-models/<model>.toml
```

The record stores model id, version, source registry, manifest path, required runner, installed timestamp, artifact placeholders, and status. Artifact placeholders are not downloaded yet.

Pulling a runner writes:

```txt
~/.takokit/manifests/installed-runners/<runner>.toml
```

The runner record stores runner id, version, kind, platforms, manifest path, installed timestamp, and metadata-only status. It is a contract install, not an execution binary install.

## Rules

- Do not hardcode model-specific behavior inside CLI handlers or React components.
- Do not require users to manually install Python, Torch, CUDA, FFmpeg, clone repos, or run model-specific Gradio apps.
- Store model metadata, license metadata, hardware metadata, and artifact checksums in manifests.
- Return typed unsupported/not-installed errors until real runners are implemented.
- Route execution requests through runner resolution before any model adapter is called.
- Check installed model records before checking runner install state for non-mock models.
