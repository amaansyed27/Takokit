# Custom voices and custom models

Takokit treats a **voice** and a **model checkpoint** as different resources.

## Custom voices

A consent-backed reference recording can be stored once and reused by compatible models:

```powershell
tako voice add .\reference.wav --name amaan --model qwen3-tts-1.7b-base --consent
tako voice list
tako voice show qwen3-tts-1.7b-base
tako speak "Hello" --model qwen3-tts-1.7b-base --voice amaan --reference-text "Exact reference transcript"
```

`--voice` accepts:

- a built-in preset name for preset-speaker models;
- a WAV path for reference-audio models;
- a saved Takokit voice-profile ID.

`tako voice show <model>` reports whether the model expects a preset, reference audio,
reference text, or a natural-language voice-design instruction.

## Custom checkpoints

Takokit does not execute arbitrary scripts from a model manifest. A custom checkpoint must
extend a bundled model whose existing runner contract is generic and already implemented.

Supported custom-model bases:

- Qwen3-TTS CustomVoice, Base, and VoiceDesign checkpoints through `takokit-python-managed`;
- Chatterbox-compatible checkpoints through `takokit-python-managed`;
- checksum-pinned Whisper checkpoints accepted by the current `whisper.cpp` runner contract.

Custom ONNX graphs and arbitrary Python repositories are rejected until a verified runner
contract exists for their input/output protocol.

Example Qwen manifest:

```toml
schema_version = 1
id = "my-qwen-voice"
name = "My Qwen Voice"
extends = "qwen3-tts-1.7b-base"
version = "1.0.0"
license = "apache-2.0"
description = "My pinned Qwen3-TTS-compatible checkpoint."

[source]
provider = "hugging-face"
repository = "owner/model"
revision = "0123456789abcdef0123456789abcdef01234567"

[artifacts]
metadata_only = false
weights = []
configs = []
voices = []
```

Register and use it:

```powershell
tako custom-model add .\my-qwen.toml
tako custom-model list
tako plan my-qwen-voice
tako pull my-qwen-voice
tako speak "Hello" --model my-qwen-voice --voice .\reference.wav
```

Sources must use a pinned 40-character Hugging Face commit SHA. Direct model artifacts must
use HTTPS and a full SHA-256 checksum. Local custom models are displayed canonically as
`local/<id>:latest`.
