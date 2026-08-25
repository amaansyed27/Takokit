import { workspaceHeaders } from "./workspace";
import type { VoiceConversionApiResponse } from "./types";

const API_BASE = window.location.origin;

export type RvcVoiceState =
  | "created" | "collecting_samples" | "ready_for_preparation" | "preprocessing"
  | "extracting_f0" | "extracting_features" | "ready_to_train" | "training"
  | "building_index" | "validating_artifacts" | "ready" | "failed" | "cancelled";

export type RvcProject = {
  schema_version: number;
  id: string;
  name: string;
  engine: "rvc";
  state: RvcVoiceState;
  imported: boolean;
  created_at: number;
  updated_at: number;
  latest_job_id?: string;
  active_checkpoint_id?: string;
  active_index_id?: string;
  last_error?: string;
};

export type RvcWarning = { code: string; message: string };
export type RvcInspection = {
  duration_ms?: number;
  sample_rate?: number;
  channels?: number;
  codec?: string;
  container?: string;
  peak_dbfs?: number;
  rms_dbfs?: number;
  silence_ratio?: number;
  clipped_ratio?: number;
};
export type RvcSample = {
  id: string;
  voice_id: string;
  display_name: string;
  source_path: string;
  managed_path: string;
  sha256: string;
  bytes: number;
  imported_at: number;
  included: boolean;
  state: "imported" | "inspected" | "prepared" | "invalid";
  inspection?: RvcInspection;
  warnings: RvcWarning[];
};
export type RvcDataset = {
  voice_id: string;
  sample_count: number;
  included_sample_count: number;
  usable_duration_ms: number;
  warning_count: number;
  duplicate_count: number;
  ready_for_preparation: boolean;
  warnings: RvcWarning[];
  inspected_at: number;
};
export type RvcTrainingPresetId = "quick" | "balanced" | "high-quality" | "custom";
export type RvcDevice = "auto" | "cuda" | "cpu";
export type RvcPrecision = "auto" | "fp16" | "fp32";
export type RvcTrainingConfig = {
  preset: RvcTrainingPresetId;
  epochs: number;
  batch_size: number;
  save_every_epochs: number;
  sample_rate_hz: number;
  model_version: string;
  f0_enabled: boolean;
  f0_method: "rmvpe";
  device: RvcDevice;
  precision: RvcPrecision;
  cache_dataset_on_gpu: boolean;
};
export type RvcTrainingPreset = {
  id: RvcTrainingPresetId;
  label: string;
  description: string;
  config?: RvcTrainingConfig;
};
export type RvcPreflight = {
  class: "recommended" | "possible" | "unsupported";
  cpu: string;
  gpu?: string;
  backend: string;
  vram_bytes?: number;
  system_ram_bytes?: number;
  available_disk_bytes: number;
  dataset_duration_ms: number;
  selected_preset: RvcTrainingPresetId;
  resolved_device: RvcDevice;
  resolved_precision: RvcPrecision;
  resource_category: string;
  reasons: string[];
};
export type RvcJob = {
  id: string;
  voice_id: string;
  config: RvcTrainingConfig;
  status: "queued" | "running" | "succeeded" | "failed" | "cancelled" | "stale";
  stage: string;
  created_at: number;
  started_at?: number;
  finished_at?: number;
  checkpoint_ids: string[];
  failure?: string;
};
export type RvcCheckpoint = {
  id: string;
  voice_id: string;
  path: string;
  sha256: string;
  bytes: number;
  epoch?: number;
  sample_rate_hz?: number;
  model_version?: string;
  f0?: boolean;
  created_at: number;
  valid_for_inference: boolean;
};
export type RvcIndex = {
  id: string;
  voice_id: string;
  path: string;
  sha256: string;
  bytes: number;
  checkpoint_id?: string;
  created_at: number;
  valid: boolean;
};
export type ManagedRvcVoice = {
  id: string;
  project_id: string;
  name: string;
  checkpoint_id: string;
  index_id?: string;
  ready_at: number;
};
export type RvcVoiceDetail = {
  project: RvcProject;
  samples: RvcSample[];
  dataset: RvcDataset;
  managed?: ManagedRvcVoice;
  checkpoints: RvcCheckpoint[];
  indexes: RvcIndex[];
  active_job?: RvcJob;
  conversion_target?: string;
};
export type PackageVerification = {
  schema_version: number;
  package_path: string;
  signed: boolean;
  signature_valid?: boolean;
  signer_fingerprint?: string;
  hashes_valid: boolean;
  voice_name?: string;
  errors: string[];
};

type Envelope<T> = { kind: string; data: T };

