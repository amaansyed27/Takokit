import { getWorkspaceContext, workspaceHeaders } from "./workspace";
import type {
  CapabilitySummary,
  DoctorResponse,
  ModelCapability,
  ModelInstallResponse,
  ModelPlan,
  ModelRemovalReport,
  ModelSummary,
  RunnerSummary,
  RuntimeSnapshot,
  SpeechApiRequest,
  SpeechApiResponse,
  TranscriptionApiRequest,
  TranscriptionApiResponse,
  VoiceConversionApiRequest,
  VoiceConversionApiResponse,
  VoiceSummary
} from "./types";

const viteApiOverride = (import.meta as ImportMeta & { env?: Record<string, string | undefined> }).env?.VITE_TAKOKIT_API_URL;
const LOCAL_API_BASE_URL = viteApiOverride || window.location.origin;

type ApiStatus = {
  service: string;
  version: string;
  server: string;
  storage_root: string;
};

type ApiDaemonIdentity = {
  instance_id?: string;
  mode: "managed" | "direct";
  pid: number;
  executable: string;
  storage_root: string;
  host: string;
  port: number;
  started_at: number;
  log_path?: string;
  build_id?: string;
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

type ApiCapabilityId =
  | "text_to_speech"
  | "speech_to_text"
  | "voice_cloning"
  | "voice_conversion"
  | "live_transcription"
  | "live_audio";

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

type ApiVoice = {
  id: string;
  name: string;
  source: string;
  model_id?: string;
  consent_required: boolean;
};

type InstalledModelEntry = {
  id?: string;
  name: string;
  canonical_reference?: string;
};

type InstalledModelsResponse = {
  kind: "installed-models";
  data: InstalledModelEntry[];
};

export class TakokitApiError extends Error {
  constructor(
    public readonly operation: string,
    public readonly code: string,
    message: string,
    public readonly details?: Record<string, unknown>
  ) {
    super(`${code}: ${message}`);
    this.name = "TakokitApiError";
  }
}

export async function generateSpeech(request: SpeechApiRequest): Promise<SpeechApiResponse> {
  if (!request.input.trim()) throw new Error("Speech text cannot be empty.");
  return requestJson<SpeechApiResponse>("generate speech", "/v1/audio/speech", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request)
  });
}

export async function transcribeAudio(request: TranscriptionApiRequest): Promise<TranscriptionApiResponse> {
  if (!request.file_path.trim()) throw new Error("Choose an existing audio path.");
  return requestJson<TranscriptionApiResponse>("transcribe audio", "/v1/audio/transcriptions", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request)
  });
}

export async function convertVoice(request: VoiceConversionApiRequest): Promise<VoiceConversionApiResponse> {
  if (!request.source_path.trim()) throw new Error("Choose an existing source audio path.");
  if (!request.target_voice.trim()) throw new Error("Choose the conversion target required by this model.");
  return requestJson<VoiceConversionApiResponse>("convert voice", "/v1/audio/conversions", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request)
  });
}

export async function getDoctor(): Promise<DoctorResponse> {
  const response = await getJson<{ data: DoctorResponse }>("load diagnostics", "/v1/doctor");
  return response.data;
}

export async function getRunnerDoctor(id: string): Promise<Record<string, unknown>> {
  const response = await getJson<{ data: Record<string, unknown> }>(
    "inspect runner",
    `/v1/runners/${encodeURIComponent(id)}/doctor`
  );
  return response.data;
}

export async function getLibraryModels(): Promise<LibraryEntry[]> {
  const response = await getJson<{ data: LibraryEntry[] }>("load model library", "/v1/library/models");
  return response.data;
}

export async function getLibraryRunners(): Promise<LibraryEntry[]> {
  const response = await getJson<{ data: LibraryEntry[] }>("load runner library", "/v1/library/runners");
  return response.data;
}

export async function getModel(id: string): Promise<ModelSummary> {
  const response = await getJson<{ data: ApiModel }>("inspect model", `/v1/models/${encodeURIComponent(id)}`);
  return toModelSummary(response.data, response.data.installed);
}

export async function getModelPlan(id: string): Promise<ModelPlan> {
  const response = await getJson<{ data: ModelPlan }>("plan model", `/v1/models/${encodeURIComponent(id)}/plan`);
  return response.data;
}

export async function pullModel(id: string): Promise<ModelInstallResponse> {
  return requestJson<ModelInstallResponse>("pull model", "/v1/models/pull", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ model: id })
  });
}

export async function previewModelRemoval(id: string): Promise<ModelRemovalReport> {
  return requestJson<ModelRemovalReport>(
    "preview model removal",
    `/v1/models/${encodeURIComponent(id)}?dry_run=true`,
    { method: "DELETE" }
  );
}

export async function removeModel(id: string): Promise<ModelRemovalReport | void> {
  return requestJson<ModelRemovalReport>("remove model", `/v1/models/${encodeURIComponent(id)}`, {
    method: "DELETE"
  });
}

