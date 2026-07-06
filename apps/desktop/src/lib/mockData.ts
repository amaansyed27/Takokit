import type { RuntimeSnapshot } from "./types";

export const mockRuntime: RuntimeSnapshot = {
  storagePath: "~/.takokit",
  server: {
    status: "online",
    url: "http://127.0.0.1:5050",
    uptime: "00:12:47"
  },
  models: [
    { id: "kokoro-82m", name: "kokoro-82m", purpose: "Fast local TTS", params: "82M", size: "165 MB", language: "English", backend: "CPU", runtime: "Python", status: "installed", license: "Apache-2.0", capabilities: ["tts"] },
    { id: "kokoro-24m", name: "kokoro-24m", purpose: "Compact local TTS", params: "24M", size: "53 MB", language: "English", backend: "CPU", runtime: "Python", status: "installed", license: "Apache-2.0", capabilities: ["tts"] },
    { id: "piper-onnx-en_US-lessac-medium", name: "piper-onnx-en_US-lessac-medium", purpose: "Lightweight offline voice", params: "115M", size: "236 MB", language: "English", backend: "CPU", runtime: "ONNX", status: "installed", license: "MIT", capabilities: ["tts"] },
    { id: "whisper.cpp-base", name: "whisper.cpp-base", purpose: "Transcription", params: "74M", size: "142 MB", language: "Multilingual", backend: "CPU", runtime: "whisper.cpp", status: "available", license: "MIT", capabilities: ["stt"] },
    { id: "chatterbox", name: "chatterbox", purpose: "Voice cloning", params: "-", size: "-", language: "English", backend: "Python", runtime: "Python", status: "planned", license: "Review", capabilities: ["clone"] },
    { id: "gpt-sovits", name: "gpt-sovits", purpose: "Few-shot voice training", params: "-", size: "-", language: "Multilingual", backend: "Python", runtime: "Python", status: "planned", license: "Review", capabilities: ["train", "clone"] },
    { id: "qwen3-tts", name: "qwen3-tts", purpose: "Voice design and streaming", params: "-", size: "-", language: "Multilingual", backend: "Python", runtime: "Python", status: "planned", license: "Review", capabilities: ["tts", "streaming"] },
    { id: "rvc", name: "rvc", purpose: "Voice conversion", params: "-", size: "-", language: "Voice", backend: "Python", runtime: "Python", status: "planned", license: "Review", capabilities: ["convert"] }
  ],
  voices: [
    { id: "af_sky", name: "af_sky", label: "Female - Calm - American", source: "Kokoro voice pack", model: "kokoro-82m", description: "Default warm test voice for local speech generation.", consent: "not required" },
    { id: "local_default", name: "local_default", label: "Neutral - Local - Mock", source: "Takokit mock voice", model: "mock-tts", description: "Local placeholder voice. No real cloning or training has run.", consent: "not required" }
  ]
};

