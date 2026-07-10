import { mockRuntime } from "./mockData";
import type { CapabilitySummary, DoctorResponse, ModelCapability, ModelPlan, ModelSummary, RunnerSummary, RuntimeSnapshot, SpeechApiRequest, SpeechApiResponse, TranscriptionApiRequest, TranscriptionApiResponse, VoiceSummary } from "./types";

const viteApiOverride = (import.meta as ImportMeta & { env?: Record<string, string | undefined> }).env?.VITE_TAKOKIT_API_URL;
const LOCAL_API_BASE_URL = viteApiOverride || window.location.origin;

type ApiStatus = {
  server: string;
  storage_root: string;
};

type ApiModel = {
  id: string;
  name: string;
  family: string;
  version: string;
  summary: string;
  license: string;
  license_warning?: string;
  runtime: "python" | "onnx" | "whisper_cpp" | "native_rust" | "external";
  backend: string;
  runner: string;
  hardware_notes: string;
  artifact_count: number;
  capabilities: ApiCapabilityId[];
  installed: boolean;
  runner_installed: boolean;
  runner_runtime_state: ModelPlan["runner_runtime_state"];
  lifecycle_state: ModelPlan["lifecycle_state"];
  executable: boolean;
  missing: string[];
  next_command: string;
  execution_status: string;
};

type ApiCapabilityId = "text_to_speech" | "speech_to_text" | "voice_cloning" | "live_transcription" | "live_audio";

type ApiCapability = {
  id: ApiCapabilityId;
  label: string;
  description: string;
};

type ApiRunner = RunnerSummary;
export type LibraryEntry = Record<string, unknown>;

type PullResponse = {
  id: string;
  installed: boolean;
  manifest_path: string;
  note: string;
};

type ModelInstallResponse = {
  model_id: string;
  required_runner: string;
  required_adapter?: string;
  executable: boolean;
  missing: string[];
};

type ApiVoice = {
  id: string;
  name: string;
  source: string;
  model_id?: string;
  consent_required: boolean;
};

export async function generateSpeech(request: SpeechApiRequest): Promise<SpeechApiResponse> {
  return requestJson<SpeechApiResponse>("/v1/audio/speech", {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify(request)
  });
}

export async function transcribeAudio(request: TranscriptionApiRequest): Promise<TranscriptionApiResponse> {
  return requestJson<TranscriptionApiResponse>("/v1/audio/transcriptions", {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify(request)
  });
}

export async function getDoctor(): Promise<DoctorResponse> {
  const response = await getJson<{ data: DoctorResponse }>("/v1/doctor");
  return response.data;
}

export async function getRunnerDoctor(id: string): Promise<Record<string, unknown>> {
  const response = await getJson<{ data: Record<string, unknown> }>(`/v1/runners/${encodeURIComponent(id)}/doctor`);
  return response.data;
}

export async function getLibraryModels(): Promise<LibraryEntry[]> {
  const response = await getJson<{ data: LibraryEntry[] }>("/v1/library/models");
  return response.data;
}

export async function getLibraryRunners(): Promise<LibraryEntry[]> {
  const response = await getJson<{ data: LibraryEntry[] }>("/v1/library/runners");
  return response.data;
}

export async function getModel(id: string): Promise<ModelSummary> {
  const response = await getJson<{ data: ApiModel }>(`/v1/models/${encodeURIComponent(id)}`);
  return toModelSummary(response.data);
}

export async function getModelPlan(id: string): Promise<ModelPlan> {
  const response = await getJson<{ data: ModelPlan }>(`/v1/models/${encodeURIComponent(id)}/plan`);
  return response.data;
}

export async function pullModel(id: string): Promise<ModelInstallResponse> {
  return requestJson<ModelInstallResponse>("/v1/models/pull", {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify({ model: id })
  });
}

export async function removeModel(id: string): Promise<void> {
  await requestNoContent(`/v1/models/${encodeURIComponent(id)}`, { method: "DELETE" });
}

export async function getRunner(id: string): Promise<RunnerSummary> {
  const response = await getJson<{ data: ApiRunner }>(`/v1/runners/${encodeURIComponent(id)}`);
  return response.data;
}

