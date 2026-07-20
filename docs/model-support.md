# Takokit model and runner support matrix

Takokit distinguishes implemented support from real-device verification:

- **Locally verified** — installation and real inference output were observed on a recorded device.
- **Executable path** — manifest, pull source, runner, adapter and output contract are implemented, but the model still needs the hardware smoke record before it is advertised as verified.
- **Hardware-blocked on test device** — the execution path exists, but the primary 8 GB laptop GPU cannot satisfy the declared requirement.

A registry entry alone is never treated as verified support. The public library must use the same status and evidence records as the runtime.

## Models

| ID | Family / mode | Capabilities | Runner | Current tier | Primary notes |
|---|---|---|---|---|---|
| `bark-small` | Bark Small | TTS / expressive audio | `takokit-python-managed` | Executable path | CPU/GPU Transformers adapter |
| `canary` | NVIDIA Canary | STT / translation | `takokit-python-managed` | Executable path | NeMo environment; GPU recommended |
| `chatterbox` | Chatterbox | TTS, zero-shot cloning | `takokit-python-managed` | Executable path | Pinned local ResembleAI snapshot; consent-backed reference audio |
| `cosyvoice2` | CosyVoice2 | multilingual TTS, cloning | `takokit-python-managed` | Executable path | Pinned official adapter and model snapshot |
| `dia` | Dia | dialogue TTS | `takokit-python-managed` | Executable path | Speaker-tagged dialogue; GPU recommended |
| `distil-whisper-large-v3` | Distil-Whisper | STT | `takokit-python-managed` | Executable path | GPU recommended |
| `f5-tts` | F5-TTS | TTS, zero-shot cloning | `takokit-python-managed` | Executable path | Consent-backed reference audio |
| `fish-speech` | Fish Speech S2 Pro | expressive TTS, cloning | `takokit-python-managed` | Executable path | Pinned managed runtime; license must remain visible |
| `gpt-sovits` | GPT-SoVITS V2 | TTS, cloning, training | `takokit-python-managed` | Executable path | Reference inference and two-stage local training implemented |
| `kokoro` | Kokoro | TTS | `takokit-onnx` | Executable path | Pinned ONNX model and voices; CPU-friendly |
| `kyutai-tts-1.6b` | Kyutai DSM | English/French TTS | `takokit-python-managed` | Executable path | CUDA-first adapter; official precomputed voice embeddings |
| `mms-tts-eng` | MMS-TTS English | TTS | `takokit-python-managed` | Executable path | CPU/GPU; model license notice required |
| `openvoice` | OpenVoice V2 | TTS, cloning, tone-colour conversion | `takokit-python-managed` | Executable path | Reference-conditioned speech and conversion implemented |
| `parakeet` | NVIDIA Parakeet | STT | `takokit-python-managed` | Executable path | NeMo environment; GPU recommended |
| `piper-lessac` | Piper Lessac medium | TTS | `takokit-python-managed` | Executable path | Official `piper-tts` phonemization and verified ONNX voice artifacts |
| `qwen2-5-omni` | Qwen2.5-Omni 3B | TTS, STT, audio-language | `takokit-python-managed` | Executable path | 32 GB RAM / 8 GB VRAM target; Qwen research license |
| `qwen3-omni` | Qwen3-Omni 30B-A3B | TTS, STT, audio-language | `takokit-python-managed` | Hardware-blocked on test device | Execution path exists; requires about 64 GB RAM / 40 GB VRAM |
| `qwen3-tts` | Qwen3-TTS 0.6B CustomVoice | preset-speaker TTS | `takokit-python-managed` | Executable path | Legacy compatibility ID; use a supported speaker such as `Ryan` |
| `qwen3-tts-0.6b-base` | Qwen3-TTS 0.6B Base | TTS, zero-shot cloning | `takokit-python-managed` | Executable path | Consent-backed reference audio; 4 GB VRAM target |
| `qwen3-tts-1.7b-base` | Qwen3-TTS 1.7B Base | TTS, zero-shot cloning | `takokit-python-managed` | Executable path | Consent-backed reference audio; 8 GB VRAM target |
| `qwen3-tts-1.7b-custom` | Qwen3-TTS 1.7B CustomVoice | preset-speaker TTS | `takokit-python-managed` | Executable path | Official preset-speaker and instruction mode |
| `qwen3-tts-1.7b-voice-design` | Qwen3-TTS 1.7B VoiceDesign | instruction-designed TTS | `takokit-python-managed` | Executable path | Requires `--instruction`; 8 GB VRAM target |
| `rvc` | RVC | voice conversion | `takokit-python-managed` | Executable path | Pinned HuBERT/RMVPE assets; user supplies an owned `.pth` target checkpoint |
| `sensevoice` | SenseVoice | multilingual STT | `takokit-python-managed` | Executable path | FunASR adapter; CPU/GPU |
| `voxtral` | Voxtral Mini | multilingual STT / audio-language | `takokit-python-managed` | Executable path | High-memory GPU recommended |
| `wav2vec2-base-960h` | Wav2Vec2 | English STT | `takokit-python-managed` | Executable path | CPU/GPU Transformers ASR |
| `whisper-base` | Whisper Base | STT | `takokit-whispercpp` | Executable path | Managed Windows/Linux whisper.cpp runtime |
| `whisper-small` | Whisper Small | STT | `takokit-whispercpp` | Executable path | Higher memory and download requirement |
| `whisper-tiny` | Whisper Tiny | STT | `takokit-whispercpp` | Locally verified | Real Windows x64 transcript previously observed |
| `xtts-v2` | Coqui XTTS v2 | multilingual TTS, cloning | `takokit-python-managed` | Executable path | Consent-backed reference audio; Coqui license notice required |
| `yourtts` | Coqui YourTTS | multilingual TTS, cloning | `takokit-python-managed` | Executable path | Consent-backed reference audio; Coqui license notice required |

