# Model Adapters

Takokit keeps model-specific implementation behind adapter traits.

Current traits:

```rust
TextToSpeechEngine
SpeechToTextEngine
VoiceCloneEngine
VoiceTrainingEngine
VoiceConversionEngine
```

The UI and API should not know whether a model runs through Python, ONNX, whisper.cpp, or native Rust. They should work with model IDs, voice IDs, request contracts, and registry metadata.

## Initial Registry

| Model | Purpose | Planned Runtime |
| --- | --- | --- |
| Kokoro | Fast local TTS | Python runner |
| Whisper | Transcription | whisper.cpp |
| Chatterbox | Voice cloning | Python runner |
| GPT-SoVITS | Few-shot voice training | Python runner |
| Qwen3-TTS | Voice design and streaming | Python runner |
| RVC | Voice conversion | Python runner |
| Piper | Lightweight offline voices | ONNX |

`mock-tts` is included as an installed test adapter. It is not a real inference engine.

## Adapter Rules

- Keep model setup and dependencies out of UI code.
- Keep runner process management out of API handlers where possible.
- Return typed errors for unsupported features.
- Record model license, hardware, and safety metadata before download or execution.