export async function pullRunner(id: string): Promise<PullResponse> {
  return requestJson<PullResponse>("/v1/runners/pull", {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify({ runner: id })
  });
}

export async function installRunner(id: string): Promise<PullResponse> {
  return requestJson<PullResponse>("/v1/runners/install", {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify({ runner: id })
  });
}

export async function removeRunner(id: string): Promise<void> {
  await requestNoContent(`/v1/runners/${encodeURIComponent(id)}`, { method: "DELETE" });
}

export async function installAdapter(id: string): Promise<void> {
  await requestJson<{ data: unknown }>("/v1/adapters/install", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ adapter: id })
  });
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
      modeNote: "Local runtime mode: each model shows its real execution state."
    };
  } catch {
    return {
      ...mockRuntime,
      models: mockRuntime.models.map((model) => ({
        ...model,
        executable: false,
        lifecycleState: "metadata-only",
        runnerRuntimeState: "runtime-missing",
        missing: ["Local Takokit API is unavailable; live state cannot be verified."],
        executionStatus: "unverified while the local API is offline"
      })),
      runners: mockRuntime.runners.map((runner) => ({
        ...runner,
        installed: false,
        install_state: "runtime-missing"
      })),
      modeNote: "Local API offline: no model is presented as executable."
    };
  }
}

const mockSpeechModel: ModelSummary = {
  id: "mock-tts",
  name: "Mock TTS",
  family: "internal-test",
  purpose: "Deterministic test WAV generator for API and CLI scaffolding.",
  version: "0.1.0",
  language: "Local",
  backend: "native_rust",
  runtime: "Rust",
  status: "installed",
  license: "internal-test",
  lifecycleState: "executable",
  runnerRuntimeState: "ready",
  executable: true,
  missing: [],
  nextCommand: "takokit speak \"hello\" --model mock-tts",
  runner: "takokit-mock",
  runnerInstalled: true,
  hardwareNotes: "CPU, no model weights",
  executionStatus: "ready",
  artifactCount: 0,
  capabilities: ["tts", "live_audio"]
};

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${LOCAL_API_BASE_URL}${path}`);
  if (!response.ok) {
    throw new Error(await errorMessage(response));
  }
  return response.json() as Promise<T>;
}

async function requestJson<T>(path: string, init: RequestInit): Promise<T> {
  const response = await fetch(`${LOCAL_API_BASE_URL}${path}`, init);
  if (!response.ok) {
    throw new Error(await errorMessage(response));
  }
  return response.json() as Promise<T>;
}

async function requestNoContent(path: string, init: RequestInit): Promise<void> {
  const response = await fetch(`${LOCAL_API_BASE_URL}${path}`, init);
  if (!response.ok) {
    throw new Error(await errorMessage(response));
  }
}

async function errorMessage(response: Response): Promise<string> {
  try {
    const body = await response.json() as { error?: { code?: string; message?: string } };
    if (body.error?.message) {
      return body.error.code ? `${body.error.code}: ${body.error.message}` : body.error.message;
    }
  } catch {
    // Fall through to status text.
  }
  return `Takokit API request failed with ${response.status}`;
}

function toModelSummary(model: ApiModel): ModelSummary {
  return {
    id: model.id,
    name: model.name,
    family: model.family,
    purpose: model.summary,
    version: model.version,
    language: model.capabilities.includes("speech_to_text") ? "Multilingual" : "Local",
    backend: model.backend,
    runner: model.runner,
    runnerInstalled: model.runner_installed,
    hardwareNotes: model.hardware_notes,
    executionStatus: model.execution_status,
    artifactCount: model.artifact_count,
    runtime: toRuntimeLabel(model.runtime),
    status: model.installed ? "installed" : "available",
    license: model.license,
    licenseWarning: model.license_warning,
    lifecycleState: model.lifecycle_state,
    runnerRuntimeState: model.runner_runtime_state,
    executable: model.executable,
    missing: model.missing,
    nextCommand: model.next_command,
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
    case "external":
      return "External";
    default:
      return "Python";
  }
}
