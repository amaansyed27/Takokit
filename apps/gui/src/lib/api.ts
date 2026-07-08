import { mockRuntime } from "./mockData";
import type { CapabilitySummary, ModelCapability, ModelSummary, RunnerSummary, RuntimeSnapshot, SpeechApiRequest, SpeechApiResponse, VoiceSummary } from "./types";

const LOCAL_API_BASE_URL = "http://127.0.0.1:5050";

type ApiStatus = {
  server: string;
  storage_root: string;
};

type ApiModel = {
  id: string;
  name: string;
  version: string;
  summary: string;
  license: string;
  runtime: "python" | "onnx" | "whisper_cpp" | "native_rust" | "external";
  backend: string;
  runner: string;
  hardware_notes: string;
  capabilities: ApiCapabilityId[];
  installed: boolean;
  runner_installed: boolean;
  execution_status: string;
};

type ApiCapabilityId = "text_to_speech" | "speech_to_text" | "voice_cloning" | "live_transcription" | "live_audio";

type ApiCapability = {
  id: ApiCapabilityId;
  label: string;
  description: string;
};

type ApiRunner = RunnerSummary;

type ApiVoice = {
  id: string;
  name: string;
  source: string;
  model_id?: string;
  consent_required: boolean;
};

export async function generateSpeech(request: SpeechApiRequest): Promise<SpeechApiResponse> {
  const response = await fetch(`${LOCAL_API_BASE_URL}/v1/audio/speech`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify(request)
  });

  if (!response.ok) {
    throw new Error(`Speech generation failed with ${response.status}`);
  }

  return response.json() as Promise<SpeechApiResponse>;
}

export const apiConfig = {
  localBaseUrl: LOCAL_API_BASE_URL,
  guiUrl: `${LOCAL_API_BASE_URL}/gui`
};

export async function loadRuntimeSnapshot(): Promise<RuntimeSnapshot> {
  try {
    const [status, capabilities, models, runners, voices] = await Promise.all([
      getJson<ApiStatus>("/v1/status"),
      getJson<{ data: ApiCapability[] }>("/v1/capabilities"),
      getJson<{ data: ApiModel[] }>("/v1/models"),
      getJson<{ data: ApiRunner[] }>("/v1/runners"),
      getJson<{ data: ApiVoice[] }>("/v1/voices")
    ]);

    return {
      storagePath: status.storage_root,
      server: {
        status: "online",
        url: LOCAL_API_BASE_URL,
        uptime: "daemon online"
      },
      models: [mockSpeechModel, ...models.data.map(toModelSummary)],
      runners: runners.data,
      voices: voices.data.map(toVoiceSummary),
      capabilities: capabilities.data.map(toCapabilitySummary),
      modeNote: "Mock mode: packages and runners are managed locally, but real model inference is not implemented."
    };
  } catch {
    return mockRuntime;
  }
}

const mockSpeechModel: ModelSummary = {
  id: "mock-tts",
  name: "Mock TTS",
  purpose: "Deterministic test WAV generator for API and CLI scaffolding.",
  version: "0.1.0",
  language: "Local",
  backend: "native_rust",
  runtime: "Rust",
  status: "installed",
  license: "internal-test",
  capabilities: ["tts", "live_audio"]
};

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${LOCAL_API_BASE_URL}${path}`);
  if (!response.ok) {
    throw new Error(`Takokit API request failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

function toModelSummary(model: ApiModel): ModelSummary {
  return {
    id: model.id,
    name: model.name,
    purpose: model.summary,
    version: model.version,
    language: model.capabilities.includes("speech_to_text") ? "Multilingual" : "Local",
    backend: model.backend,
    runtime: toRuntimeLabel(model.runtime),
    status: model.installed ? "installed" : "available",
    license: model.license,
    capabilities: model.capabilities.map(toCapability).filter(Boolean) as ModelCapability[]
  };
}

function toCapabilitySummary(capability: ApiCapability): CapabilitySummary {
  return {
    id: toCapability(capability.id) ?? "tts",
    label: capability.label,
    description: capability.description
  };
}

function toVoiceSummary(voice: ApiVoice): VoiceSummary {
  return {
    id: voice.id,
    name: voice.name,
    label: voice.name,
    source: voice.source,
    model: voice.model_id ?? "none",
    description: `${voice.source} voice profile.`,
    consent: voice.consent_required ? "required" : "not required"
  };
}

function toCapability(capability: ApiModel["capabilities"][number]): ModelCapability | null {
  switch (capability) {
    case "text_to_speech":
      return "tts";
    case "speech_to_text":
      return "stt";
    case "voice_cloning":
      return "voice_cloning";
    case "live_transcription":
      return "live_transcription";
    case "live_audio":
      return "live_audio";
    default:
      return null;
  }
}

function toRuntimeLabel(runtime: ApiModel["runtime"]): ModelSummary["runtime"] {
  switch (runtime) {
    case "native_rust":
      return "Rust";
    case "onnx":
      return "ONNX";
    case "whisper_cpp":
      return "whisper.cpp";
    default:
      return "Python";
  }
}
