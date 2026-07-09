# Runners

Takokit uses shared runners. It does not create one runner per model and it is not a Piper wrapper.

Runtime runner manifests live under `registry/runners/`. They are parseable contracts used by CLI/API planning logic, not decorative catalog entries.

## Runtime Runner Families

- `takokit-onnx`: shared ONNX runner for Piper, Kokoro ONNX, and future ONNX voice/audio exports.
- `takokit-whispercpp`: shared whisper.cpp runner for Whisper tiny/base/small/medium/large STT.
- `takokit-python-managed`: managed Python/PyTorch runner for Qwen3-TTS, CosyVoice/CosyVoice2, F5-TTS, Fish Speech, Dia, Chatterbox, GPT-SoVITS, OpenVoice, RVC, and similar research models.
- `takokit-transformers-audio`: planned managed runner for Qwen Omni, Voxtral, and other audio-language models.
- `takokit-nemo`: planned managed runner for NeMo-style ASR/TTS models such as Parakeet and Canary.

## Lifecycle States

Runner lifecycle states:

- `contract-installed`: Takokit installed the runner contract manifest locally.
- `runtime-installed`: runner runtime bits exist, but readiness is not confirmed.
- `ready`: runner runtime is ready to execute.
- `failed`: runner setup failed.
- `runtime-missing`: internal planning state for runners that have not been pulled or initialized.

Model lifecycle states:

- `metadata-only`: Takokit knows the model exists, but has no verified installed artifacts.
- `artifacts-ready`: model artifacts are installed and recorded as downloaded.
- `runner-ready`: the required runner is ready, but model execution has not been proven.
- `executable`: Takokit can actually run the model.
- `failed`: install or execution setup failed.

Today, no runtime model is marked executable. `mock-tts` remains the only path that writes audio, and it is an internal contract-test model rather than a real model family.

## Python-Managed Layout

The managed Python runner layout is created under:

```txt
~/.takokit/runners/python-managed/
  runtime/
  env/
  packages/
  wheels/
  logs/
  manifests/
  cache/
```

Users should not manually install Python, Torch, CUDA packages, FFmpeg, clone model repositories, run Gradio apps, or patch requirements files. When this runner becomes executable, Takokit must own that setup behind `takokit runner pull` and model-specific adapters.

`takokit doctor` checks that this layout exists and reports the runtime as not initialized without failing the current development path.

## Commands

```bash
takokit runners
takokit runner show takokit-onnx
takokit runner show takokit-python-managed
takokit runner pull takokit-whispercpp
takokit plan whisper-base
```

Pulling a runner currently installs the contract record only. It does not install Python, Torch, CUDA, FFmpeg, ONNX Runtime, whisper.cpp binaries, or NeMo packages yet.
