# Spec Alignment

## Still Aligned

- Local-first voice runtime
- CLI
- Local API
- Model registry
- Engine adapter layer
- GUI
- CLI-first runtime
- Local model lifecycle
- Checksum-backed artifact lifecycle foundation
- Separate execution planning and runner execution
- TTS/STT/voice cloning/training/conversion goals
- Safety/licensing concerns

## Accepted Changes

- Tauri desktop app replaced by local browser GUI.
- Python FastAPI backend replaced by Rust Axum daemon.
- Python is allowed only as an isolated managed runner later, not the main app backend.
- Bare `takokit` opens a local interactive terminal launcher.
- `curl`/`irm` installers are planned for release distribution, with scaffold scripts in the repo until real artifacts exist.
- Piper ONNX is the first real ONNX target; Kokoro ONNX follows after the runner/artifact path is proven.
- Piper voice artifacts can be referenced as downloadable model files, but GPL runtime code from `OHF-Voice/piper1-gpl` must not be vendored without an explicit licensing decision.

## Not Built Yet

- Real Kokoro TTS
- Real Whisper/STT
- Real Piper ONNX TTS
- Voice cloning
- Voice training
- Voice conversion
- Signatures
- Release packaging
- Actual install script release URLs
- Public model library website
