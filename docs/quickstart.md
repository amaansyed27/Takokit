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

`quickstart` creates the storage layout, bootstraps the pinned `uv 0.11.24` binary into `%USERPROFILE%\.takokit\tools\uv\`, installs the ONNX and whisper.cpp runners, downloads Kokoro and Whisper Tiny, and runs a real fast smoke suite. It synthesizes `samples\hello.wav` with Kokoro, then transcribes that exact WAV with Whisper Tiny. It fails rather than substituting silence or accepting an empty transcript. Bootstrap source, SHA-256, managed path, version check, and command output are recorded at `%USERPROFILE%\.takokit\logs\uv-bootstrap.log`.

Use structured output when automating the smoke:

```powershell
target\release\takokit.exe test --suite fast --run --json
```

Rows are `passed`, `skipped`, `ready`, or `failed`; executable failures make the command return non-zero.

For the optional Qwen path:

```powershell
target\release\takokit.exe runner pull takokit-python-managed
target\release\takokit.exe runner install takokit-python-managed
target\release\takokit.exe adapter install qwen3_tts
target\release\takokit.exe pull qwen3-tts
target\release\takokit.exe speak "Hello from Takokit" --model qwen3-tts --voice Ryan
```
