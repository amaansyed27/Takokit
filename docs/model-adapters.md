# Model Adapters And Packages

Takokit keeps model-specific implementation behind adapter and package contracts.

Current adapter traits:

```rust
TextToSpeechEngine
SpeechToTextEngine
VoiceCloneEngine
```

`mock-tts` is the only executable speech engine today. It writes a deterministic test WAV and is not real inference.

Package resolution and runner execution are separate. `takokit-package` builds an `ExecutionPlan` from manifests and installed records. Runner engines consume that plan and either produce output or return a typed execution error.

Current runner traits:

```rust
SpeechRunner
TranscriptionRunner
```

The ONNX runner scaffold exists in `takokit-models`, but it returns:

```txt
inference_not_implemented: ONNX runner contract resolved, but real ONNX execution is not implemented yet.
```

It does not generate Kokoro, Piper, or any other real-model audio yet.

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
kind = "onnx"
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
- Return typed unsupported/not-installed planning errors before execution.
- Return typed `inference_not_implemented` from runner execution scaffolds until real runners are implemented.
- Route execution requests through execution planning before any runner engine is called.
- Check installed model records before checking runner install state for non-mock models.
- Keep `takokit-package` responsible for manifests, installed state, and planning only. Keep execution in model/runner crates.

## First ONNX Target

Piper ONNX is the first real ONNX target. The decision is recorded in [decisions/0001-first-onnx-model.md](decisions/0001-first-onnx-model.md). Kokoro ONNX remains the next target after Piper proves the artifact and runner path.
