# Quickstart (Windows)

The fast local stack is Kokoro for TTS and Whisper Tiny for STT. Qwen3-TTS is optional and heavier.

```powershell
cargo build --release
target\release\takokit.exe quickstart
target\release\takokit.exe samples create
target\release\takokit.exe speak "Hello from Takokit" --model kokoro
target\release\takokit.exe transcribe "$env:USERPROFILE\.takokit\samples\hello.wav" --model whisper-tiny
target\release\takokit.exe test --suite fast --run
target\release\takokit.exe gui
```

`quickstart` creates the storage layout, bootstraps `uv` when needed, installs the ONNX and whisper.cpp runners, downloads Kokoro and Whisper Tiny, and runs the fast smoke suite. Bootstrap output is logged at `%USERPROFILE%\.takokit\logs\uv-bootstrap.log`.

For the optional Qwen path:

```powershell
target\release\takokit.exe runner pull takokit-python-managed
target\release\takokit.exe runner install takokit-python-managed
target\release\takokit.exe adapter install qwen3_tts
target\release\takokit.exe pull qwen3-tts
target\release\takokit.exe speak "Hello from Takokit" --model qwen3-tts --voice Ryan
```
