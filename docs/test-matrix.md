# Launch Test Matrix

This is a runtime matrix, not a catalog. An executable entry has a verified artifact set, a ready runner, and a command that produced a real local result. `planned` and `blocked` entries are intentionally not presented as runnable.

## Verified Windows x64 Run

Integration root: an isolated `TAKOKIT_HOME` under `%TEMP%`. GPU: RTX 5060 Laptop GPU (8 GB). Qwen3-TTS completed on the managed CPU PyTorch fallback in this run; it is real local inference but slower than a CUDA-enabled wheel path.

| Model | Category | Runner | Artifact / runtime state | Command tested | Actual result | Blocker / notes | License and hardware |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `piper-lessac` | TTS | `takokit-onnx` | Lessac model/config pulled and SHA-256 verified | `takokit speak "Hello from Takokit" --model piper-lessac` | Typed `piper_text_frontend_not_implemented` after config/artifact preparation | `piper-plus` is incompatible with upstream Piper phoneme maps; eSpeak-compatible frontend remains behind a GPL boundary | Piper voices MIT; runtime boundary not bundled |
| `kokoro` | TTS | `takokit-onnx` | INT8 model (92,361,271 bytes) and voices (28,214,398 bytes) pulled, SHA-256 verified; runner ready | `takokit speak "Hello from Takokit" --model kokoro` | Real 24 kHz WAV, 55,340 bytes | None on verified CPU path | Apache-2.0 weights; MIT `kokoro-onnx`; CPU-friendly |
| `qwen3-tts` | TTS / built-in voice | `takokit-python-managed` | 11 pinned official files (2,493,919,668 bytes total) pulled, SHA-256 verified and materialized; `qwen3_tts` adapter ready | `takokit speak "Hello from Takokit" --model qwen3-tts --voice Ryan` | Real 24 kHz WAV, 165,164 bytes | Reference-audio cloning is intentionally not exposed | Apache-2.0; CPU fallback verified, CUDA wheel not yet selected |
| `chatterbox` | TTS / cloning | `takokit-python-managed` | Metadata-only; adapter slot `not-installed` | `takokit plan chatterbox` | Blocked plan | Exact pinned weight set and consent-gated adapter still required | MIT upstream; model family is large |
| `f5-tts` | TTS / cloning | `takokit-python-managed` | Metadata-only; adapter slot `not-installed` | `takokit plan f5-tts` | Blocked plan | No commercial-safe verified pretrained artifact selected | Code MIT; common weights CC-BY-NC |
| `whisper-base` | STT | `takokit-whispercpp` | ggml model pulled and SHA-256 verified; whisper.cpp v1.9.1 ready | `takokit transcribe <qwen-wav> --model whisper-base` | Real transcript: `Hello from Togokit.` | None on verified Windows x64 path | MIT; CPU path verified |
| `whisper-tiny` | STT | `takokit-whispercpp` | 77,691,713-byte ggml model pulled and SHA-256 verified; shared runner ready | `takokit transcribe <kokoro-hello.wav> --model whisper-tiny` | Real transcript: `Hello from Tackacit.` in the isolated fast suite | None on verified Windows x64 path | MIT; CPU-friendly |
| `whisper-small` | STT | `takokit-whispercpp` | 487,601,967-byte ggml model pulled and SHA-256 verified; shared runner ready | `takokit transcribe <kokoro-hello.wav> --model whisper-small` | Real transcript: `Hello from Taka Kit.` (28.4 s pull + execution command wall time) | None on verified Windows x64 path; this closes the prior manifest-only gap | MIT; CPU path verified; 6 GB RAM declared |
| `sensevoice` | STT | `takokit-python-managed` | Metadata-only; no managed adapter | `takokit plan sensevoice` | Blocked plan | Exact artifact set and adapter not selected | License/artifacts need verification |
| `parakeet` | STT | `takokit-nemo` | Metadata-only; NeMo runner scaffold only | `takokit plan parakeet` | Blocked plan | NeMo ASR adapter plus managed audio dependencies not installed | Parakeet weights require per-checkpoint review; hardware varies |
| `canary` | STT | `takokit-nemo` | Metadata-only; NeMo runner scaffold only | `takokit plan canary` | Blocked plan | NeMo ASR adapter plus managed audio dependencies not installed | Per-checkpoint license/artifact review required |
| `openvoice` | voice cloning / conversion | `takokit-python-managed` | Metadata-only; adapter slot `not-installed` | `takokit plan openvoice` | Blocked plan | Verified artifact set plus explicit consent gate required | License and consent review required |
| `rvc` | voice conversion | `takokit-python-managed` | Metadata-only; adapter slot `not-installed` | `takokit plan rvc` | Blocked plan | Explicit consent gate and GPL/runtime boundary are required before any adapter install | Do not auto-install or treat as commercial-safe |

## Launch Commands

```bash
takokit doctor
takokit models
takokit runners
takokit test --suite launch
takokit test --suite launch --json
takokit test --suite launch --run
```

`--run` executes only real model handlers and reports lifecycle-blocked models as skipped. The fast suite uses a Kokoro-generated spoken WAV for Whisper, never a silence substitute. For a caller-provided transcription:

```bash
takokit test whisper-base --file ./sample.wav
takokit test whisper-tiny --file ./sample.wav
```

## Deferred Runner Families

`takokit-transformers-audio` covers Qwen Omni and Voxtral. Their manifests remain metadata-only because no exact checkpoint/artifact lock and no managed Transformers audio adapter has been verified on this machine. `takokit-nemo` covers Parakeet and Canary; its exact blocker is the missing managed NeMo ASR adapter and its platform audio-runtime dependencies. Both are represented in CLI/API/GUI plans and runner doctor state, but neither claims execution.
