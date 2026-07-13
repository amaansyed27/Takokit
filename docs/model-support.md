# Takokit model and runner support matrix

Takokit targets an Ollama-like experience for local voice models, but it does not equate a registry entry with working support. Every model is assigned one of these tiers:

- **Locally verified** — real installation and inference output has been observed on a supported device.
- **Executable path** — the manifest, runner, isolated adapter and output contract are implemented and compile-tested, but the full upstream download and inference path still requires the hardware smoke guide.
- **Planned / blocked** — discoverable for roadmap and compatibility planning, but Takokit will not report it executable.

The public website must derive its status labels from the same registry and support evidence used by the runtime.

## Models

| ID | Family | Capabilities | Runner | Current tier | Primary device / notes |
|---|---|---|---|---|---|
| `kokoro` | Kokoro | TTS | `takokit-onnx` | Executable path | CPU or GPU; pinned ONNX model and voices |
| `piper-lessac` | Piper | TTS | `takokit-onnx` | Planned / blocked | CPU; safe phoneme frontend is still blocked |
| `qwen3-tts` | Qwen3-TTS | TTS, voice design | `takokit-python-managed` | Executable path | NVIDIA GPU recommended; isolated `qwen3_tts` adapter |
| `chatterbox` | Chatterbox | TTS, zero-shot cloning | `takokit-python-managed` | Executable path | GPU recommended; consent-backed reference voice |
| `f5-tts` | F5-TTS | TTS, zero-shot cloning | `takokit-python-managed` | Executable path | GPU recommended; consent-backed reference voice |
| `xtts-v2` | Coqui XTTS | multilingual TTS, cloning | `takokit-python-managed` | Executable path | CPU/GPU; isolated Coqui environment |
| `yourtts` | YourTTS | multilingual TTS, cloning | `takokit-python-managed` | Executable path | CPU/GPU; isolated Coqui environment |
| `dia` | Dia | dialogue TTS | `takokit-python-managed` | Executable path | NVIDIA GPU recommended |
| `bark-small` | Bark | expressive TTS/audio | `takokit-python-managed` | Executable path | CPU/GPU; Transformers adapter |
| `mms-tts-eng` | MMS-TTS | English TTS | `takokit-python-managed` | Executable path | CPU/GPU; model-license review required |
| `kyutai-tts-1.6b` | Kyutai DSM | English/French TTS, live-audio capable | `takokit-python-managed` | Executable path | CUDA required in the first Takokit adapter; official Moshi API and voice embeddings |
| `cosyvoice2` | CosyVoice2 | TTS, multilingual cloning | `takokit-python-managed` | Planned / blocked | Official isolated adapter still required |
| `fish-speech` | Fish Speech | expressive TTS, cloning | `takokit-python-managed` | Planned / blocked | Runtime and license validation still required |
| `openvoice` | OpenVoice | TTS, tone-colour cloning/conversion | `takokit-python-managed` | Planned / blocked | Conversion contract and dependency validation required |
| `gpt-sovits` | GPT-SoVITS | TTS, cloning, training | `takokit-python-managed` | Planned / blocked | Dataset and training-job orchestration required |
| `rvc` | RVC | voice conversion, training | `takokit-python-managed` | Planned / blocked | Conversion and training jobs required |
| `whisper-tiny` | Whisper | STT | `takokit-whispercpp` | Locally verified | Windows x64 CPU/GPU; real transcript observed |
| `whisper-base` | Whisper | STT | `takokit-whispercpp` | Executable path | Windows x64 runner is implemented |
| `whisper-small` | Whisper | STT | `takokit-whispercpp` | Executable path | Higher memory and download requirement |
| `distil-whisper-large-v3` | Distil-Whisper | STT | `takokit-python-managed` | Executable path | GPU recommended; Transformers ASR pipeline |
| `wav2vec2-base-960h` | Wav2Vec2 | English STT | `takokit-python-managed` | Executable path | CPU/GPU; Transformers ASR pipeline |
| `sensevoice` | SenseVoice | multilingual STT | `takokit-python-managed` | Executable path | FunASR adapter; CPU/GPU |
| `voxtral` | Voxtral Mini | multilingual STT, audio-language | `takokit-python-managed` | Executable path | High-memory GPU recommended |
| `canary` | NVIDIA Canary | multilingual STT/translation | `takokit-python-managed` | Executable path | NVIDIA GPU; NeMo adapter |
| `parakeet` | NVIDIA Parakeet | high-throughput English STT | `takokit-python-managed` | Executable path | NVIDIA GPU; NeMo adapter |
| `qwen2.5-omni` | Qwen Omni | audio-language, STT | `takokit-transformers-audio` | Planned / blocked | Omni-audio execution contract required |
| `qwen3-omni` | Qwen Omni | audio-language, STT, speech output | `takokit-transformers-audio` | Planned / blocked | Omni-audio execution contract required |

The production catalog therefore contains **27 model IDs across TTS, STT, cloning, conversion and training families**. Only models that pass the comprehensive hardware guide may be promoted from executable path to locally verified.

## Runners

| Runner | Runtime | Capabilities | Model families | Readiness |
|---|---|---|---|---|
| `takokit-onnx` | Rust + managed ONNX environment | TTS | Kokoro, Piper and future ONNX audio exports | Kokoro path implemented; Piper frontend blocked |
| `takokit-whispercpp` | managed native whisper.cpp | STT | Whisper Tiny/Base/Small and future GGML Whisper variants | Windows x64 runtime implemented |
| `takokit-python-managed` | one isolated Python environment per adapter | TTS, STT, cloning and future training/conversion | Qwen, Chatterbox, F5, Coqui, Dia, Bark, MMS, Kyutai, SenseVoice, Voxtral, NeMo families | Adapter-specific; never share conflicting dependencies |
| `takokit-transformers-audio` | planned managed Transformers service | audio-language, STT, TTS | Qwen Omni and future multimodal families | Contract only |
| `takokit-nemo` | planned dedicated NeMo service | STT/TTS | Canary, Parakeet and future NeMo families | Contract retained; current Canary/Parakeet adapters use isolated managed Python |

## Installation contract

For an executable-path model, `tako pull <model>` must:

1. Resolve its model and runner manifests.
2. Check platform and hardware requirements.
3. Install the runner contract and runtime when missing.
4. Create an isolated model-family adapter environment when required.
5. Download or lazily resolve upstream weights into global Takokit storage.
6. Verify pinned artifacts where upstream distribution permits checksums.
7. Run readiness checks and mark the model executable only when all required components are present.
8. Keep generated output and history in the active project `.tako` session.

## Voice profiles

Chatterbox, F5-TTS, XTTS v2 and YourTTS can use reusable local voice profiles. Profile creation requires explicit ownership or permission confirmation. The source audio is copied into `~/.takokit/voices/<profile-id>/`, while the creation event is recorded in the active project session.

Kyutai TTS currently uses official precomputed voice embeddings from the Kyutai voice repository. Takokit does not mislabel arbitrary reference audio as Kyutai voice cloning.

## Promotion gate

A model can be advertised as verified only when the evidence sheet records:

- operating system and architecture,
- CPU/GPU and available memory,
- Takokit commit/version,
- dependency installation result,
- model download result,
- inference command,
- validated output path and non-empty output,
- elapsed time,
- failure/retry behaviour,
- test date and tester.