export async function listRvcVoices(): Promise<RvcProject[]> {
  return (await request<Envelope<RvcProject[]>>("/v1/voices/rvc")).data;
}
export async function getRvcVoice(voice: string): Promise<RvcVoiceDetail> {
  return (await request<Envelope<RvcVoiceDetail>>(pathFor(voice))).data;
}
export async function createRvcVoice(name: string, consentNote?: string): Promise<RvcProject> {
  return (await request<Envelope<RvcProject>>("/v1/voices/rvc", {
    method: "POST",
    body: JSON.stringify({ name, consent_affirmed: true, consent_note: consentNote })
  })).data;
}
export async function importRvcVoice(checkpoint: string, index: string | undefined, name: string): Promise<RvcProject> {
  return (await request<Envelope<RvcProject>>("/v1/voices/rvc/import", {
    method: "POST",
    body: JSON.stringify({ checkpoint, index, name, consent_affirmed: true, consent_note: "Permission acknowledged in Voice Studio." })
  })).data;
}
export async function addRvcSamples(voice: string, paths: string[]): Promise<RvcSample[]> {
  return (await request<Envelope<RvcSample[]>>(`${pathFor(voice)}/samples`, {
    method: "POST", body: JSON.stringify({ paths })
  })).data;
}
export async function setRvcSampleIncluded(voice: string, sample: string, included: boolean): Promise<RvcSample> {
  return (await request<Envelope<RvcSample>>(`${pathFor(voice)}/samples/${encodeURIComponent(sample)}`, {
    method: "PATCH", body: JSON.stringify({ included })
  })).data;
}
export async function removeRvcSample(voice: string, sample: string): Promise<void> {
  await request(`${pathFor(voice)}/samples/${encodeURIComponent(sample)}`, { method: "DELETE" });
}
export async function inspectRvcDataset(voice: string): Promise<RvcDataset> {
  return (await request<Envelope<RvcDataset>>(`${pathFor(voice)}/dataset/inspect`, { method: "POST", body: "{}" })).data;
}
export async function clearRvcPreparedDataset(voice: string): Promise<void> {
  await request(`${pathFor(voice)}/dataset/prepared`, { method: "DELETE" });
}
export async function getRvcPresets(): Promise<RvcTrainingPreset[]> {
  return (await request<Envelope<RvcTrainingPreset[]>>("/v1/voices/rvc/presets")).data;
}
export async function preflightRvc(voice: string, config: RvcTrainingConfig): Promise<RvcPreflight> {
  return (await request<Envelope<RvcPreflight>>(`${pathFor(voice)}/preflight`, { method: "POST", body: JSON.stringify(config) })).data;
}
export async function prepareRvc(voice: string, preset: RvcTrainingPresetId, custom?: RvcTrainingConfig): Promise<RvcJob> {
  return trainingAction(voice, "prepare", preset, custom);
}
export async function trainRvc(voice: string, preset: RvcTrainingPresetId, custom?: RvcTrainingConfig): Promise<RvcJob> {
  return trainingAction(voice, "train", preset, custom);
}
async function trainingAction(voice: string, action: string, preset: RvcTrainingPresetId, custom?: RvcTrainingConfig): Promise<RvcJob> {
  return (await request<Envelope<RvcJob>>(`${pathFor(voice)}/${action}`, {
    method: "POST", body: JSON.stringify({ preset, custom })
  })).data;
}
export async function getRvcJob(voice: string): Promise<RvcJob | null> {
  return (await request<Envelope<RvcJob | null>>(`${pathFor(voice)}/train/status`)).data;
}
export async function getRvcLogs(voice: string): Promise<string> {
  return (await request<Envelope<{text: string}>>(`${pathFor(voice)}/train/logs?max_bytes=262144`)).data.text;
}
export async function cancelRvcJob(voice: string): Promise<RvcJob> {
  return (await request<Envelope<RvcJob>>(`${pathFor(voice)}/train/cancel`, { method: "POST", body: "{}" })).data;
}
export async function recoverRvcJob(voice: string): Promise<RvcJob> {
  return (await request<Envelope<RvcJob>>(`${pathFor(voice)}/train/recover`, { method: "POST", body: "{}" })).data;
}
export async function activateRvcCheckpoint(voice: string, checkpointId: string, indexId?: string): Promise<ManagedRvcVoice> {
  return (await request<Envelope<ManagedRvcVoice>>(`${pathFor(voice)}/checkpoint`, {
    method: "POST", body: JSON.stringify({ checkpoint_id: checkpointId, index_id: indexId })
  })).data;
}
export async function testRvcVoice(voice: string, input: string): Promise<VoiceConversionApiResponse> {
  return (await request<Envelope<VoiceConversionApiResponse>>(`${pathFor(voice)}/test`, {
    method: "POST", body: JSON.stringify({ input, workspace_root: null })
  })).data;
}
export async function exportRvcVoice(voice: string, output: string, sign: boolean, includeReference = false): Promise<string> {
  return (await request<Envelope<{path: string}>>(`${pathFor(voice)}/export`, {
    method: "POST", body: JSON.stringify({ output, sign, include_reference: includeReference })
  })).data.path;
}
export async function verifyRvcPackage(packagePath: string): Promise<PackageVerification> {
  return (await request<Envelope<PackageVerification>>("/v1/voices/rvc/package/verify", {
    method: "POST", body: JSON.stringify({ package: packagePath })
  })).data;
}
export async function importRvcPackage(packagePath: string, name?: string): Promise<RvcProject> {
  return (await request<Envelope<RvcProject>>("/v1/voices/rvc/package/import", {
    method: "POST", body: JSON.stringify({ package: packagePath, name, consent_affirmed: true, consent_note: "Permission acknowledged in Voice Studio." })
  })).data;
}
export async function removeRvcVoice(voice: string, dryRun = false): Promise<Record<string, unknown>> {
  return (await request<Envelope<Record<string, unknown>>>(`${pathFor(voice)}?dry_run=${dryRun}`, { method: "DELETE" })).data;
}

function pathFor(voice: string): string {
  return `/v1/voices/rvc/${encodeURIComponent(voice)}`;
}

async function request<T = unknown>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: workspaceHeaders(init.body ? { "Content-Type": "application/json", ...init.headers } : init.headers)
  });
  if (response.ok) {
    if (response.status === 204) return undefined as T;
    return response.json() as Promise<T>;
  }
  let message = `RVC request failed with HTTP ${response.status}`;
  try {
    const body = await response.json() as { error?: { message?: string; code?: string } };
    if (body.error?.message) message = body.error.code ? `${body.error.code}: ${body.error.message}` : body.error.message;
  } catch { /* keep fallback */ }
  throw new Error(message);
}