The bundled production catalog contains **31 model IDs**. `tako test --suite launch` now reads this catalog dynamically rather than maintaining a smaller hard-coded launch list.

## Runners

| Runner | Runtime | Current use | Readiness |
|---|---|---|---|
| `takokit-onnx` | Rust-managed ONNX Runtime | Kokoro | Executable contract; Kokoro hardware smoke still required for verification |
| `takokit-whispercpp` | Managed native whisper.cpp | Whisper Tiny/Base/Small | Windows/Linux runtime implemented; Tiny locally verified |
| `takokit-python-managed` | Isolated Python environment per adapter | Piper, Qwen, Chatterbox, F5, Coqui, Dia, Bark, MMS, Kyutai, SenseVoice, Voxtral, NeMo, OpenVoice, GPT-SoVITS and RVC | Full catalog adapter contract; each family retains an isolated environment and install log |
| `takokit-transformers-audio` | Compatibility contract | No current launch model; Qwen Omni moved to managed Python | Retained for future native service separation |
| `takokit-nemo` | Compatibility contract | No current launch model; Canary/Parakeet use isolated managed Python | Retained for future dedicated NeMo service |

## Installation contract

For every executable-path model, `tako pull <model>` must:

1. Parse the model and runner manifests.
2. Validate platform and declared hardware requirements.
3. Install the runner contract and runtime when missing.
4. Create an isolated adapter environment when required.
5. Pull the pinned model snapshot or checksum-backed artifacts into global Takokit storage.
6. Refuse readiness when any required source, package or artifact is incomplete.
7. Keep inference outputs and history inside the active project `.tako` session.
8. Remain idempotent when repeated.

## Verification gate

Run the catalog-wide plan first:

```powershell
.\target\release\tako.exe test --suite launch --json |
  Tee-Object "$Evidence\launch-catalog.json"
```

Run real model tests with the repository script:

```powershell
.\scripts\run_all_model_smokes.ps1 `
  -Audio C:\path\to\test01_20s.wav `
  -ReferenceAudio C:\path\to\owned-reference.wav `
  -TrainingSamples C:\path\to\gpt-sovits-dataset `
  -RvcTarget C:\path\to\owned-rvc-checkpoint
```

A model moves to **Locally verified** only after its evidence row records the device, commit, pull result, inference command, non-empty output, elapsed time, quality review and retry result. An explicit hardware blocker is not a software pass, but it must be reported separately from an adapter failure.
