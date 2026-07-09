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
configs = []
voices = []
```

Artifact-backed manifests can declare typed model/config files:

```toml
[artifacts]
metadata_only = false

[[artifacts.weights]]
name = "en_US-lessac-medium.onnx"
url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
sha256 = "5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f"
bytes = 63201294
role = "model"

[[artifacts.configs]]
name = "en_US-lessac-medium.onnx.json"
url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"
sha256 = "efe19c417bed055f2d69908248c6ba650fa135bc868b0e6abb3da181dab690a0"
bytes = 4885
role = "config"
```

Blank checksums are not valid for artifact install. They are allowed only when a manifest or pull request is explicitly metadata-only.

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

Pulling a metadata-only model writes an installed record:

```txt
~/.takokit/manifests/installed-models/<model>.toml
```

The record stores model id, version, source registry, manifest path, required runner, installed timestamp, artifact URL/checksum/role/local path, downloaded state, and status. Verified artifact installs store files under `~/.takokit/blobs/sha256/<hash>` and set `downloaded = true` for each installed artifact.

Pulling a runner writes:

```txt
~/.takokit/manifests/installed-runners/<runner>.toml
```

The runner record stores runner id, version, kind, platforms, manifest path, installed timestamp, and metadata-only status. It is a contract install, not an execution binary install.

## Rules

- Do not hardcode model-specific behavior inside CLI handlers or React components.
- Do not require users to manually install Python, Torch, CUDA, FFmpeg, clone repos, or run model-specific Gradio apps.
- Store model metadata, license metadata, hardware metadata, and artifact checksums in manifests.
- Require SHA256 before artifact-backed downloads.
- Delete temporary downloads on checksum mismatch.
- Return typed unsupported/not-installed planning errors before execution.
- Return typed `inference_not_implemented` from runner execution scaffolds until real runners are implemented.
- Route execution requests through execution planning before any runner engine is called.
- Check installed model records before checking runner install state for non-mock models.
- Keep `takokit-package` responsible for manifests, installed state, and planning only. Keep execution in model/runner crates.

## First ONNX Target

Piper ONNX is the first real ONNX target. The decision is recorded in [decisions/0001-first-onnx-model.md](decisions/0001-first-onnx-model.md). Kokoro ONNX remains the next target after Piper proves the artifact and runner path.

The old `rhasspy/piper` runtime repository is archived and points to `OHF-Voice/piper1-gpl`, which is GPL-3.0. Takokit may reference Piper voice artifacts, but must not vendor GPL runtime code without an explicit licensing decision. See [references.md](references.md).
