# Takokit model support

Takokit separates **catalogued**, **installable**, and **executable** states. A model is only shown as executable after its verified artifacts, runner, and any required isolated adapter are ready.

## Executable built-in paths

| Category | Model | Runtime | Notes |
|---|---|---|---|
| TTS | Kokoro | ONNX | Native Takokit ONNX path with verified model and voice artifacts. |
| TTS | Qwen3-TTS | Managed Python | Official `qwen-tts` adapter. |
| TTS / cloning | Chatterbox | Managed Python | Official Chatterbox API; accepts a consent-backed voice profile or reference audio. |
| TTS / cloning | F5-TTS | Managed Python | Official F5-TTS API; accepts a consent-backed voice profile or reference audio. |
| TTS | Dia | Managed Python | Official Transformers Dia implementation. |
| TTS | Bark Small | Managed Python | Official Transformers `BarkModel` API. |
| TTS | MMS-TTS English | Managed Python | Official Transformers `VitsModel` API; non-commercial model license review required. |
| TTS / cloning | XTTS v2 | Managed Python | Official Coqui TTS API; consent-backed voice profile required. |
| TTS / cloning | YourTTS | Managed Python | Official Coqui TTS API; consent-backed voice profile required. |
| STT | Whisper Tiny | whisper.cpp | Native managed whisper.cpp runtime. |
| STT | Whisper Base | whisper.cpp | Native managed whisper.cpp runtime. |
| STT | Whisper Small | whisper.cpp | Native managed whisper.cpp runtime. |
| STT | SenseVoice | Managed Python | Official FunASR `AutoModel` API. |
| STT | Voxtral Mini | Managed Python | Official Transformers Voxtral transcription API. |
| STT | Canary 1B v2 | Managed Python | Official NVIDIA NeMo ASR API. |
| STT | Parakeet TDT 0.6B v3 | Managed Python | Official NVIDIA NeMo ASR API. |
| STT | Distil-Whisper Large v3 | Managed Python | Official Transformers ASR pipeline. |
| STT | Wav2Vec2 Base 960h | Managed Python | Official Transformers ASR pipeline. |

The built-in `mock-tts` engine remains available for deterministic API and interface tests, but it is not counted as a production model.

## Catalogued but not yet executable

These manifests remain visible for planning and licensing, but Takokit will not report them as executable until a verified adapter exists:

| Model | Reason |
|---|---|
| Piper Lessac | Safe text frontend and phoneme preparation are not yet implemented. |
| CosyVoice2 | Official runtime integration and dependency isolation still required. |
| Fish Speech | Official runtime integration and model-license validation still required. |
| OpenVoice | Tone-color conversion adapter and output contract still required. |
| GPT-SoVITS | Dataset preparation, training-job orchestration, and inference adapter still required. |
| RVC | Voice-conversion training/inference adapter and consent workflow still required. |
| Qwen2.5-Omni | Omni-audio runner is not yet implemented. |
| Qwen3-Omni | Omni-audio runner is not yet implemented. |

## Managed adapter isolation

Each managed adapter receives its own environment below:

```text
~/.takokit/runners/python-managed/adapters/<adapter>/venv
```

Models and caches remain global under `~/.takokit`. Generated audio, transcripts, profile artifacts, and session history are stored in the project that launched Takokit:

```text
<project>/.tako/sessions/<session-id>/outputs
```

## Voice profiles

Chatterbox, F5-TTS, XTTS v2, and YourTTS can use reusable voice profiles. Creating a profile requires explicit confirmation that the user owns the voice or has permission to clone it. Reference audio is copied into global local-only voice storage and the creation event is recorded in the active project session.
