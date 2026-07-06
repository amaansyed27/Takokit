import type { RuntimeSnapshot } from "./types";

export const mockRuntime: RuntimeSnapshot = {
  storagePath: "~/.takokit",
  server: {
    status: "online",
    url: "http://127.0.0.1:7860",
    uptime: "00:12:47"
  },
  models: [
    { id: "kokoro-82m", name: "kokoro-82m", purpose: "Fast local TTS", params: "82M", size: "165 MB", language: "English", backend: "CPU", runtime: "Python", status: "installed", capabilities: ["tts"] },
    { id: "kokoro-24m", name: "kokoro-24m", purpose: "Compact local TTS", params: "24M", size: "53 MB", language: "English", backend: "CPU", runtime: "Python", status: "installed", capabilities: ["tts"] },
    { id: "piper-onnx-en_US-lessac-medium", name: "piper-onnx-en_US-lessac-medium", purpose: "Lightweight offline voice", params: "115M", size: "236 MB", language: "English", backend: "CPU", runtime: "ONNX", status: "installed", capabilities: ["tts"] },
    { id: "whisper", name: "Whisper", purpose: "Transcription", params: "-", size: "-", language: "Multilingual", backend: "CPU/GPU", runtime: "whisper.cpp", status: "planned", capabilities: ["stt"] },
    { id: "chatterbox", name: "Chatterbox", purpose: "Voice cloning", params: "-", size: "-", language: "English", backend: "Python", runtime: "Python", status: "planned", capabilities: ["clone"] },
    { id: "gpt-sovits", name: "GPT-SoVITS", purpose: "Few-shot voice training", params: "-", size: "-", language: "Multilingual", backend: "Python", runtime: "Python", status: "planned", capabilities: ["train", "clone"] },
    { id: "qwen3-tts", name: "Qwen3-TTS", purpose: "Voice design and streaming", params: "-", size: "-", language: "Multilingual", backend: "Python", runtime: "Python", status: "planned", capabilities: ["tts", "streaming"] },
    { id: "rvc", name: "RVC", purpose: "Voice conversion", params: "-", size: "-", language: "Voice", backend: "Python", runtime: "Python", status: "planned", capabilities: ["convert"] }
  ],
  voices: [
    { id: "af_sky", name: "af_sky", label: "Female • Calm • American", source: "Kokoro voice pack", model: "kokoro-82m", description: "Female, calm, American", consent: "not required" },
    { id: "am_adam", name: "am_adam", label: "Male • Clear • American", source: "Kokoro voice pack", model: "kokoro-82m", description: "Male, clear, American", consent: "not required" }
  ]
};
