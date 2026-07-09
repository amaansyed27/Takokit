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

Today, `whisper-base` can become executable when its verified artifact is pulled and `takokit-whispercpp` is installed on Windows x64. `mock-tts` remains the only TTS path that writes audio, and it is an internal contract-test model rather than a real model family.

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
  adapters/
```

Users should not manually install Python, Torch, CUDA packages, FFmpeg, clone model repositories, run Gradio apps, or patch requirements files. When this runner becomes executable, Takokit must own that setup behind `takokit runner pull` and model-specific adapters.

`takokit runner install takokit-python-managed` initializes this layout and writes adapter slots for `qwen3_tts`, `chatterbox`, `f5_tts`, `cosyvoice2`, `dia`, `fish_speech`, `openvoice`, `gpt_sovits`, and `rvc`. Each adapter slot is marked `not-installed`; Takokit does not install Python/Torch or model dependencies yet.

`takokit doctor` checks that this layout exists and reports the runtime state without failing the current development path.

## Commands

```bash
takokit runners
takokit runner show takokit-onnx
takokit runner show takokit-python-managed
takokit runner pull takokit-whispercpp
takokit runner install takokit-whispercpp
takokit runner doctor takokit-whispercpp
takokit runner doctor takokit-whispercpp --json
takokit plan whisper-base
```

Pulling a runner installs the contract record only. Installing a runner performs explicit runtime setup:

- `takokit-whispercpp`: on Windows x64, downloads the official whisper.cpp v1.9.1 `whisper-bin-x64.zip`, verifies SHA256 `7d8be46ecd31828e1eb7a2ecdd0d6b314feafd82163038ab6092594b0a063539`, extracts it under `~/.takokit/runners/whispercpp/runtime/`, and marks the runner `ready` only if `whisper-cli.exe` is present.
- `takokit-python-managed`: creates the managed runtime layout and planned adapter slots. Python/Torch and model adapters are not installed yet.
- `takokit-onnx`, `takokit-transformers-audio`, and `takokit-nemo`: create runtime directories and record the exact missing implementation component.

Failed runtime installs persist `failed` state in the installed runner record with a note and log path. Re-running `takokit runner install <runner>` is safe and attempts to repair the runtime state.
