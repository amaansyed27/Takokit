import type { ModelSummary, RuntimeSnapshot } from "./types";

function model(
  input: Pick<ModelSummary, "id" | "name" | "family" | "purpose" | "backend" | "runner" | "runtime" | "license" | "capabilities"> &
    Partial<ModelSummary>
): ModelSummary {
  return {
    version: "0.1.0",
    language: input.capabilities.includes("stt") ? "Multilingual" : "Local",
    runnerInstalled: false,
    hardwareNotes: "runner contract pending",
    executionStatus: "API offline; start takokit serve or takokit gui",
    artifactCount: 0,
    status: "available",
    lifecycleState: "metadata-only",
    runnerRuntimeState: "runtime-missing",
    executable: false,
    missing: ["local API offline"],
    nextCommand: "takokit serve",
    ...input
  };
}

export const mockRuntime: RuntimeSnapshot = {
  storagePath: "~/.takokit",
  server: {
    status: "offline",
    url: "http://127.0.0.1:5050",
    uptime: "API unavailable"
  },
  models: [
    model({
      id: "mock-tts",
      name: "Mock TTS",
      family: "internal-test",
      purpose: "Internal deterministic WAV generator for API and CLI scaffolding.",
      backend: "native_rust",
      runner: "takokit-mock",
      runtime: "Rust",
      status: "installed",
      license: "internal-test",
      runnerInstalled: true,
      lifecycleState: "executable",
      runnerRuntimeState: "ready",
      executable: true,
      missing: [],
      nextCommand: "takokit speak \"hello\" --model mock-tts",
      executionStatus: "internal test path executable",
      capabilities: ["tts", "live_audio"]
    }),
    model({
      id: "piper-lessac",
      name: "Piper Lessac",
      family: "piper",
      purpose: "Verified Piper Lessac medium ONNX model/config artifacts. TTS execution is blocked.",
      backend: "onnx",
      runner: "takokit-onnx",
      runtime: "ONNX",
      license: "mit",
      artifactCount: 2,
      missing: ["local API offline"],
      capabilities: ["tts", "live_audio"]
    }),
    model({
      id: "whisper-base",
      name: "Whisper Base",
      family: "whisper",
      purpose: "Whisper Base STT through the local whisper.cpp runner when installed.",
      backend: "whispercpp",
      runner: "takokit-whispercpp",
      runtime: "whisper.cpp",
      license: "mit",
      artifactCount: 1,
      capabilities: ["stt", "live_transcription"]
    }),
    model({
      id: "qwen3-tts",
      name: "Qwen3-TTS",
      family: "qwen",
      purpose: "Managed Python TTS and voice design target. Adapter slot only.",
      backend: "python-managed",
      runner: "takokit-python-managed",
      runtime: "Python",
      license: "apache-2.0",
      capabilities: ["tts", "voice_cloning", "live_audio"]
    })
  ],
  runners: [
    { id: "takokit-onnx", name: "Takokit ONNX Runner", version: "0.1.0", kind: "onnx", platforms: ["windows-x64", "linux-x64", "macos-arm64"], install_state: "runtime-missing", dependency_strategy: "bundled-native", supported_model_families: ["Piper", "Kokoro ONNX"], supported_tasks: ["text_to_speech", "live_audio"], description: "Shared ONNX runner. Piper execution is blocked on phonemizer/token preparation and ONNX session execution.", installed: false },
    { id: "takokit-whispercpp", name: "Takokit whisper.cpp Runner", version: "0.1.0", kind: "whispercpp", platforms: ["windows-x64"], install_state: "runtime-missing", dependency_strategy: "bundled-native", supported_model_families: ["Whisper"], supported_tasks: ["speech_to_text", "live_transcription"], description: "Shared whisper.cpp runner for local STT.", installed: false },
    { id: "takokit-python-managed", name: "Takokit Managed Python Runner", version: "0.1.0", kind: "python-managed", platforms: ["windows-x64", "linux-x64", "macos-arm64"], install_state: "runtime-missing", dependency_strategy: "managed", supported_model_families: ["Qwen", "F5-TTS", "Chatterbox", "RVC"], supported_tasks: ["text_to_speech", "voice_cloning", "live_audio"], description: "Managed Python runner layout and adapter slots.", installed: false }
  ],
  voices: [
    { id: "default", name: "Default mock voice", label: "Default mock voice", source: "takokit-mock", model: "mock-tts", description: "Internal test voice. No cloning or training has run.", consent: "not required" }
  ],
  capabilities: [
    { id: "tts", label: "TTS", description: "Text input to speech or audio output." },
    { id: "stt", label: "STT", description: "Audio file or input to text transcript." },
    { id: "voice_cloning", label: "Voice Cloning", description: "Voice samples to a reusable local voice profile." },
    { id: "live_transcription", label: "Live Transcription API", description: "Local STT models exposed through an API for streaming or submitted audio." },
    { id: "live_audio", label: "Live Audio API", description: "Compatible local voice models exposed through an API for speech output." }
  ],
  modeNote: "API offline. Start takokit serve or takokit gui to inspect real runtime state."
};