export async function getRunner(id: string): Promise<RunnerSummary> {
  const response = await getJson<{ data: ApiRunner }>("inspect runner", `/v1/runners/${encodeURIComponent(id)}`);
  return response.data;
}

export async function pullRunner(id: string): Promise<PullResponse> {
  return requestJson<PullResponse>("pull runner", "/v1/runners/pull", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ runner: id })
  });
}

export async function installRunner(id: string): Promise<PullResponse> {
  return requestJson<PullResponse>("install runner", "/v1/runners/install", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ runner: id })
  });
}

export async function removeRunner(id: string): Promise<void> {
  await requestJson<unknown>("remove runner", `/v1/runners/${encodeURIComponent(id)}`, { method: "DELETE" });
}

export async function installAdapter(id: string): Promise<void> {
  await requestJson<{ data: unknown }>("install adapter", "/v1/adapters/install", {
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
  const [status, identity, capabilities, models, installed, runners, voices] = await Promise.all([
    getJson<ApiStatus>("load daemon status", "/v1/status"),
    getJson<ApiDaemonIdentity>("load daemon identity", "/v1/daemon/identity"),
    getJson<{ data: ApiCapability[] }>("load capabilities", "/v1/capabilities"),
    getJson<{ data: ApiModel[] }>("load models", "/v1/models"),
    getJson<InstalledModelsResponse>("load installed inventory", "/v1/models/installed"),
    getJson<{ data: ApiRunner[] }>("load runners", "/v1/runners"),
    getJson<{ data: ApiVoice[] }>("load voices", "/v1/voices")
  ]);

  const installedReferences = new Set<string>();
  installed.data.forEach((entry) => {
    installedReferences.add(entry.name);
    if (entry.id) installedReferences.add(entry.id);
    if (entry.canonical_reference) installedReferences.add(entry.canonical_reference);
  });
  const catalogModels = models.data.map((model) =>
    toModelSummary(model, model.installed || installedReferences.has(model.id) || installedReferences.has(model.name))
  );
  const installedModels = catalogModels.filter((model) => model.status === "installed");
  const context = getWorkspaceContext();

  return {
    storagePath: status.storage_root,
    workspacePath: context.workspace ?? "Not selected",
    buildId: identity.build_id ?? "legacy or unknown",
    server: {
      status: "online",
      url: LOCAL_API_BASE_URL,
      uptime: `${identity.mode} daemon · pid ${identity.pid}`
    },
    models: installedModels,
    catalogModels,
    runners: runners.data,
    voices: voices.data.map(toVoiceSummary),
    capabilities: capabilities.data.map(toCapabilitySummary),
    modeNote: "Installed state is verified from the canonical local inventory; catalog availability is shown separately."
  };
}

async function getJson<T>(operation: string, path: string): Promise<T> {
  const response = await fetch(`${LOCAL_API_BASE_URL}${path}`, {
    headers: workspaceHeaders()
  });
  return expectJson<T>(operation, response);
}

async function requestJson<T>(operation: string, path: string, init: RequestInit): Promise<T> {
  const response = await fetch(`${LOCAL_API_BASE_URL}${path}`, {
    ...init,
    headers: workspaceHeaders(init.headers)
  });
  return expectJson<T>(operation, response);
}

async function expectJson<T>(operation: string, response: Response): Promise<T> {
  if (response.ok) {
    if (response.status === 204) return undefined as T;
    return response.json() as Promise<T>;
  }

  try {
    const body = (await response.json()) as {
      error?: {
        code?: string;
        message?: string;
        path?: string;
        model?: string;
        log_path?: string;
        next_action?: string;
        [key: string]: unknown;
      };
    };
    if (body.error) {
      const { code = `http_${response.status}`, message = response.statusText || "Takokit API request failed", ...details } = body.error;
      throw new TakokitApiError(operation, code, message, details);
    }
  } catch (error) {
    if (error instanceof TakokitApiError) throw error;
  }
  throw new TakokitApiError(operation, `http_${response.status}`, response.statusText || "Takokit API request failed");
}

function toModelSummary(model: ApiModel, installed: boolean): ModelSummary {
  return {
    id: model.id,
    name: model.name,
    family: model.family,
    purpose: model.summary,
    version: model.version,
    language: model.capabilities.includes("speech_to_text") ? "Multilingual" : "Model-defined",
    backend: model.backend,
    runner: model.runner,
    runnerInstalled: model.runner_installed,
    hardwareNotes: model.hardware_notes,
    executionStatus: model.execution_status,
    artifactCount: model.artifact_count,
    runtime: toRuntimeLabel(model.runtime),
    status: installed ? "installed" : "available",
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

function toCapability(capability: ApiCapabilityId): ModelCapability | null {
  switch (capability) {
    case "text_to_speech":
      return "tts";
    case "speech_to_text":
      return "stt";
    case "voice_cloning":
      return "voice_cloning";
    case "voice_conversion":
      return "voice_conversion";
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
