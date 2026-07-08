# Spec Alignment

## Still Aligned

- Local-first voice runtime
- CLI
- Local API
- Model registry
- Engine adapter layer
- GUI
- TTS/STT/voice cloning/training/conversion goals
- Safety/licensing concerns

## Accepted Changes

- Tauri desktop app replaced by local browser GUI.
- Python FastAPI backend replaced by Rust Axum daemon.
- Python is allowed only as an isolated managed runner later, not the main app backend.

## Not Built Yet

- Real Kokoro TTS
- Real Whisper/STT
- Voice cloning
- Voice training
- Voice conversion
- Artifact downloads
- Checksums/signatures
- Public model library website
