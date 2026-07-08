import type { RuntimeSnapshot } from "./types";

export const mockRuntime: RuntimeSnapshot = {
  storagePath: "~/.takokit",
  server: {
    status: "online",
    url: "http://127.0.0.1:5050",
    uptime: "00:12:47"
  },
  models: [
    { id: "mock-tts", name: "Mock TTS", purpose: "Deterministic test WAV generator for API and CLI scaffolding.", version: "0.1.0", language: "Local", backend: "native_rust", runtime: "Rust", status: "installed", license: "internal-test", capabilities: ["tts", "live_audio"] },
    { id: "kokoro", name: "Kokoro", purpose: "Fast local text-to-speech model. Mock registry entry only.", version: "0.1.0", language: "English", backend: "onnx", runtime: "ONNX", status: "available", license: "apache-2.0", capabilities: ["tts", "live_audio"] },
    { id: "piper-lessac", name: "Piper Lessac", purpose: "Piper English Lessac placeholder.", version: "0.1.0", language: "English", backend: "onnx", runtime: "ONNX", status: "available", license: "mit", capabilities: ["tts", "live_audio"] },
    { id: "whisper-base", name: "Whisper Base", purpose: "Whisper transcription placeholder.", version: "0.1.0", language: "Multilingual", backend: "whispercpp", runtime: "whisper.cpp", status: "available", license: "mit", capabilities: ["stt", "live_transcription"] },
    { id: "chatterbox", name: "Chatterbox", purpose: "Voice cloning and TTS placeholder.", version: "0.1.0", language: "Local", backend: "python-managed", runtime: "Python", status: "available", license: "research-check-required", capabilities: ["tts", "voice_cloning", "live_audio"] },
    { id: "gpt-sovits", name: "GPT-SoVITS", purpose: "Few-shot voice cloning and TTS placeholder.", version: "0.1.0", language: "Local", backend: "python-managed", runtime: "Python", status: "available", license: "research-check-required", capabilities: ["tts", "voice_cloning", "live_audio"] }
  ],
  runners: [
    { id: "takokit-onnx", name: "Takokit ONNX Runner", version: "0.1.0", kind: "native", platforms: ["windows-x64", "linux-x64", "macos-arm64"], description: "Runner contract only. Execution is not implemented yet.", installed: false },
    { id: "takokit-whispercpp", name: "Takokit whisper.cpp Runner", version: "0.1.0", kind: "whispercpp", platforms: ["windows-x64", "linux-x64", "macos-arm64"], description: "Runner contract only. Execution is not implemented yet.", installed: false },
    { id: "takokit-python", name: "Takokit Managed Python Runner", version: "0.1.0", kind: "python-managed", platforms: ["windows-x64", "linux-x64", "macos-arm64"], description: "Future isolated Python runner contract.", installed: false }
  ],
  voices: [
    { id: "local_default", name: "local_default", label: "Neutral - Local - Mock", source: "Takokit mock voice", model: "mock-tts", description: "Local placeholder voice. No real cloning or training has run.", consent: "not required" }
  ],
  capabilities: [
    { id: "tts", label: "TTS", description: "Text input to speech or audio output." },
    { id: "stt", label: "STT", description: "Audio file or input to text transcript." },
    { id: "voice_cloning", label: "Voice Cloning", description: "Voice samples to a reusable local voice profile." },
    { id: "live_transcription", label: "Live Transcription API", description: "Local STT models exposed through an API for streaming or submitted audio." },
    { id: "live_audio", label: "Live Audio API", description: "Compatible local voice models exposed through an API for speech output." }
  ],
  modeNote: "Mock mode: package metadata is shown, but real model inference is not implemented."
};
